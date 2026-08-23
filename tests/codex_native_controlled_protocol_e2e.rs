//! Controlled, opt-in L1 qualification against an installed Codex App Server.
//!
//! This test is ignored in ordinary CI because it needs the locally installed
//! App Server. Its own CODEX_HOME, workspace, hook declarations, and trust
//! state are all temporary. It never reads or writes the Owner's live root.

use hookstat::domain::{EvidenceCoverage, TerminalStatus};
use hookstat::evidence::{
    CorrelationOutcome, EvidenceCorrelator, InvocationCoverage, SourceCoverage,
};
use hookstat::native::{NativeEvidenceReader, NativeNormalizer};
use hookstat::runtime::codex::{CodexNativeCursor, CodexNativeIntegration, CodexProtocolVersion};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;
use tempfile::tempdir;

const RESPONSE_TIMEOUT: Duration = Duration::from_secs(20);

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn app_server_command() -> Command {
    #[cfg(windows)]
    {
        let shim = std::env::var_os("PATH")
            .map(|value| std::env::split_paths(&value).collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .map(|path| path.join("codex.cmd"))
            .find(|path| path.is_file());
        let mut command = Command::new(shim.unwrap_or_else(|| "codex".into()));
        command.arg("app-server");
        command
    }
    #[cfg(not(windows))]
    {
        let mut command = Command::new("codex");
        command.arg("app-server");
        command
    }
}

fn send(stdin: &mut ChildStdin, value: &Value) {
    serde_json::to_writer(&mut *stdin, value).unwrap();
    stdin.write_all(b"\n").unwrap();
    stdin.flush().unwrap();
}

fn receive_response(receiver: &Receiver<Value>, id: u64) -> Value {
    loop {
        let value = receiver.recv_timeout(RESPONSE_TIMEOUT).unwrap();
        if value.get("id").and_then(Value::as_u64) == Some(id) {
            assert!(value.get("error").is_none(), "App Server request failed");
            return value;
        }
    }
}

fn request(
    stdin: &mut ChildStdin,
    receiver: &Receiver<Value>,
    id: u64,
    method: &str,
    params: Value,
) -> Value {
    send(
        stdin,
        &json!({"method": method, "id": id, "params": params}),
    );
    receive_response(receiver, id)
}

fn trust_controlled_hooks(
    stdin: &mut ChildStdin,
    receiver: &Receiver<Value>,
    cwd: &str,
    expected_handlers: usize,
) -> Value {
    let before = request(stdin, receiver, 2, "hooks/list", json!({"cwds": [cwd]}));
    let contexts = before["result"]["data"].as_array().unwrap();
    let mut trusted = serde_json::Map::new();
    for context in contexts {
        for hook in context["hooks"].as_array().unwrap() {
            trusted.insert(
                hook["key"].as_str().unwrap().to_owned(),
                json!({"trusted_hash": hook["currentHash"].as_str().unwrap()}),
            );
        }
    }
    assert_eq!(
        trusted.len(),
        expected_handlers,
        "controlled fixture exposed an unexpected handler count"
    );
    let _ = request(
        stdin,
        receiver,
        3,
        "config/batchWrite",
        json!({
            "edits": [{
                "keyPath": "hooks.state",
                "value": Value::Object(trusted),
                "mergeStrategy": "upsert"
            }],
            "reloadUserConfig": true
        }),
    );
    let after = request(stdin, receiver, 4, "hooks/list", json!({"cwds": [cwd]}));
    for context in after["result"]["data"].as_array().unwrap() {
        for hook in context["hooks"].as_array().unwrap() {
            assert_eq!(hook["trustStatus"].as_str(), Some("trusted"));
            assert_eq!(hook["enabled"].as_bool(), Some(true));
        }
    }
    after
}

fn controlled_lifecycle(
    codex_home: &std::path::Path,
    workspace: &std::path::Path,
    expected_handlers: usize,
) -> (Value, Vec<Value>) {
    let mut child = app_server_command()
        .env("CODEX_HOME", codex_home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("installed Codex App Server must be launchable");
    let stdout = child.stdout.take().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Ok(value) = serde_json::from_str::<Value>(&line) {
                let _ = sender.send(value);
            }
        }
    });
    let _child = ChildGuard(child);

    send(
        &mut stdin,
        &json!({
            "method": "initialize",
            "id": 1,
            "params": {"clientInfo": {"name": "hookstat-g34-qualified", "version": "0.3.0"}}
        }),
    );
    let _ = receive_response(&receiver, 1);
    send(&mut stdin, &json!({"method": "initialized", "params": {}}));

    let cwd = workspace.to_str().unwrap();
    let hooks_list = trust_controlled_hooks(&mut stdin, &receiver, cwd, expected_handlers);
    let thread = request(
        &mut stdin,
        &receiver,
        5,
        "thread/start",
        json!({"cwd": cwd, "ephemeral": true}),
    );
    let thread_id = thread["result"]["thread"]["id"].as_str().unwrap();
    let _ = request(
        &mut stdin,
        &receiver,
        6,
        "turn/start",
        json!({
            "threadId": thread_id,
            "input": [{"type": "text", "text": "native-l1-controlled"}]
        }),
    );

    let mut lifecycle = Vec::new();
    while lifecycle
        .iter()
        .filter(|value: &&Value| value["method"] == "hook/started")
        .count()
        < expected_handlers
        || lifecycle
            .iter()
            .filter(|value: &&Value| value["method"] == "hook/completed")
            .count()
            < expected_handlers
    {
        let value = receiver.recv_timeout(RESPONSE_TIMEOUT).unwrap();
        if matches!(
            value.get("method").and_then(Value::as_str),
            Some("hook/started" | "hook/completed")
        ) {
            lifecycle.push(value);
        }
    }
    (hooks_list, lifecycle)
}

/// Requires a deliberate local invocation, for example:
/// `cargo +1.97.1 test --test codex_native_controlled_protocol_e2e -- --ignored`.
#[test]
#[ignore = "requires an installed Codex App Server and runs only in a disposable CODEX_HOME"]
fn controlled_app_server_proves_native_l1_without_hookstat_proxying() {
    let temp = tempdir().unwrap();
    let codex_home = temp.path().join("codex-home");
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&codex_home).unwrap();
    std::fs::create_dir_all(&workspace).unwrap();
    // The commands exist only in the disposable root, are never captured, and
    // deliberately exercise one success and one non-success terminal state.
    std::fs::write(
        codex_home.join("hooks.json"),
        r#"{
  "hooks": {
    "SessionStart": [{"hooks": [
      {"type": "command", "command": "cmd /c exit 0"},
      {"type": "command", "command": "cmd /c exit 7"}
    ]}]
  }
}"#,
    )
    .unwrap();

    let mut child = app_server_command()
        .env("CODEX_HOME", &codex_home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("installed Codex App Server must be launchable");
    let stdout = child.stdout.take().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Ok(value) = serde_json::from_str::<Value>(&line) {
                let _ = sender.send(value);
            }
        }
    });
    let _child = ChildGuard(child);

    send(
        &mut stdin,
        &json!({
            "method": "initialize",
            "id": 1,
            "params": {"clientInfo": {"name": "hookstat-g34-qualified", "version": "0.3.0"}}
        }),
    );
    let _ = receive_response(&receiver, 1);
    send(&mut stdin, &json!({"method": "initialized", "params": {}}));

    let cwd = workspace.to_str().unwrap();
    let hooks_list = trust_controlled_hooks(&mut stdin, &receiver, cwd, 2);
    let thread = request(
        &mut stdin,
        &receiver,
        5,
        "thread/start",
        json!({"cwd": cwd, "ephemeral": true}),
    );
    let thread_id = thread["result"]["thread"]["id"].as_str().unwrap();
    let _ = request(
        &mut stdin,
        &receiver,
        6,
        "turn/start",
        json!({
            "threadId": thread_id,
            "input": [{"type": "text", "text": "native-l1-controlled"}]
        }),
    );

    let mut lifecycle = Vec::new();
    while lifecycle
        .iter()
        .filter(|value: &&Value| value["method"] == "hook/started")
        .count()
        < 2
        || lifecycle
            .iter()
            .filter(|value: &&Value| value["method"] == "hook/completed")
            .count()
            < 2
    {
        let value = receiver.recv_timeout(RESPONSE_TIMEOUT).unwrap();
        if matches!(
            value.get("method").and_then(Value::as_str),
            Some("hook/started" | "hook/completed")
        ) {
            lifecycle.push(value);
        }
    }

    let mut integration =
        CodexNativeIntegration::with_hooks_list(&CodexProtocolVersion::tested(), &hooks_list)
            .unwrap();
    for value in lifecycle {
        integration.reader.ingest_json(value).unwrap();
    }
    let records = integration
        .reader
        .read(&mut CodexNativeCursor::default())
        .unwrap();
    assert_eq!(records.len(), 4);
    let canonical = records
        .iter()
        .map(|record| integration.normalizer.normalize(record).unwrap())
        .collect::<Vec<_>>();
    let mut correlator = EvidenceCorrelator::default();
    let mut invocations = Vec::new();
    for evidence in canonical {
        if let CorrelationOutcome::Produced(correlated) = correlator.observe(evidence).unwrap()
            && correlated.invocation_coverage == InvocationCoverage::Complete
        {
            invocations.push(
                integration
                    .normalizer
                    .identity_resolver()
                    .qualification_invocation(&correlated)
                    .unwrap(),
            );
        }
    }
    assert_eq!(invocations.len(), 2);
    assert!(
        invocations
            .iter()
            .any(|value| value.terminal_status == TerminalStatus::Completed)
    );
    assert!(
        invocations
            .iter()
            .any(|value| value.terminal_status == TerminalStatus::Failed)
    );
    assert!(invocations.iter().all(|value| value.duration_ms.is_some()));
    assert!(
        invocations
            .iter()
            .all(|value| value.coverage == EvidenceCoverage::NotAdmitted)
    );
    assert_ne!(invocations[0].handler.key, invocations[1].handler.key);
}

fn write_single_controlled_hook(codex_home: &std::path::Path, command: &str) {
    std::fs::write(
        codex_home.join("hooks.json"),
        format!(
            r#"{{
  "hooks": {{
    "SessionStart": [{{"hooks": [{{"type": "command", "command": "{command}"}}]}}]
  }}
}}"#
        ),
    )
    .unwrap();
}

fn completed_canonical(
    hooks_list: Value,
    lifecycle: Vec<Value>,
) -> Vec<hookstat::evidence::CanonicalEvidence> {
    let mut integration =
        CodexNativeIntegration::with_hooks_list(&CodexProtocolVersion::tested(), &hooks_list)
            .unwrap();
    for value in lifecycle {
        integration.reader.ingest_json(value).unwrap();
    }
    let mut cursor = CodexNativeCursor::default();
    integration
        .reader
        .read(&mut cursor)
        .unwrap()
        .iter()
        .filter_map(|record| {
            let evidence = integration.normalizer.normalize(record).unwrap();
            (evidence.lifecycle == hookstat::evidence::EvidenceLifecycle::Completed)
                .then_some(evidence)
        })
        .collect()
}

#[test]
#[ignore = "requires an installed Codex App Server and runs only in a disposable CODEX_HOME"]
fn controlled_restart_and_config_change_keep_native_identity_limited() {
    let temp = tempdir().unwrap();
    let codex_home = temp.path().join("codex-home");
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&codex_home).unwrap();
    std::fs::create_dir_all(&workspace).unwrap();

    write_single_controlled_hook(&codex_home, "cmd /c exit 0");
    let (first_hooks_list, first_lifecycle) = controlled_lifecycle(&codex_home, &workspace, 1);
    let first = completed_canonical(first_hooks_list, first_lifecycle);
    let (second_hooks_list, second_lifecycle) = controlled_lifecycle(&codex_home, &workspace, 1);
    let second = completed_canonical(second_hooks_list, second_lifecycle);
    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    assert_eq!(first[0].runtime_handler_ref, second[0].runtime_handler_ref);
    assert_eq!(first[0].revision_ref, second[0].revision_ref);

    write_single_controlled_hook(&codex_home, "cmd /c exit 2");
    let (changed_hooks_list, changed_lifecycle) = controlled_lifecycle(&codex_home, &workspace, 1);
    let changed = completed_canonical(changed_hooks_list, changed_lifecycle);
    assert_eq!(changed.len(), 1);
    assert_eq!(first[0].runtime_handler_ref, changed[0].runtime_handler_ref);
    assert_ne!(first[0].revision_ref, changed[0].revision_ref);
    assert_eq!(changed[0].terminal_status, Some(TerminalStatus::Failed));
    assert_eq!(changed[0].source_coverage, SourceCoverage::IdentityLimited);
}
