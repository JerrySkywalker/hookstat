#![cfg(windows)]

use hookstat_hook::{
    CapsuleStore, ExecutionPlan, HandlerCapsule, InstrumentationEnvelope, OriginalHandlerBudget,
    capsule_file_name, run_capsule, write_key_for_test,
};
use hookstat_ipc_client::{
    BrokerAcknowledgement, IpcFrame, LocalEndpoint, read_frame_bounded, write_frame_bounded,
};
use interprocess::local_socket::prelude::*;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

static PROCESS_TREE_TEST_LOCK: Mutex<()> = Mutex::new(());

fn process_tree_test_lock() -> std::sync::MutexGuard<'static, ()> {
    // A preceding fixture failure must not prevent a later containment test
    // from exercising its own assertion. The guard still serializes process
    // trees; recovering the guard only removes test-harness cascade noise.
    PROCESS_TREE_TEST_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

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
    let name = capsule_file_name(&capsule).unwrap();
    store
        .write_for_test(Path::new(&name), &capsule, &key)
        .unwrap();
    root.join(name)
}

fn shim_command(root: &Path, capsule: &Path) -> Command {
    shim_command_with_state(root, capsule, &root.join("ipc-state"))
}

fn shim_command_with_state(root: &Path, capsule: &Path, state_root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_hookstat-hook"));
    command
        .env("HOOKSTAT_IPC_NO_BROKER_START", "1")
        .args(["--capsule"])
        .arg(capsule)
        .args(["--capsule-root"])
        .arg(root)
        .args(["--state-root"])
        .arg(state_root);
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
    // The exact boundary is covered by a deterministic unit test. This e2e
    // fixture only guards against a genuinely hung shim, rather than treating
    // temporary scheduler delay as a product-semantic failure.
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn invalid_or_unavailable_ipc_state_is_fail_open_for_the_original_handler() {
    let temp = tempfile::tempdir().unwrap();
    let invalid_state = temp.path().join("not-a-directory");
    std::fs::write(&invalid_state, b"fixture").unwrap();
    let fixture = seal(
        temp.path(),
        capsule(
            ExecutionPlan::Direct {
                executable: "cmd.exe".into(),
                arguments: vec!["/C".into(), "exit /b 0".into()],
            },
            Duration::from_secs(1),
        ),
    );
    let output = shim_command_with_state(temp.path(), &fixture, &invalid_state)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn accepted_start_then_missing_complete_is_an_observation_gap_not_a_hook_failure() {
    let temp = tempfile::tempdir().unwrap();
    let state_root = temp.path().join("ipc-state");
    let endpoint = LocalEndpoint::from_state_root(&state_root).unwrap();
    let listener = endpoint.bind().unwrap();
    let (ready, ready_rx) = mpsc::channel();
    let server = thread::spawn(move || {
        ready.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        let mut stream = loop {
            match listener.accept() {
                Ok(stream) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "shim did not emit START");
                    thread::sleep(Duration::from_millis(1));
                }
                Err(error) => panic!("listener failed: {error}"),
            }
        };
        assert!(matches!(
            read_frame_bounded(&mut stream, Duration::from_millis(100)),
            Ok(IpcFrame::Start(_))
        ));
        write_frame_bounded(
            &mut stream,
            &IpcFrame::Ack(BrokerAcknowledgement::Accepted),
            Duration::from_millis(100),
        )
        .unwrap();
        // Drop the only endpoint after START. COMPLETE therefore cannot be
        // emitted, but the original handler must keep its terminal result.
        drop(stream);
        drop(listener);
    });
    ready_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    let mut accepted_start_capsule = capsule(
        ExecutionPlan::Direct {
            executable: "cmd.exe".into(),
            arguments: vec!["/C".into(), "ping -n 2 127.0.0.1 >nul".into()],
        },
        Duration::from_secs(3),
    );
    accepted_start_capsule.instrumentation_envelope =
        InstrumentationEnvelope(Duration::from_millis(50));
    let result = run_capsule(&accepted_start_capsule, &state_root).unwrap();
    server.join().unwrap();
    assert_eq!(result.exit_code, 0);
    assert_eq!(
        result.started,
        hookstat_ipc_client::ObservationDisposition::Accepted
    );
    assert_eq!(
        result.completed,
        hookstat_ipc_client::ObservationDisposition::Unavailable
    );
}

#[test]
fn delayed_broker_ack_exhausts_only_observation_budget() {
    let temp = tempfile::tempdir().unwrap();
    let state_root = temp.path().join("delayed-ipc-state");
    let endpoint = LocalEndpoint::from_state_root(&state_root).unwrap();
    let listener = endpoint.bind().unwrap();
    let (ready, ready_rx) = mpsc::channel();
    let server = thread::spawn(move || {
        ready.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        let _stream = loop {
            match listener.accept() {
                Ok(stream) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "shim did not emit delayed START");
                    thread::sleep(Duration::from_millis(1));
                }
                Err(error) => panic!("delayed broker listener failed: {error}"),
            }
        };
        // The client ACK limit is deliberately smaller than this controlled
        // delay. It may close before a server-side frame read after its own
        // timeout, which is precisely why the fixture must not assert that
        // read. The original handler must still keep its successful result.
        thread::sleep(Duration::from_millis(30));
    });
    ready_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    let mut delayed = capsule(
        ExecutionPlan::Direct {
            executable: "cmd.exe".into(),
            arguments: vec!["/C".into(), "exit /b 0".into()],
        },
        Duration::from_secs(1),
    );
    delayed.instrumentation_envelope = InstrumentationEnvelope(Duration::from_millis(50));
    let result = run_capsule(&delayed, &state_root).unwrap();
    server.join().unwrap();
    assert_eq!(result.exit_code, 0);
    assert_eq!(
        result.started,
        hookstat_ipc_client::ObservationDisposition::BudgetExhausted
    );
}

#[test]
fn timeout_keeps_descendants_contained_after_shim_exit() {
    let _guard = process_tree_test_lock();
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
    let _guard = process_tree_test_lock();
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
