#![cfg(windows)]

use hookstat_hook::{
    CapsuleStore, ExecutionPlan, HandlerCapsule, InstrumentationEnvelope, OriginalHandlerBudget,
    write_key_for_test,
};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

static PROCESS_TREE_TEST_LOCK: Mutex<()> = Mutex::new(());

fn capsule(plan: ExecutionPlan, budget: Duration) -> HandlerCapsule {
    HandlerCapsule {
        handler_key: "hk_shim_fixture".into(),
        revision: "fixture_revision".into(),
        definition_fingerprint: "sha256:fixture".into(),
        runtime: "controlled_runtime".into(),
        runtime_instance: "controlled_instance".into(),
        event: "controlled_event".into(),
        source_scope: "controlled_scope".into(),
        original_budget: OriginalHandlerBudget(budget),
        instrumentation_envelope: InstrumentationEnvelope(Duration::from_millis(20)),
        execution: plan,
    }
}

fn seal(root: &Path, capsule: HandlerCapsule) -> std::path::PathBuf {
    let key = [3_u8; 32];
    let store = CapsuleStore::open(root).unwrap();
    write_key_for_test(root, &key).unwrap();
    store
        .write_for_test(Path::new("fixture.hshc"), &capsule, &key)
        .unwrap();
    root.join("fixture.hshc")
}

fn shim_command(root: &Path, capsule: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_hookstat-hook"));
    command
        .env("HOOKSTAT_IPC_NO_BROKER_START", "1")
        .args(["--capsule"])
        .arg(capsule)
        .args(["--capsule-root"])
        .arg(root)
        .args(["--state-root"])
        .arg(root.join("ipc-state"));
    command
}

fn invoke(root: &Path, capsule: &Path) -> std::process::Output {
    shim_command(root, capsule).output().unwrap()
}

fn descendant_plan(delay_seconds: u8) -> ExecutionPlan {
    ExecutionPlan::Direct {
        executable: "powershell.exe".into(),
        arguments: vec![
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-Command".into(),
            format!(
                concat!(
                    "Start-Process -FilePath powershell.exe -ArgumentList @(",
                    "'-NoProfile','-NonInteractive','-Command',",
                    "'Set-Content -LiteralPath $env:HS_G36_STARTED -Value started; ",
                    "Start-Sleep -Seconds {delay_seconds}; Set-Content -LiteralPath $env:HS_G36_LEAK -Value leaked'); ",
                    "Start-Sleep -Seconds 10"
                ),
                delay_seconds = delay_seconds,
            ),
        ],
    }
}

fn wait_for(path: &Path, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if path.exists() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    path.exists()
}

#[test]
fn direct_exit_and_shell_fallback_preserve_original_exit_codes_when_broker_is_absent() {
    let temp = tempfile::tempdir().unwrap();
    let direct = seal(
        temp.path(),
        capsule(
            ExecutionPlan::Direct {
                executable: "cmd.exe".into(),
                arguments: vec!["/C".into(), "exit /b 7".into()],
            },
            Duration::from_secs(1),
        ),
    );
    assert_eq!(invoke(temp.path(), &direct).status.code(), Some(7));

    let fallback_root = tempfile::tempdir().unwrap();
    let fallback = seal(
        fallback_root.path(),
        capsule(
            ExecutionPlan::Shell {
                command: "exit /b 0".into(),
            },
            Duration::from_secs(1),
        ),
    );
    assert_eq!(
        invoke(fallback_root.path(), &fallback).status.code(),
        Some(0)
    );
}

#[test]
fn original_timeout_is_exact_and_outer_envelope_does_not_extend_business_runtime() {
    let temp = tempfile::tempdir().unwrap();
    let fixture = seal(
        temp.path(),
        capsule(
            ExecutionPlan::Direct {
                executable: "cmd.exe".into(),
                arguments: vec!["/C".into(), "ping -n 2 127.0.0.1 >nul".into()],
            },
            Duration::from_millis(25),
        ),
    );
    let started = Instant::now();
    let output = invoke(temp.path(), &fixture);
    assert_eq!(output.status.code(), Some(124));
    assert!(started.elapsed() < Duration::from_millis(500));
}

#[test]
fn timeout_keeps_descendants_contained_after_shim_exit() {
    let _guard = PROCESS_TREE_TEST_LOCK.lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let started = temp.path().join("descendant-started.txt");
    let leaked = temp.path().join("descendant-leaked.txt");
    let fixture = seal(
        temp.path(),
        capsule(descendant_plan(8), Duration::from_secs(6)),
    );
    let output = shim_command(temp.path(), &fixture)
        .env("HS_G36_STARTED", &started)
        .env("HS_G36_LEAK", &leaked)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(124));
    assert!(wait_for(&started, Duration::from_secs(1)));
    std::thread::sleep(Duration::from_millis(8_500));
    assert!(!leaked.exists());
}

#[test]
fn externally_killed_shim_keeps_descendants_contained() {
    let _guard = PROCESS_TREE_TEST_LOCK.lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let started = temp.path().join("forced-started.txt");
    let leaked = temp.path().join("forced-leaked.txt");
    let fixture = seal(
        temp.path(),
        capsule(descendant_plan(8), Duration::from_secs(10)),
    );
    let mut shim = shim_command(temp.path(), &fixture)
        .env("HS_G36_STARTED", &started)
        .env("HS_G36_LEAK", &leaked)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let descendant_started = wait_for(&started, Duration::from_secs(10));
    // This is the exact disposable shim process spawned above, not an Owner
    // process. Its Job Object must close and terminate its own descendants.
    shim.kill().unwrap();
    shim.wait().unwrap();
    assert!(descendant_started);
    std::thread::sleep(Duration::from_millis(1_250));
    assert!(!leaked.exists());
}
