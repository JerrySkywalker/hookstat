//! Production-path rehearsal using only sanitized temporary configuration.

use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

#[cfg(windows)]
const SUCCESS: &str = "exit /b 0";
#[cfg(not(windows))]
const SUCCESS: &str = "exit 0";
#[cfg(windows)]
const FAILURE: &str = "exit /b 7";
#[cfg(not(windows))]
const FAILURE: &str = "exit 7";

fn run(binary: &str, args: &[&str], local_app_data: &Path) -> std::process::Output {
    Command::new(binary)
        .args(args)
        .env("LOCALAPPDATA", local_app_data)
        .env("APPDATA", local_app_data)
        .output()
        .unwrap()
}

#[test]
fn shadow_apply_proxy_ingest_report_and_restore_match_known_fixture_counts() {
    let temp = tempdir().unwrap();
    let config_root = temp.path().join("shadow");
    let state_parent = temp.path().join("data");
    let data_root = state_parent.join("HookStat");
    fs::create_dir_all(&config_root).unwrap();
    let config = config_root.join("hooks.json");
    let original = format!(
        r#"{{"hooks":{{"Stop":[{{"hooks":[{{"type":"command","command":"{SUCCESS}"}},{{"type":"command","command":"{FAILURE}"}}]}}]}}}}"#
    );
    fs::write(&config, &original).unwrap();
    let binary = env!("CARGO_BIN_EXE_hookstat");

    let config_root_text = config_root.to_str().unwrap();
    let data_root_text = data_root.to_str().unwrap();
    let dry_run = run(
        binary,
        &[
            "codex",
            "instrument",
            "--dry-run",
            "--config-root",
            config_root_text,
        ],
        &state_parent,
    );
    assert!(dry_run.status.success());
    let plan: Value = serde_json::from_slice(&dry_run.stdout).unwrap();
    assert_eq!(plan["discovered"], 2);
    assert_eq!(plan["instrumentable"], 2);
    let keys = plan["handlers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value["handler"]["key"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();

    let apply = run(
        binary,
        &[
            "codex",
            "instrument",
            "--apply",
            "--config-root",
            config_root_text,
            "--data-root",
            data_root_text,
        ],
        &state_parent,
    );
    assert!(apply.status.success());
    let manifest = fs::read_dir(data_root.join("manifests"))
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .next()
        .unwrap();
    for key in &keys {
        let proxy = run(
            binary,
            &[
                "codex",
                "proxy",
                "--manifest",
                manifest.to_str().unwrap(),
                "--handler",
                key,
            ],
            &state_parent,
        );
        assert!(matches!(proxy.status.code(), Some(0) | Some(7)));
    }

    let report = run(binary, &["report", "--json"], &state_parent);
    assert!(report.status.success());
    let report: Value = serde_json::from_slice(&report.stdout).unwrap();
    assert_eq!(report["handlers"].as_array().unwrap().len(), 2);
    let total_runs = report["handlers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["runs"].as_u64().unwrap())
        .sum::<u64>();
    let total_failures = report["handlers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["failed_runs"].as_u64().unwrap())
        .sum::<u64>();
    assert_eq!(total_runs, 2);
    assert_eq!(total_failures, 1);

    let restore = run(
        binary,
        &[
            "codex",
            "instrument",
            "--restore",
            "--config-root",
            config_root_text,
            "--data-root",
            data_root_text,
        ],
        &state_parent,
    );
    assert!(restore.status.success());
    assert_eq!(fs::read_to_string(config).unwrap(), original);
}
