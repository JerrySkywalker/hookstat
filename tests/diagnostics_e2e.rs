//! CLI diagnostics contract: read-only, JSON-safe, and explicitly exported.

use hookstat::domain::{
    EvidenceCoverage, EvidenceKind, ExecutionMode, HandlerIdentity, HookEvent, HookInvocation,
    Runtime, TerminalStatus,
};
use hookstat::ledger::Ledger;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
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

#[test]
fn read_only_report_projects_existing_ledger_without_creating_a_receipt_spool() {
    let temporary = tempdir().unwrap();
    let data_root = temporary.path().join("HookStat");
    fs::create_dir_all(&data_root).unwrap();
    let ledger_path = data_root.join("ledger.sqlite3");
    let mut ledger = Ledger::open_path(&ledger_path).unwrap();
    // This CLI contract projects the selected (default 7d) finite period. Keep
    // the fixture admitted to that period; Unix epoch data would be correctly
    // excluded by the v0.2.1 bounded query and would test no useful behavior.
    let occurred_at_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    ledger
        .ingest(&[HookInvocation {
            source_key: "fixture".into(),
            source_record_id: "one".into(),
            runtime: Runtime::Codex,
            evidence_kind: EvidenceKind::SyntheticFixture,
            coverage: EvidenceCoverage::SyntheticFixture,
            handler: HandlerIdentity {
                key: "hk_fixture".into(),
                revision: "revision-1".into(),
                label: "Fixture hook".into(),
                source_kind: "fixture".into(),
                event: HookEvent::Stop,
                matcher_identity: "fixture".into(),
                structural_identity: "fixture".into(),
                execution_mode: ExecutionMode::Sync,
            },
            occurred_at_unix_ms,
            terminal_status: TerminalStatus::Failed,
            duration_ms: None,
            error_fingerprint: Some("exit_nonzero".into()),
        }])
        .unwrap();
    drop(ledger);
    let before = fs::read(&ledger_path).unwrap();
    let binary = env!("CARGO_BIN_EXE_hookstat");
    let output = run(
        binary,
        &[
            "report",
            "--json",
            "--read-only",
            "--data-root",
            data_root.to_str().unwrap(),
        ],
        temporary.path(),
    );
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema_version"], 3);
    assert_eq!(report["receipt_integrity_observed"], false);
    assert_eq!(report["intelligence"][0]["handler_key"], "hk_fixture");
    assert_eq!(fs::read(&ledger_path).unwrap(), before);
    assert!(!data_root.join("receipts").exists());
}
