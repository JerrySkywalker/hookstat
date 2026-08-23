use hookstat::domain::TerminalStatus;
use hookstat::evidence::{
    CorrelationOutcome, EvidenceCorrelator, EvidenceLifecycle, InvocationCoverage,
    NativeAdmissionState, SourceCoverage,
};
use hookstat::native::{
    CapabilityAssessment, NativeCapabilityProbe, NativeEvidenceReader, NativeNormalizer,
    RuntimeIdentityResolver,
};
use hookstat::runtime::codex::{
    CODEX_TESTED_CLI_VERSION, CodexNativeCapabilityProbe, CodexNativeCursor, CodexNativeError,
    CodexNativeIntegration, CodexNativeReader, CodexProtocolVersion,
};
use serde_json::{Value, json};
use std::path::PathBuf;

const PRIVATE_SOURCE_PATH: &str = "C:/controlled/private/qualification/hooks.json";
const PRIVATE_STATUS_TEXT: &str = "secret command and private hook output";

fn hooks_list() -> Value {
    json!({
        "result": {
            "data": [{
                "hooks": [
                    {
                        "eventName": "sessionStart",
                        "sourcePath": PRIVATE_SOURCE_PATH,
                        "displayOrder": 0,
                        "currentHash": "sha256:controlled-success-revision"
                    },
                    {
                        "eventName": "sessionStart",
                        "sourcePath": PRIVATE_SOURCE_PATH,
                        "displayOrder": 1,
                        "currentHash": "sha256:controlled-failure-revision"
                    }
                ]
            }]
        }
    })
}

fn notification(
    method: &str,
    handler_index: i64,
    status: &str,
    turn_id: &str,
    completed_at: Option<i64>,
    duration_ms: Option<i64>,
) -> Value {
    json!({
        "method": method,
        "params": {
            "threadId": "thread-private-opaque-input",
            "turnId": turn_id,
            "run": {
                "id": format!("session-start:{handler_index}:{PRIVATE_SOURCE_PATH}"),
                "eventName": "sessionStart",
                "executionMode": "sync",
                "scope": "thread",
                "sourcePath": PRIVATE_SOURCE_PATH,
                "source": "user",
                "displayOrder": handler_index,
                "status": status,
                "statusMessage": PRIVATE_STATUS_TEXT,
                "startedAt": 1_700_000_000_i64,
                "completedAt": completed_at,
                "durationMs": duration_ms,
                "entries": [{"kind": "context", "text": PRIVATE_STATUS_TEXT}]
            }
        }
    })
}

fn normalized(
    values: Vec<Value>,
) -> (
    hookstat::runtime::codex::CodexNativeIntegration,
    Vec<hookstat::evidence::CanonicalEvidence>,
) {
    let mut integration =
        CodexNativeIntegration::with_hooks_list(&CodexProtocolVersion::tested(), &hooks_list())
            .unwrap();
    for value in values {
        integration.reader.ingest_json(value).unwrap();
    }
    let mut cursor = CodexNativeCursor::default();
    let records = integration.reader.read(&mut cursor).unwrap();
    let canonical = records
        .iter()
        .map(|record| integration.normalizer.normalize(record).unwrap())
        .collect();
    (integration, canonical)
}

fn produced(value: CorrelationOutcome) -> hookstat::evidence::CorrelatedEvidence {
    match value {
        CorrelationOutcome::Produced(value) => value,
        CorrelationOutcome::Duplicate => panic!("expected correlated evidence"),
    }
}

#[test]
fn capability_matrix_is_deterministic_and_version_aware() {
    let probe = CodexNativeCapabilityProbe;
    let first = probe.probe(&CodexProtocolVersion::tested());
    let second = probe.probe(&CodexProtocolVersion::tested());
    assert_eq!(first, second);
    assert_eq!(first.admission, NativeAdmissionState::Qualified);
    assert_eq!(first.source_coverage, SourceCoverage::IdentityLimited);
    assert_eq!(
        first.stable_handler_attribution,
        CapabilityAssessment::NotProven
    );
    assert_eq!(
        first.replay_or_delivery_characteristics,
        CapabilityAssessment::NotProven
    );
    assert_eq!(first.facts().len(), 11);

    let incompatible = probe.probe(&CodexProtocolVersion::new(
        "0.150.0",
        "different-source-commit",
    ));
    assert_eq!(
        incompatible.version_compatibility,
        CapabilityAssessment::Incompatible
    );
    assert_eq!(incompatible.admission, NativeAdmissionState::Unavailable);
    assert_ne!(CODEX_TESTED_CLI_VERSION, "0.150.0");
    assert!(matches!(
        CodexNativeIntegration::with_hooks_list(
            &CodexProtocolVersion::new("0.150.0", "different-source-commit"),
            &hooks_list(),
        ),
        Err(CodexNativeError::IncompatibleProtocol)
    ));
}

#[test]
fn hook_started_and_completed_normalize_then_reach_qualification_invocation() {
    let (integration, canonical) = normalized(vec![
        notification("hook/started", 0, "running", "turn-a", None, None),
        notification(
            "hook/completed",
            0,
            "completed",
            "turn-a",
            Some(1_700_000_001),
            Some(42),
        ),
    ]);
    assert_eq!(canonical[0].lifecycle, EvidenceLifecycle::Started);
    assert_eq!(canonical[1].lifecycle, EvidenceLifecycle::Completed);
    assert_eq!(
        canonical[1].terminal_status,
        Some(TerminalStatus::Completed)
    );
    assert_eq!(canonical[1].duration_ms, Some(42));
    assert!(canonical[1].revision_ref.is_some());

    let mut correlator = EvidenceCorrelator::default();
    let started = produced(correlator.observe(canonical[0].clone()).unwrap());
    assert_eq!(started.terminal_status, TerminalStatus::Incomplete);
    assert_eq!(started.invocation_coverage, InvocationCoverage::Incomplete);
    let completed = produced(correlator.observe(canonical[1].clone()).unwrap());
    assert_eq!(completed.terminal_status, TerminalStatus::Completed);
    assert_eq!(completed.duration_ms, Some(42));
    let invocation = integration
        .normalizer
        .identity_resolver()
        .qualification_invocation(&completed)
        .unwrap();
    assert_eq!(invocation.terminal_status, TerminalStatus::Completed);
    assert_eq!(invocation.duration_ms, Some(42));
    assert_eq!(
        invocation.coverage,
        hookstat::domain::EvidenceCoverage::NotAdmitted
    );
    assert_eq!(
        invocation.handler.source_kind,
        "codex_native_location_limited"
    );
}

#[test]
fn terminal_statuses_and_duration_preserve_known_semantics() {
    for (wire, expected) in [
        ("completed", TerminalStatus::Completed),
        ("failed", TerminalStatus::Failed),
        ("blocked", TerminalStatus::Blocked),
        ("stopped", TerminalStatus::Stopped),
    ] {
        let (_, canonical) = normalized(vec![notification(
            "hook/completed",
            0,
            wire,
            "turn-status",
            Some(1_700_000_001),
            Some(57),
        )]);
        assert_eq!(canonical[0].terminal_status, Some(expected));
        assert_eq!(canonical[0].duration_ms, Some(57));
    }

    let (_, canonical) = normalized(vec![notification(
        "hook/completed",
        0,
        "completed",
        "turn-duration",
        Some(1_700_000_003),
        Some(999),
    )]);
    assert_eq!(canonical[0].occurred_at_unix_ms, 1_700_000_003_000);
    assert_eq!(canonical[0].duration_ms, Some(999));
}

#[test]
fn nonterminal_or_async_completion_is_not_misrepresented_as_success() {
    let mut integration =
        CodexNativeIntegration::with_hooks_list(&CodexProtocolVersion::tested(), &hooks_list())
            .unwrap();
    integration
        .reader
        .ingest_json(notification(
            "hook/completed",
            0,
            "running",
            "turn-running",
            Some(1_700_000_001),
            Some(1),
        ))
        .unwrap();
    let mut cursor = CodexNativeCursor::default();
    let record = integration.reader.read(&mut cursor).unwrap().remove(0);
    assert!(matches!(
        integration.normalizer.normalize(&record),
        Err(CodexNativeError::NonTerminalCompletion)
    ));

    let mut async_value = notification(
        "hook/completed",
        0,
        "completed",
        "turn-async",
        Some(1_700_000_001),
        Some(1),
    );
    async_value["params"]["run"]["executionMode"] = json!("async");
    let mut reader = CodexNativeReader::default();
    reader.ingest_json(async_value).unwrap();
    let record = reader
        .read(&mut CodexNativeCursor::default())
        .unwrap()
        .remove(0);
    assert!(matches!(
        integration.normalizer.normalize(&record),
        Err(CodexNativeError::UnexpectedExecutionMode)
    ));
}

#[test]
fn multiple_handlers_remain_distinct_and_duplicate_or_out_of_order_delivery_is_idempotent() {
    let (_, canonical) = normalized(vec![
        notification(
            "hook/completed",
            1,
            "failed",
            "turn-b",
            Some(1_700_000_002),
            Some(4),
        ),
        notification("hook/started", 1, "running", "turn-b", None, None),
        notification("hook/started", 0, "running", "turn-b", None, None),
        notification(
            "hook/completed",
            0,
            "completed",
            "turn-b",
            Some(1_700_000_003),
            Some(5),
        ),
    ]);
    assert_ne!(canonical[0].invocation_key, canonical[2].invocation_key);
    assert_ne!(
        canonical[0].runtime_handler_ref,
        canonical[2].runtime_handler_ref
    );

    let mut correlator = EvidenceCorrelator::default();
    let best_effort = produced(correlator.observe(canonical[0].clone()).unwrap());
    assert_eq!(
        best_effort.invocation_coverage,
        InvocationCoverage::BestEffort
    );
    let failed = produced(correlator.observe(canonical[1].clone()).unwrap());
    assert_eq!(failed.terminal_status, TerminalStatus::Failed);
    assert_eq!(
        correlator.observe(canonical[1].clone()).unwrap(),
        CorrelationOutcome::Duplicate
    );
    let _ = produced(correlator.observe(canonical[2].clone()).unwrap());
    let completed = produced(correlator.observe(canonical[3].clone()).unwrap());
    assert_eq!(completed.terminal_status, TerminalStatus::Completed);
}

#[test]
fn missing_identity_proof_lowers_coverage_without_fabricating_stability() {
    let mut reader = CodexNativeReader::default();
    reader
        .ingest_json(notification(
            "hook/started",
            0,
            "running",
            "turn-identity",
            None,
            None,
        ))
        .unwrap();
    let record = reader
        .read(&mut CodexNativeCursor::default())
        .unwrap()
        .remove(0);
    let normalizer = hookstat::runtime::codex::CodexNativeNormalizer::default();
    let canonical = normalizer.normalize(&record).unwrap();
    assert_eq!(canonical.source_coverage, SourceCoverage::IdentityLimited);
    assert_eq!(canonical.revision_ref, None);
    let identity = normalizer.identity_resolver().resolve(&record).unwrap();
    assert!(!identity.stable_handler_attribution_proven);
    assert_eq!(
        CodexNativeCapabilityProbe
            .probe(&CodexProtocolVersion::tested())
            .admission,
        NativeAdmissionState::Qualified
    );
}

#[test]
fn reader_cursor_is_adapter_internal_and_private_wire_content_does_not_persist() {
    let (mut integration, canonical) = normalized(vec![notification(
        "hook/started",
        0,
        "running",
        "turn-private",
        None,
        None,
    )]);
    let persisted = serde_json::to_string(&canonical[0]).unwrap();
    for private_value in [
        PRIVATE_SOURCE_PATH,
        PRIVATE_STATUS_TEXT,
        "thread-private-opaque-input",
        "turn-private",
        "session-start:0",
    ] {
        assert!(!persisted.contains(private_value));
    }

    let mut cursor = CodexNativeCursor::default();
    assert_eq!(integration.reader.read(&mut cursor).unwrap().len(), 1);
    assert_eq!(cursor.position(), 1);
    assert!(integration.reader.read(&mut cursor).unwrap().is_empty());

    let (integration, lifecycle) = normalized(vec![
        notification(
            "hook/started",
            0,
            "running",
            "turn-private-output",
            None,
            None,
        ),
        notification(
            "hook/completed",
            0,
            "failed",
            "turn-private-output",
            Some(1_700_000_001),
            Some(7),
        ),
    ]);
    let mut correlator = EvidenceCorrelator::default();
    let _ = produced(correlator.observe(lifecycle[0].clone()).unwrap());
    let completed = produced(correlator.observe(lifecycle[1].clone()).unwrap());
    let invocation = integration
        .normalizer
        .identity_resolver()
        .qualification_invocation(&completed)
        .unwrap();
    let persisted_invocation = serde_json::to_string(&invocation).unwrap();
    for private_value in [
        PRIVATE_SOURCE_PATH,
        PRIVATE_STATUS_TEXT,
        "thread-private-opaque-input",
        "turn-private-output",
        "session-start:0",
    ] {
        assert!(!persisted_invocation.contains(private_value));
    }
}

#[test]
fn codex_wire_types_do_not_cross_the_runtime_neutral_boundary() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let evidence = std::fs::read_to_string(manifest.join("src/evidence.rs")).unwrap();
    let native = std::fs::read_to_string(manifest.join("src/native.rs")).unwrap();
    for boundary in [evidence, native] {
        assert!(!boundary.contains("CodexWire"));
        assert!(!boundary.contains("sourcePath"));
        assert!(!boundary.contains("hook/started"));
    }
}
