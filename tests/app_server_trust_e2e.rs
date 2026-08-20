#![cfg(windows)]

use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::tempdir;

fn run(binary: &str, arguments: &[&str], codex_home: &Path, _data_root: &Path) -> Output {
    Command::new(binary)
        .args(arguments)
        .env("CODEX_HOME", codex_home)
        .env("HOOKSTAT_TEST_ISOLATED_APP_SERVER", "1")
        .output()
        .unwrap()
}

/// This deliberately runs only when an installed Codex App Server is available.
/// It uses a temporary CODEX_HOME and data root, never an Owner configuration.
#[test]
#[ignore = "requires the locally installed Codex App Server"]
fn official_app_server_trust_upserts_only_current_instrumentation_targets() {
    let temp = tempdir().unwrap();
    let codex_home = temp.path().join("codex-home");
    let data_root = temp.path().join("hookstat-data");
    fs::create_dir_all(&codex_home).unwrap();
    let config = codex_home.join("hooks.json");
    let original = br#"{
  "hooks": {
    "Stop": [{"hooks": [
      {"type": "command", "command": "exit /b 0"},
      {"type": "command", "command": "exit /b 0"}
    ]}]
  }
}"#;
    fs::write(&config, original).unwrap();
    let binary = env!("CARGO_BIN_EXE_hookstat");
    let config_root = codex_home.to_str().unwrap();
    let data_root_text = data_root.to_str().unwrap();
    let apply = run(
        binary,
        &[
            "codex",
            "instrument",
            "--apply",
            "--config-root",
            config_root,
            "--data-root",
            data_root_text,
        ],
        &codex_home,
        &data_root,
    );
    assert!(apply.status.success());
    let preflight = run(
        binary,
        &[
            "codex",
            "instrument",
            "--trust",
            "--dry-run",
            "--config-root",
            config_root,
            "--data-root",
            data_root_text,
        ],
        &codex_home,
        &data_root,
    );
    assert!(preflight.status.success());
    let preflight: Value = serde_json::from_slice(&preflight.stdout).unwrap();
    assert_eq!(preflight["targets"], 2);
    assert_eq!(preflight["writes"], 0);
    let trusted = run(
        binary,
        &[
            "codex",
            "instrument",
            "--trust",
            "--config-root",
            config_root,
            "--data-root",
            data_root_text,
        ],
        &codex_home,
        &data_root,
    );
    assert!(trusted.status.success());
    let trusted: Value = serde_json::from_slice(&trusted.stdout).unwrap();
    assert_eq!(trusted["targets"], 2);
    assert_eq!(trusted["writes"], 2);
    assert_eq!(trusted["verified"], 2);
    let repeat = run(
        binary,
        &[
            "codex",
            "instrument",
            "--trust",
            "--config-root",
            config_root,
            "--data-root",
            data_root_text,
        ],
        &codex_home,
        &data_root,
    );
    assert!(repeat.status.success());
    let repeat: Value = serde_json::from_slice(&repeat.stdout).unwrap();
    assert_eq!(repeat["writes"], 0);
    assert_eq!(repeat["verified"], 2);
    let restore = run(
        binary,
        &[
            "codex",
            "instrument",
            "--restore",
            "--config-root",
            config_root,
            "--data-root",
            data_root_text,
        ],
        &codex_home,
        &data_root,
    );
    assert!(restore.status.success());
    assert_eq!(fs::read(&config).unwrap(), original);
}
