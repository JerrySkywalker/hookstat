//! CLI diagnostics contract: read-only, JSON-safe, and explicitly exported.

use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn run(binary: &str, arguments: &[&str], app_data: &Path) -> std::process::Output {
    Command::new(binary)
        .args(arguments)
        .env("LOCALAPPDATA", app_data)
        .env("APPDATA", app_data)
        .env("XDG_DATA_HOME", app_data)
        .output()
        .unwrap()
}

#[test]
fn doctor_json_and_explicit_export_are_sanitized_and_do_not_create_data_state() {
    let temporary = tempdir().unwrap();
    let state_parent = temporary.path().join("state-parent");
    let data_root = state_parent.join("HookStat");
    let export_path = temporary.path().join("diagnostics.json");
    let binary = env!("CARGO_BIN_EXE_hookstat");
    let data_root_text = data_root.to_str().unwrap();

    let doctor = run(
        binary,
        &["doctor", "--json", "--data-root", data_root_text],
        &state_parent,
    );
    assert!(doctor.status.success());
    let doctor_json: Value = serde_json::from_slice(&doctor.stdout).unwrap();
    assert_eq!(doctor_json["read_only"], true);
    assert_eq!(doctor_json["schema_version"], 1);
    assert!(!data_root.exists());
    let doctor_text = String::from_utf8(doctor.stdout).unwrap();
    for forbidden in ["prompt", "stdout", "stderr", "credential", "command"] {
        assert!(!doctor_text.contains(forbidden));
    }

    let preview = run(
        binary,
        &[
            "diagnostics",
            "export",
            "--output",
            export_path.to_str().unwrap(),
            "--data-root",
            data_root_text,
        ],
        &state_parent,
    );
    assert!(preview.status.success());
    assert!(!export_path.exists());

    let exported = run(
        binary,
        &[
            "diagnostics",
            "export",
            "--output",
            export_path.to_str().unwrap(),
            "--apply",
            "--data-root",
            data_root_text,
        ],
        &state_parent,
    );
    assert!(exported.status.success());
    let export_json: Value = serde_json::from_slice(&fs::read(export_path).unwrap()).unwrap();
    assert_eq!(export_json["read_only"], true);
}
