use hookstat::codex::{ProxyHandler, ProxyManifest};
use hookstat::domain::{
    EvidenceCoverage, ExecutionMode, HandlerIdentity, HookEvent, TerminalStatus,
};
use hookstat::receipt::ReceiptSpool;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use tempfile::tempdir;

fn handler(key: &str) -> HandlerIdentity {
    HandlerIdentity {
        key: key.into(),
        revision: format!("revision-{key}"),
        label: format!("fixture-{key}"),
        source_kind: "fixture_hooks_json".into(),
        event: HookEvent::Stop,
        matcher_identity: "any".into(),
        structural_identity: format!("g0:{key}"),
        execution_mode: ExecutionMode::Sync,
    }
}

fn manifest(path: &Path, handlers: &[(&str, &str)]) {
    let mut values = BTreeMap::new();
    for (key, command) in handlers {
        values.insert(
            (*key).into(),
            ProxyHandler {
                handler: handler(key),
                command: (*command).into(),
                command_windows: Some((*command).into()),
            },
        );
    }
    let manifest = ProxyManifest {
        schema_version: 1,
        config_path_fingerprint: "fixture".into(),
        original_config_sha256: "fixture".into(),
        handlers: values,
    };
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec(&manifest).unwrap()).unwrap();
}

#[cfg(windows)]
fn shell_output(command: &str, input: &[u8]) -> Output {
    let mut child = Command::new(std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into()))
        .args(["/C", command])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}
#[cfg(not(windows))]
fn shell_output(command: &str, input: &[u8]) -> Output {
    let mut child = Command::new(std::env::var_os("SHELL").unwrap_or_else(|| "/bin/sh".into()))
        .args(["-lc", command])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

fn proxy_output(manifest: &Path, key: &str, input: &[u8]) -> Output {
    let binary = env!("CARGO_BIN_EXE_hookstat");
    let mut child = Command::new(binary)
        .args([
            "codex",
            "proxy",
            "--manifest",
            manifest.to_str().unwrap(),
            "--handler",
            key,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

#[cfg(windows)]
const OUTPUT_FAILURE: &str = "echo proxy-stdout & echo proxy-stderr 1>&2 & exit /b 7";
#[cfg(not(windows))]
const OUTPUT_FAILURE: &str = "printf proxy-stdout; printf proxy-stderr >&2; exit 7";
#[cfg(windows)]
const STDIN_COMMAND: &str = "more";
#[cfg(not(windows))]
const STDIN_COMMAND: &str = "cat";
#[cfg(windows)]
const SUCCESS: &str = "exit /b 0";
#[cfg(not(windows))]
const SUCCESS: &str = "exit 0";
#[cfg(windows)]
const CONTROL: &str = "echo control 1>&2 & exit /b 2";
#[cfg(not(windows))]
const CONTROL: &str = "printf control >&2; exit 2";

#[test]
fn proxy_preserves_fixture_stdout_stderr_stdin_and_exit_code() {
    let temp = tempdir().unwrap();
    let manifest_path = temp.path().join("state/manifests/hooks.json");
    manifest(
        &manifest_path,
        &[("failure", OUTPUT_FAILURE), ("stdin", STDIN_COMMAND)],
    );
    for (key, command, input) in [
        ("failure", OUTPUT_FAILURE, b"fixture-input\n".as_slice()),
        ("stdin", STDIN_COMMAND, b"fixture-input\n".as_slice()),
    ] {
        let expected = shell_output(command, input);
        let actual = proxy_output(&manifest_path, key, input);
        assert_eq!(actual.status.code(), expected.status.code());
        assert_eq!(actual.stdout, expected.stdout);
        assert_eq!(actual.stderr, expected.stderr);
    }
    let spool = ReceiptSpool::open(temp.path().join("state/receipts")).unwrap();
    let scan = spool.scan();
    assert_eq!(scan.invocations.len(), 2);
    assert!(
        scan.invocations
            .iter()
            .any(|value| value.terminal_status == TerminalStatus::Failed)
    );
}

#[test]
fn proxy_records_control_as_unknown_and_telemetry_failure_is_fail_open() {
    let temp = tempdir().unwrap();
    let manifest_path = temp.path().join("state/manifests/hooks.json");
    manifest(&manifest_path, &[("control", CONTROL)]);
    assert_eq!(
        proxy_output(&manifest_path, "control", b"").status.code(),
        Some(2)
    );
    let spool = ReceiptSpool::open(temp.path().join("state/receipts")).unwrap();
    assert_eq!(
        spool.scan().invocations[0].terminal_status,
        TerminalStatus::Unknown
    );
    let blocked = temp.path().join("blocked/manifests/hooks.json");
    manifest(&blocked, &[("success", SUCCESS)]);
    fs::create_dir_all(blocked.parent().unwrap().parent().unwrap()).unwrap();
    fs::write(
        blocked.parent().unwrap().parent().unwrap().join("receipts"),
        b"not a directory",
    )
    .unwrap();
    assert_eq!(
        proxy_output(&blocked, "success", b"").status.code(),
        Some(0)
    );
}

#[test]
fn concurrent_proxy_processes_create_distinct_complete_receipts() {
    let temp = tempdir().unwrap();
    let manifest_path = temp.path().join("state/manifests/hooks.json");
    let commands = ["a", "b", "c", "d"].map(|key| (key, SUCCESS));
    manifest(&manifest_path, &commands);
    let binary = env!("CARGO_BIN_EXE_hookstat");
    let mut children = Vec::new();
    for (key, _) in commands {
        children.push(
            Command::new(binary)
                .args([
                    "codex",
                    "proxy",
                    "--manifest",
                    manifest_path.to_str().unwrap(),
                    "--handler",
                    key,
                ])
                .spawn()
                .unwrap(),
        );
    }
    for mut child in children {
        assert!(child.wait().unwrap().success());
    }
    let spool = ReceiptSpool::open(temp.path().join("state/receipts")).unwrap();
    let scan = spool.scan();
    assert_eq!(scan.invocations.len(), 4);
    assert_eq!(scan.starts_without_completion, 0);
    assert!(
        scan.invocations
            .iter()
            .all(|value| value.terminal_status == TerminalStatus::Completed)
    );
}

#[test]
fn started_without_completion_remains_explicitly_incomplete() {
    let temp = tempdir().unwrap();
    let spool = ReceiptSpool::open(temp.path().join("receipts")).unwrap();
    spool
        .write_start(&hookstat::receipt::ReceiptStart {
            schema_version: 1,
            invocation_id: "start-only".into(),
            handler: handler("start-only"),
            source: "fixture".into(),
            started_at_unix_ms: 1_000,
            coverage: EvidenceCoverage::Partial,
        })
        .unwrap();
    let scan = spool.scan();
    assert_eq!(scan.starts_without_completion, 1);
    assert_eq!(
        scan.invocations[0].terminal_status,
        TerminalStatus::Incomplete
    );
}
