#![cfg(windows)]

use hookstat::codex::{apply, discover_paths};
use hookstat::domain::TerminalStatus;
use hookstat::receipt::ReceiptSpool;
use serde_json::Value;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use tempfile::tempdir;

/// Mirrors Codex 0.147's `command_runner.rs`: it appends a literal
/// `"<command_line>"` with `raw_arg` after `cmd.exe /C`.
fn prepend_path(command: &mut Command, directory: &Path) {
    let mut entries = vec![directory.to_path_buf()];
    if let Some(path) = std::env::var_os("PATH") {
        entries.extend(std::env::split_paths(&path));
    }
    command.env("PATH", std::env::join_paths(entries).unwrap());
}

fn binary_output_with_path(binary: &Path, arguments: &[&str], directories: &[&Path]) -> Output {
    let mut entries = directories
        .iter()
        .map(|directory| (*directory).to_path_buf())
        .collect::<Vec<_>>();
    if let Some(path) = std::env::var_os("PATH") {
        entries.extend(std::env::split_paths(&path));
    }
    Command::new(binary)
        .args(arguments)
        .env("PATH", std::env::join_paths(entries).unwrap())
        .output()
        .unwrap()
}

fn codex_0147_outer_quoted_output_with_path(
    command_line: &str,
    path_directory: Option<&Path>,
) -> Output {
    let shell = std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into());
    let mut command = Command::new(shell);
    command
        .arg("/C")
        .raw_arg(format!(r#""{command_line}""#))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(directory) = path_directory {
        prepend_path(&mut command, directory);
    }
    command.output().unwrap()
}

fn codex_0147_outer_quoted_output(command_line: &str) -> Output {
    codex_0147_outer_quoted_output_with_path(command_line, None)
}

/// Mirrors the non-`/c` hook shell path used by Codex 0.147 when the selected
/// `TurnEnvironment` is PowerShell.
fn codex_0147_powershell_output_with_path(
    command_line: &str,
    path_directory: Option<&Path>,
) -> Output {
    let mut command = Command::new("pwsh.exe");
    command
        .args(["-NoProfile", "-NonInteractive", "-Command", command_line])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(directory) = path_directory {
        prepend_path(&mut command, directory);
    }
    command.output().unwrap()
}

fn codex_0147_powershell_output(command_line: &str) -> Output {
    codex_0147_powershell_output_with_path(command_line, None)
}

fn codex_0147_outer_quoted_output_with_context(
    command_line: &str,
    path_directory: &Path,
    input: &[u8],
    cwd: &Path,
) -> Output {
    use std::io::Write;

    let shell = std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into());
    let mut command = Command::new(shell);
    command
        .arg("/C")
        .raw_arg(format!(r#""{command_line}""#))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(cwd)
        .env("HOOKSTAT_SHELL_E2E", "preserved");
    prepend_path(&mut command, path_directory);
    let mut child = command.spawn().unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

fn codex_0147_powershell_output_with_context(
    command_line: &str,
    path_directory: &Path,
    input: &[u8],
    cwd: &Path,
) -> Output {
    use std::io::Write;

    let mut command = Command::new("pwsh.exe");
    command
        .args(["-NoProfile", "-NonInteractive", "-Command", command_line])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(cwd)
        .env("HOOKSTAT_SHELL_E2E", "preserved");
    prepend_path(&mut command, path_directory);
    let mut child = command.spawn().unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

fn first_commands(config: &Path) -> (String, String) {
    let value: Value = serde_json::from_slice(&fs::read(config).unwrap()).unwrap();
    let handler = &value["hooks"]["Stop"][0]["hooks"][0];
    (
        handler["command"].as_str().unwrap().to_owned(),
        handler["commandWindows"].as_str().unwrap().to_owned(),
    )
}

#[test]
fn codex_0147_outer_quotes_reach_tokenized_proxy_without_embedded_quotes() {
    let temp = tempdir().unwrap();
    let config_root = temp.path().join("Config Root \u{6d4b}\u{8bd5}");
    let config = config_root.join("hooks.json");
    let data_root = temp.path().join("HookStat Data \u{6d4b}\u{8bd5}");
    let executable = temp
        .path()
        .join("HookStat Executable \u{6d4b}\u{8bd5}")
        .join("hookstat.exe");
    fs::create_dir_all(&config_root).unwrap();
    fs::create_dir_all(executable.parent().unwrap()).unwrap();
    fs::copy(env!("CARGO_BIN_EXE_hookstat"), &executable).unwrap();
    fs::write(
        &config,
        br#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"exit /b 7"}]}]}}"#,
    )
    .unwrap();

    let discovery = discover_paths(std::slice::from_ref(&config)).unwrap();
    assert_eq!(
        apply(&discovery, &data_root, &executable).unwrap().applied,
        1
    );
    let (legacy, windows) = first_commands(&config);
    assert_eq!(legacy.matches('"').count(), 6);
    assert_eq!(windows.matches('"').count(), 0);
    assert!(windows.contains("--manifest-token m1_"));

    // The legacy HookStat command has the exact risky nested-quote shape
    // described by openai/codex#38168. Exact observed failure is token-layout
    // dependent, so this test proves the shape and runs the corrected command
    // through the same Codex `raw_arg` spawn form.
    assert!(legacy.contains("--manifest \""));
    assert!(legacy.contains("--handler \""));

    let spool = ReceiptSpool::open(data_root.join("receipts")).unwrap();
    assert!(spool.scan().invocations.is_empty());

    let fixed_output = codex_0147_outer_quoted_output_with_path(&windows, executable.parent());
    assert_eq!(fixed_output.status.code(), Some(7));
    let scan = spool.scan();
    assert_eq!(scan.invocations.len(), 1);
    assert_eq!(scan.starts_without_completion, 0);
    assert_eq!(scan.invocations[0].terminal_status, TerminalStatus::Failed);
}

#[test]
fn leading_quoted_executable_is_cmd_compatible_but_not_powershell_compatible() {
    let temp = tempdir().unwrap();
    let config_root = temp.path().join("Config Root \u{6d4b}\u{8bd5}");
    let config = config_root.join("hooks.json");
    let data_root = temp.path().join("HookStat Data \u{6d4b}\u{8bd5}");
    let executable = temp
        .path()
        .join("HookStat Executable \u{6d4b}\u{8bd5}")
        .join("hookstat.exe");
    fs::create_dir_all(&config_root).unwrap();
    fs::create_dir_all(executable.parent().unwrap()).unwrap();
    fs::copy(env!("CARGO_BIN_EXE_hookstat"), &executable).unwrap();
    fs::write(
        &config,
        br#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"exit /b 7"}]}]}}"#,
    )
    .unwrap();

    let discovery = discover_paths(std::slice::from_ref(&config)).unwrap();
    assert_eq!(
        apply(&discovery, &data_root, &executable).unwrap().applied,
        1
    );
    let (_, windows) = first_commands(&config);
    let former_windows = format!(
        "\"{}\"{}",
        executable.display(),
        windows.strip_prefix("hookstat.exe").unwrap()
    );
    let spool = ReceiptSpool::open(data_root.join("receipts")).unwrap();

    // The current form is valid when Codex chooses cmd.exe and preserves its
    // raw outer quote behavior.
    assert_eq!(
        codex_0147_outer_quoted_output(&former_windows)
            .status
            .code(),
        Some(7)
    );
    assert_eq!(spool.scan().invocations.len(), 1);

    // The same leading quoted executable is a PowerShell string expression,
    // not a command invocation. No proxy receipt may be written.
    assert!(
        !codex_0147_powershell_output(&former_windows)
            .status
            .success()
    );
    assert_eq!(spool.scan().invocations.len(), 1);

    // PowerShell's call operator demonstrates that this is the executable
    // entry rule, rather than a manifest, handler, or proxy failure.
    let call_operator = format!("& {former_windows}");
    // PowerShell itself normalizes an unhandled native non-zero exit to `1`;
    // receipt growth proves that the call operator did enter the proxy.
    assert!(
        !codex_0147_powershell_output(&call_operator)
            .status
            .success()
    );
    assert_eq!(spool.scan().invocations.len(), 2);

    assert_eq!(
        codex_0147_outer_quoted_output_with_path(&windows, executable.parent())
            .status
            .code(),
        Some(7)
    );
    assert_eq!(spool.scan().invocations.len(), 3);
    assert!(
        !codex_0147_powershell_output_with_path(&windows, executable.parent())
            .status
            .success()
    );
    assert_eq!(spool.scan().invocations.len(), 4);
}

#[test]
fn shell_neutral_command_preserves_proxy_behavior_in_both_codex_shell_families() {
    let temp = tempdir().unwrap();
    let config_root = temp.path().join("Config Root");
    let config = config_root.join("hooks.json");
    let data_root = temp.path().join("HookStat Data \u{6d4b}\u{8bd5}");
    let executable = temp
        .path()
        .join("HookStat PATH \u{7a7a}\u{53e3}")
        .join("hookstat.exe");
    let cwd = temp.path().join("cwd-preserved");
    fs::create_dir_all(&config_root).unwrap();
    fs::create_dir_all(executable.parent().unwrap()).unwrap();
    fs::create_dir_all(&cwd).unwrap();
    fs::copy(env!("CARGO_BIN_EXE_hookstat"), &executable).unwrap();
    fs::write(
        &config,
        br#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"more & echo stdout-%HOOKSTAT_SHELL_E2E% & echo stderr-%HOOKSTAT_SHELL_E2E% 1>&2 & cd & exit /b 7"}]}]}}"#,
    )
    .unwrap();

    let discovery = discover_paths(std::slice::from_ref(&config)).unwrap();
    assert_eq!(
        apply(&discovery, &data_root, &executable).unwrap().applied,
        1
    );
    let (_, windows) = first_commands(&config);
    assert_eq!(windows.matches('"').count(), 0);
    assert_eq!(
        windows
            .bytes()
            .filter(|byte| matches!(
                byte,
                b'&' | b'|' | b'<' | b'>' | b'^' | b'(' | b')' | b'%' | b'!'
            ))
            .count(),
        0
    );

    let input = b"stdin-preserved\r\n";
    let cmd = codex_0147_outer_quoted_output_with_context(
        &windows,
        executable.parent().unwrap(),
        input,
        &cwd,
    );
    assert_eq!(cmd.status.code(), Some(7));
    let cmd_stdout = String::from_utf8_lossy(&cmd.stdout);
    assert!(cmd_stdout.contains("stdin-preserved"));
    assert!(cmd_stdout.contains("stdout-preserved"));
    assert!(cmd_stdout.contains("cwd-preserved"));
    assert!(String::from_utf8_lossy(&cmd.stderr).contains("stderr-preserved"));

    let powershell = codex_0147_powershell_output_with_context(
        &windows,
        executable.parent().unwrap(),
        input,
        &cwd,
    );
    // PowerShell normalizes the nonzero native exit of an otherwise complete
    // hook, but the proxy's receipt retains the original handler exit code.
    assert!(!powershell.status.success());
    let powershell_stdout = String::from_utf8_lossy(&powershell.stdout);
    assert!(powershell_stdout.contains("stdin-preserved"));
    assert!(powershell_stdout.contains("stdout-preserved"));
    assert!(powershell_stdout.contains("cwd-preserved"));
    assert!(String::from_utf8_lossy(&powershell.stderr).contains("stderr-preserved"));

    let spool = ReceiptSpool::open(data_root.join("receipts")).unwrap();
    let scan = spool.scan();
    assert_eq!(scan.invocations.len(), 2);
    assert_eq!(scan.starts_without_completion, 0);
    assert!(
        scan.invocations
            .iter()
            .all(|invocation| invocation.terminal_status == TerminalStatus::Failed)
    );
    let completion_exit_codes = fs::read_dir(data_root.join("receipts/records"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .ends_with(".complete.json")
        })
        .map(|entry| serde_json::from_slice::<Value>(&fs::read(entry.path()).unwrap()).unwrap())
        .map(|completion| completion["exit_code"].as_i64())
        .collect::<Vec<_>>();
    assert_eq!(completion_exit_codes, vec![Some(7), Some(7)]);
}

#[test]
fn apply_requires_the_first_path_hookstat_to_be_the_running_executable() {
    let temp = tempdir().unwrap();
    let config_root = temp.path().join("Config Root");
    let config = config_root.join("hooks.json");
    let data_root = temp.path().join("HookStat Data \u{6d4b}\u{8bd5}");
    let installed_directory = temp.path().join("HookStat PATH \u{7a7a}\u{53e3}");
    let installed = installed_directory.join("hookstat.exe");
    fs::create_dir_all(&config_root).unwrap();
    fs::create_dir_all(&installed_directory).unwrap();
    fs::copy(env!("CARGO_BIN_EXE_hookstat"), &installed).unwrap();
    let original = br#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"exit /b 0"},{"type":"command","command":"exit /b 0","commandWindows":"exit /b 0"}]}]}}"#;
    fs::write(&config, original).unwrap();
    let config_root_text = config_root.to_str().unwrap();
    let data_root_text = data_root.to_str().unwrap();

    let apply = binary_output_with_path(
        &installed,
        &[
            "codex",
            "instrument",
            "--apply",
            "--config-root",
            config_root_text,
            "--data-root",
            data_root_text,
        ],
        &[&installed_directory],
    );
    assert!(apply.status.success());
    let applied: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    for handler in applied["hooks"]["Stop"][0]["hooks"].as_array().unwrap() {
        let command = handler["commandWindows"].as_str().unwrap();
        assert!(command.starts_with("hookstat.exe codex proxy "));
        assert_eq!(command.matches('"').count(), 0);
    }

    let restored = binary_output_with_path(
        &installed,
        &[
            "codex",
            "instrument",
            "--restore",
            "--config-root",
            config_root_text,
            "--data-root",
            data_root_text,
        ],
        &[&installed_directory],
    );
    assert!(restored.status.success());
    assert_eq!(fs::read(&config).unwrap(), original);

    let runner_directory = temp.path().join("runner");
    let shadow_directory = temp.path().join("shadow");
    let runner = runner_directory.join("hookstat.exe");
    fs::create_dir_all(&runner_directory).unwrap();
    fs::create_dir_all(&shadow_directory).unwrap();
    fs::copy(env!("CARGO_BIN_EXE_hookstat"), &runner).unwrap();
    fs::copy(
        env!("CARGO_BIN_EXE_hookstat"),
        shadow_directory.join("hookstat.exe"),
    )
    .unwrap();
    let shadowed = binary_output_with_path(
        &runner,
        &[
            "codex",
            "instrument",
            "--apply",
            "--config-root",
            config_root_text,
            "--data-root",
            data_root_text,
        ],
        &[&shadow_directory, &runner_directory],
    );
    assert!(!shadowed.status.success());
    assert!(String::from_utf8_lossy(&shadowed.stderr).contains("requires hookstat.exe on PATH"));
    assert_eq!(fs::read(&config).unwrap(), original);
}

#[test]
fn malformed_manifest_tokens_and_handler_key_injection_fail_closed() {
    let binary = env!("CARGO_BIN_EXE_hookstat");
    for (manifest_token, handler) in [
        ("m1_abc&whoami", "hk_0123abcd"),
        ("m1_YQ", "handler&whoami"),
        ("m1_YQ", "handler|more"),
        ("m1_YQ", "handler key"),
        ("m1_YQ", "handler\"quoted"),
    ] {
        let output = Command::new(binary)
            .args([
                "codex",
                "proxy",
                "--manifest-token",
                manifest_token,
                "--handler",
                handler,
            ])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(2),
            "{manifest_token} / {handler}"
        );
    }
}
