#![cfg(windows)]

use hookstat_hook::{
    CapsuleStore, ExecutionPlan, HandlerCapsule, InstrumentationEnvelope, OriginalHandlerBudget,
    write_key_for_test,
};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

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

fn invoke(root: &Path, capsule: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_hookstat-hook"))
        .env("HOOKSTAT_IPC_NO_BROKER_START", "1")
        .args(["--capsule"])
        .arg(capsule)
        .args(["--capsule-root"])
        .arg(root)
        .args(["--state-root"])
        .arg(root.join("ipc-state"))
        .output()
        .unwrap()
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
