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
fn codex_0147_outer_quoted_output(command_line: &str) -> Output {
    let shell = std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into());
    Command::new(shell)
        .arg("/C")
        .raw_arg(format!(r#""{command_line}""#))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap()
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
    assert_eq!(windows.matches('"').count(), 2);
    let first_close = windows[1..].find('"').unwrap() + 1;
    assert!(
        !windows[(first_close + 1)..].contains('"'),
        "the Windows command must not contain an embedded quote"
    );
    assert!(windows.contains("--manifest-token m1_"));

    // The legacy HookStat command has the exact risky nested-quote shape
    // described by openai/codex#38168. Exact observed failure is token-layout
    // dependent, so this test proves the shape and runs the corrected command
    // through the same Codex `raw_arg` spawn form.
    assert!(legacy.contains("--manifest \""));
    assert!(legacy.contains("--handler \""));

    let spool = ReceiptSpool::open(data_root.join("receipts")).unwrap();
    assert!(spool.scan().invocations.is_empty());

    let fixed_output = codex_0147_outer_quoted_output(&windows);
    assert_eq!(fixed_output.status.code(), Some(7));
    let scan = spool.scan();
    assert_eq!(scan.invocations.len(), 1);
    assert_eq!(scan.starts_without_completion, 0);
    assert_eq!(scan.invocations[0].terminal_status, TerminalStatus::Failed);
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
