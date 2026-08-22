#![cfg(all(windows, feature = "performance-harness"))]

use hookstat::codex::{ProxyHandler, ProxyManifest};
use hookstat::domain::{EvidenceCoverage, ExecutionMode, HandlerIdentity, HookEvent};
use hookstat::receipt::ReceiptSpool;
use std::collections::BTreeMap;
use std::fs;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

fn fixture_handler() -> HandlerIdentity {
    HandlerIdentity {
        key: "hk_g28_fixture".into(),
        revision: "hr_g28_fixture".into(),
        label: "g28_fixture".into(),
        source_kind: "synthetic_fixture".into(),
        event: HookEvent::PreToolUse,
        matcher_identity: "g28_any".into(),
        structural_identity: "g28:0:0".into(),
        execution_mode: ExecutionMode::Sync,
    }
}

/// This is the minimized G28 feedback loop: it uses only a temporary manifest
/// and a no-output fixture executable, then asserts the currently-shipping
/// proxy preserves both a successful exit and complete metadata-only evidence.
#[test]
fn current_proxy_runs_a_disposable_native_fixture_with_complete_evidence() {
    let temp = tempdir().unwrap();
    let manifest = temp.path().join("manifests").join("g28-fixture.json");
    fs::create_dir_all(manifest.parent().unwrap()).unwrap();
    let fixture = env!("CARGO_BIN_EXE_hookstat-g28-handler-fixture");
    let command = fixture.to_owned();
    assert!(
        Command::new(fixture).status().unwrap().success(),
        "disposable fixture must directly exit successfully"
    );
    let shell = Command::new(std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into()))
        .args(["/D", "/C", command.as_str()])
        .output()
        .unwrap();
    assert!(
        shell.status.success(),
        "cmd fixture returned {:?}: {}",
        shell.status.code(),
        String::from_utf8_lossy(&shell.stderr).trim()
    );
    let mut handlers = BTreeMap::new();
    handlers.insert(
        "hk_g28_fixture".into(),
        ProxyHandler {
            handler: fixture_handler(),
            command: command.clone(),
            command_windows: Some(command),
        },
    );
    let value = ProxyManifest {
        schema_version: 1,
        config_path_fingerprint: "g28_fixture_config".into(),
        original_config_sha256: "g28_fixture_original".into(),
        handlers,
    };
    fs::write(&manifest, serde_json::to_vec(&value).unwrap()).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_hookstat"))
        .args([
            "codex",
            "proxy",
            "--manifest",
            manifest.to_str().unwrap(),
            "--handler",
            "hk_g28_fixture",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .unwrap();
    let scan = ReceiptSpool::open_existing(temp.path().join("receipts"))
        .unwrap()
        .scan();
    assert!(
        output.status.success(),
        "disposable proxy returned {:?}, terminal {:?}: {}",
        output.status.code(),
        scan.invocations.first().map(|value| value.terminal_status),
        String::from_utf8_lossy(&output.stderr).trim()
    );

    assert_eq!(scan.invocations.len(), 1);
    assert_eq!(scan.starts_without_completion, 0);
    assert_eq!(scan.invocations[0].coverage, EvidenceCoverage::Partial);
}

#[test]
fn dedicated_job_probe_records_only_bounded_numeric_samples() {
    let output = Command::new(env!("CARGO_BIN_EXE_hookstat-hook-fixture"))
        .args(["--job-probe", "100"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "dedicated Job Object probe failed with {:?}",
        output.status.code()
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    let samples = value["job_object_cycle_ms"].as_array().unwrap();
    assert_eq!(samples.len(), 100);
    assert!(
        samples
            .iter()
            .all(|value| value.as_f64().is_some_and(|value| value >= 0.0))
    );
}

#[test]
fn one_second_disposable_proxy_timeout_is_incomplete_not_success() {
    let temp = tempdir().unwrap();
    let manifest = temp.path().join("manifests").join("g28-timeout.json");
    fs::create_dir_all(manifest.parent().unwrap()).unwrap();
    let fixture = env!("CARGO_BIN_EXE_hookstat-g28-handler-fixture");
    let command = format!("{fixture} --sleep-ms 2000");
    let mut handlers = BTreeMap::new();
    handlers.insert(
        "hk_g28_fixture".into(),
        ProxyHandler {
            handler: fixture_handler(),
            command: command.clone(),
            command_windows: Some(command),
        },
    );
    let value = ProxyManifest {
        schema_version: 1,
        config_path_fingerprint: "g28_timeout_config".into(),
        original_config_sha256: "g28_timeout_original".into(),
        handlers,
    };
    fs::write(&manifest, serde_json::to_vec(&value).unwrap()).unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_hookstat"))
        .args([
            "codex",
            "proxy",
            "--manifest",
            manifest.to_str().unwrap(),
            "--handler",
            "hk_g28_fixture",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    thread::sleep(Duration::from_secs(1));
    assert!(
        child.try_wait().unwrap().is_none(),
        "fixture ended before deadline"
    );
    child.kill().unwrap();
    let _ = child.wait().unwrap();

    let scan = ReceiptSpool::open_existing(temp.path().join("receipts"))
        .unwrap()
        .scan();
    assert_eq!(scan.invocations.len(), 1);
    assert_eq!(scan.starts_without_completion, 1);
    assert_eq!(
        scan.invocations[0].terminal_status,
        hookstat::domain::TerminalStatus::Incomplete
    );
}
