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
        execution_mode: if key == "async" {
            ExecutionMode::Async
        } else {
            ExecutionMode::Sync
        },
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
fn shell_output_with(
    command: &str,
    input: &[u8],
    cwd: Option<&Path>,
    environment: Option<(&str, &str)>,
) -> Output {
    let mut child = Command::new(std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into()));
    child
        .args(["/C", command])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = cwd {
        child.current_dir(cwd);
    }
    if let Some((key, value)) = environment {
        child.env(key, value);
    }
    let mut child = child.spawn().unwrap();
    use std::io::Write;
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}
#[cfg(not(windows))]
fn shell_output_with(
    command: &str,
    input: &[u8],
    cwd: Option<&Path>,
    environment: Option<(&str, &str)>,
) -> Output {
    let mut child = Command::new(std::env::var_os("SHELL").unwrap_or_else(|| "/bin/sh".into()));
    child
        .args(["-lc", command])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = cwd {
        child.current_dir(cwd);
    }
    if let Some((key, value)) = environment {
        child.env(key, value);
    }
    let mut child = child.spawn().unwrap();
    use std::io::Write;
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

fn shell_output(command: &str, input: &[u8]) -> Output {
    shell_output_with(command, input, None, None)
}

fn proxy_output_with(
    manifest: &Path,
    key: &str,
    input: &[u8],
    cwd: Option<&Path>,
    environment: Option<(&str, &str)>,
) -> Output {
    let binary = env!("CARGO_BIN_EXE_hookstat");
    let mut child = Command::new(binary);
    child
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
        .stderr(Stdio::piped());
    if let Some(cwd) = cwd {
        child.current_dir(cwd);
    }
    if let Some((key, value)) = environment {
        child.env(key, value);
    }
    let mut child = child.spawn().unwrap();
    use std::io::Write;
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

fn proxy_output(manifest: &Path, key: &str, input: &[u8]) -> Output {
    proxy_output_with(manifest, key, input, None, None)
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
#[cfg(windows)]
const HIGH_EXIT: &str = "exit /b 259";
#[cfg(windows)]
fn large_stdout_command(path: &Path) -> String {
    format!("type {}", path.to_string_lossy())
}
#[cfg(not(windows))]
fn large_stdout_command(path: &Path) -> String {
    format!("cat \"{}\"", path.to_string_lossy())
}
#[cfg(windows)]
fn large_stderr_command(path: &Path) -> String {
    format!("type {} 1>&2", path.to_string_lossy())
}
#[cfg(not(windows))]
fn large_stderr_command(path: &Path) -> String {
    format!("cat \"{}\" >&2", path.to_string_lossy())
}
#[cfg(windows)]
const ENV_AND_CWD: &str = "echo %HOOKSTAT_PROXY_ENV% & cd";
#[cfg(not(windows))]
const ENV_AND_CWD: &str = "printf '%s\\n' \"$HOOKSTAT_PROXY_ENV\"; pwd";

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
    let commands = ["a", "b", "c", "async"].map(|key| (key, SUCCESS));
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
    assert!(
        scan.invocations
            .iter()
            .any(|value| value.handler.execution_mode == ExecutionMode::Async)
    );
}

#[cfg(windows)]
#[test]
fn proxy_preserves_full_windows_exit_code_without_u8_truncation() {
    let temp = tempdir().unwrap();
    let manifest_path = temp.path().join("state/manifests/hooks.json");
    manifest(&manifest_path, &[("high-exit", HIGH_EXIT)]);
    let expected = shell_output(HIGH_EXIT, b"");
    let actual = proxy_output(&manifest_path, "high-exit", b"");
    assert_eq!(expected.status.code(), Some(259));
    assert_eq!(actual.status.code(), expected.status.code());
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

#[test]
fn proxy_preserves_large_streams_working_directory_and_environment() {
    let temp = tempdir().unwrap();
    let manifest_path = temp.path().join("state/manifests/hooks.json");
    let large_file = temp.path().join("large-stream.bin");
    fs::write(&large_file, vec![b'x'; 65_536]).unwrap();
    let large_stdout = large_stdout_command(&large_file);
    let large_stderr = large_stderr_command(&large_file);
    manifest(
        &manifest_path,
        &[
            ("large-stdout", &large_stdout),
            ("large-stderr", &large_stderr),
            ("context", ENV_AND_CWD),
            ("streaming-stdin", STDIN_COMMAND),
        ],
    );
    let expected_large_stdout = shell_output(&large_stdout, b"");
    let actual_large_stdout = proxy_output(&manifest_path, "large-stdout", b"");
    assert_eq!(
        actual_large_stdout.status.code(),
        expected_large_stdout.status.code()
    );
    assert_eq!(actual_large_stdout.stdout, expected_large_stdout.stdout);
    assert_eq!(actual_large_stdout.stderr, expected_large_stdout.stderr);
    assert_eq!(actual_large_stdout.stdout.len(), 65_536);
    let expected_large_stderr = shell_output(&large_stderr, b"");
    let actual_large_stderr = proxy_output(&manifest_path, "large-stderr", b"");
    assert_eq!(
        actual_large_stderr.status.code(),
        expected_large_stderr.status.code()
    );
    assert_eq!(actual_large_stderr.stdout, expected_large_stderr.stdout);
    assert_eq!(actual_large_stderr.stderr, expected_large_stderr.stderr);
    assert_eq!(actual_large_stderr.stderr.len(), 65_536);
    let large_input = vec![b'z'; 131_072];
    let expected_input = shell_output(STDIN_COMMAND, &large_input);
    let actual_input = proxy_output(&manifest_path, "streaming-stdin", &large_input);
    assert_eq!(actual_input.status.code(), expected_input.status.code());
    assert_eq!(actual_input.stdout, expected_input.stdout);
    assert_eq!(actual_input.stderr, expected_input.stderr);
    let expected_context = shell_output_with(
        ENV_AND_CWD,
        b"",
        Some(temp.path()),
        Some(("HOOKSTAT_PROXY_ENV", "opaque-fixture")),
    );
    let actual_context = proxy_output_with(
        &manifest_path,
        "context",
        b"",
        Some(temp.path()),
        Some(("HOOKSTAT_PROXY_ENV", "opaque-fixture")),
    );
    assert_eq!(actual_context.status.code(), expected_context.status.code());
    assert_eq!(actual_context.stdout, expected_context.stdout);
    assert_eq!(actual_context.stderr, expected_context.stderr);
}

#[cfg(not(windows))]
#[test]
fn proxy_preserves_non_utf8_output_without_decoding_it() {
    let temp = tempdir().unwrap();
    let manifest_path = temp.path().join("state/manifests/hooks.json");
    let binary = "printf '\\377\\200\\000'";
    manifest(&manifest_path, &[("binary", binary)]);
    let expected = shell_output(binary, b"");
    let actual = proxy_output(&manifest_path, "binary", b"");
    assert_eq!(actual.status.code(), expected.status.code());
    assert_eq!(actual.stdout, vec![255, 128, 0]);
    assert_eq!(actual.stdout, expected.stdout);
}

#[cfg(windows)]
#[test]
fn cancellation_kills_the_proxy_process_tree_and_leaves_incomplete_coverage() {
    let temp = tempdir().unwrap();
    let manifest_path = temp.path().join("state/manifests/hooks.json");
    manifest(&manifest_path, &[("slow", "ping -n 20 127.0.0.1 > NUL")]);
    let binary = env!("CARGO_BIN_EXE_hookstat");
    let mut child = Command::new(binary)
        .args([
            "codex",
            "proxy",
            "--manifest",
            manifest_path.to_str().unwrap(),
            "--handler",
            "slow",
        ])
        .spawn()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(250));
    let status = Command::new("taskkill")
        .args(["/PID", &child.id().to_string(), "/T", "/F"])
        .status()
        .unwrap();
    assert!(status.success());
    let _ = child.wait();
    let spool = ReceiptSpool::open(temp.path().join("state/receipts")).unwrap();
    let scan = spool.scan();
    assert_eq!(scan.starts_without_completion, 1);
    assert_eq!(
        scan.invocations[0].terminal_status,
        TerminalStatus::Incomplete
    );
}
