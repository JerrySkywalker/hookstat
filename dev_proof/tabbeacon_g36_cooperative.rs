use hookstat::ipc::{BrokerConfig, BrokerHost};
use hookstat_ipc_client_proof::{
    Completion, CooperativeProducer, ExitClassification, LifecycleFrame, ObservationDisposition,
    TerminalOutcome,
};
use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tabbeacon::providers::codex::{CodexHookRuntime, HookDispatchOutcome};

fn git(cwd: &Path, arguments: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(arguments)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "Never")
        .status()
        .expect("disposable Git fixture starts");
    assert!(status.success(), "disposable Git fixture command failed");
}

#[test]
fn current_hookstat_source_observes_real_tabbeacon_runtime_without_a_wrapper() {
    let root = std::env::temp_dir().join(format!(
        "tabbeacon-g36-current-proof-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    let repository = root.join("repository");
    let tabbeacon_state = root.join("tabbeacon-state");
    let hookstat_state = root.join("hookstat-state");
    fs::create_dir_all(&repository).expect("create disposable repository");
    git(&repository, &["init", "--quiet"]);
    fs::write(repository.join("README.md"), "controlled G36 proof\n")
        .expect("write disposable repository fixture");
    git(&repository, &["add", "README.md"]);
    git(
        &repository,
        &[
            "-c",
            "user.name=G36 Proof",
            "-c",
            "user.email=g36-proof@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "fixture",
        ],
    );
    git(
        &repository,
        &[
            "remote",
            "add",
            "origin",
            "https://example.invalid/controlled/g36-proof.git",
        ],
    );

    let host = BrokerHost::start(BrokerConfig::for_state_root(&hookstat_state))
        .expect("start disposable HookStat broker");
    let producer = CooperativeProducer::for_state_root(&hookstat_state)
        .expect("construct current-source cooperative producer");
    let lifecycle = LifecycleFrame {
        runtime: "codex".into(),
        runtime_instance: "controlled_tabbeacon".into(),
        invocation: "controlled_g36_invocation".into(),
        handler: "tabbeacon_codex_hook".into(),
        event: "UserPromptSubmit".into(),
        source_scope: "controlled_proof".into(),
        revision: Some("tabbeacon_b3f5685".into()),
        occurred_at_unix_ms: 1_700_000_000_000,
    };
    assert_eq!(
        producer.emit_start(lifecycle.clone()),
        ObservationDisposition::Accepted
    );

    let payload = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "controlled-g36-session",
        "cwd": repository,
        "model": "controlled-model",
        "permission_mode": "default",
        "transcript_path": null,
        "turn_id": "controlled-turn"
    });
    let runtime = CodexHookRuntime::new(&tabbeacon_state, true);
    let mut output = Vec::new();
    let outcome = runtime.dispatch_to(
        &serde_json::to_vec(&payload).expect("serialize controlled payload"),
        UNIX_EPOCH,
        &mut output,
    );
    assert!(matches!(
        outcome,
        HookDispatchOutcome::Applied | HookDispatchOutcome::PreservedCurrentState
    ));
    assert_eq!(
        producer.emit_complete(
            lifecycle,
            Completion {
                terminal_status: TerminalOutcome::Completed,
                exit_classification: ExitClassification::ExitCode,
                exit_value: Some(0),
                duration_ms: 1,
            },
        ),
        ObservationDisposition::Accepted
    );
    assert_eq!(host.health().accepted, 2);
    host.stop();

    let configuration_source = include_str!("../src/providers/codex/config.rs");
    assert!(configuration_source.contains("format!(\"{executable} hook codex\")"));
    assert!(!configuration_source.contains("hookstat-hook"));
    let _ = fs::remove_dir_all(root);
}
