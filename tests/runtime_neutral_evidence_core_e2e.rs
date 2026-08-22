use hookstat::analytics::{TimeWindow, aggregate};
use hookstat::domain::{
    EvidenceKind, ExecutionMode, HandlerIdentity, HookEvent, HookInvocation, Runtime,
    TerminalStatus,
};
use hookstat::evidence::{
    AuthorityRouter, CanonicalEvidence, CoreIngestOutcome, CoverageDomain, DomainAuthority,
    EventFamily, EvidenceTransport, InvocationCoverage, InvocationKey, NativeAdmissionState,
    RevisionRef, RuntimeHandlerRef, RuntimeId, RuntimeInstance, RuntimeNeutralEvidenceCore,
    SourceCoverage, SourceScope,
};
use hookstat::ledger::Ledger;
use sha2::{Digest, Sha256};

fn runtime(value: &str) -> RuntimeId {
    RuntimeId::new(value).unwrap()
}

fn instance(value: &str) -> RuntimeInstance {
    RuntimeInstance::new(value).unwrap()
}

fn invocation(value: &str) -> InvocationKey {
    InvocationKey::new(value).unwrap()
}

fn handler(value: &str) -> RuntimeHandlerRef {
    RuntimeHandlerRef::new(value).unwrap()
}

fn event(value: &str) -> EventFamily {
    EventFamily::new(value).unwrap()
}

fn scope(value: &str) -> SourceScope {
    SourceScope::new(value).unwrap()
}

fn domain(runtime_name: &str, event_name: &str, scope_name: &str) -> CoverageDomain {
    CoverageDomain {
        runtime: runtime(runtime_name),
        event: event(event_name),
        source_scope: scope(scope_name),
    }
}

fn core(rules: Vec<DomainAuthority>) -> RuntimeNeutralEvidenceCore {
    RuntimeNeutralEvidenceCore::new(AuthorityRouter::new(rules).unwrap())
}

fn started(
    runtime_name: &str,
    instance_name: &str,
    invocation_name: &str,
    event_name: &str,
    scope_name: &str,
    transport: EvidenceTransport,
) -> CanonicalEvidence {
    CanonicalEvidence {
        schema_version: 1,
        runtime: runtime(runtime_name),
        runtime_instance: instance(instance_name),
        invocation_key: invocation(invocation_name),
        runtime_handler_ref: handler("opaque_handler_a"),
        event: event(event_name),
        lifecycle: hookstat::evidence::EvidenceLifecycle::Started,
        occurred_at_unix_ms: 1_000,
        terminal_status: None,
        duration_ms: None,
        source_scope: scope(scope_name),
        revision_ref: Some(RevisionRef::new("revision_a").unwrap()),
        evidence_transport: transport,
        source_coverage: SourceCoverage::Complete,
        invocation_coverage: InvocationCoverage::Incomplete,
    }
}

fn completed(
    runtime_name: &str,
    instance_name: &str,
    invocation_name: &str,
    event_name: &str,
    scope_name: &str,
    transport: EvidenceTransport,
    terminal: TerminalStatus,
) -> CanonicalEvidence {
    let mut value = started(
        runtime_name,
        instance_name,
        invocation_name,
        event_name,
        scope_name,
        transport,
    );
    value.lifecycle = hookstat::evidence::EvidenceLifecycle::Completed;
    value.occurred_at_unix_ms = 1_007;
    value.terminal_status = Some(terminal);
    value.duration_ms = Some(7);
    value.invocation_coverage = InvocationCoverage::Complete;
    value
}

fn produced(value: CoreIngestOutcome) -> hookstat::evidence::CorrelatedEvidence {
    match value {
        CoreIngestOutcome::Produced(value) => value,
        unexpected => panic!("expected production record, got {unexpected:?}"),
    }
}

fn handler_identity() -> HandlerIdentity {
    HandlerIdentity {
        key: "resolved-handler-a".into(),
        revision: "resolved-revision-a".into(),
        label: "Resolved handler A".into(),
        source_kind: "synthetic_identity_resolver".into(),
        event: HookEvent::PreToolUse,
        matcher_identity: "opaque-matcher".into(),
        structural_identity: "synthetic:0".into(),
        execution_mode: ExecutionMode::Sync,
    }
}

/// This explicit adapter seam is outside the generic evidence core. It is the
/// only place that turns a resolved opaque handler reference into a ledger row.
fn resolved_invocation(evidence: &hookstat::evidence::CorrelatedEvidence) -> HookInvocation {
    HookInvocation {
        source_key: "synthetic-runtime-evidence".into(),
        source_record_id: ledger_source_record_id(&evidence.correlation_key),
        runtime: Runtime::Codex,
        evidence_kind: EvidenceKind::SyntheticFixture,
        coverage: evidence.legacy_coverage(),
        handler: handler_identity(),
        occurred_at_unix_ms: evidence.occurred_at_unix_ms,
        terminal_status: evidence.terminal_status,
        duration_ms: evidence.duration_ms,
        error_fingerprint: evidence.error_fingerprint().map(str::to_owned).or_else(|| {
            evidence
                .terminal_status
                .is_execution_failure()
                .then_some("synthetic_failure".into())
        }),
    }
}

/// The legacy ledger has a two-part receipt key, whereas runtime-neutral
/// correlation has three parts. A runtime adapter hashes all three opaque
/// components at the boundary so distinct runtime instances cannot collapse
/// into one ledger row or denominator sample.
fn ledger_source_record_id(correlation_key: &hookstat::evidence::CorrelationKey) -> String {
    let mut hasher = Sha256::new();
    for value in [
        correlation_key.runtime.as_str(),
        correlation_key.runtime_instance.as_str(),
        correlation_key.invocation_key.as_str(),
    ] {
        hasher.update(value.as_bytes());
        hasher.update([0]);
    }
    format!("synthetic-correlation-{:x}", hasher.finalize())
}

#[derive(Clone, Copy)]
enum SyntheticDurableLifecycle {
    Invoked,
    Result(TerminalStatus),
}

#[derive(Clone, Copy)]
struct SyntheticDurableRecord {
    cursor: u64,
    occurred_at_unix_ms: i64,
    lifecycle: SyntheticDurableLifecycle,
}

/// Synthetic Runtime B's adapter-side durable reader. The core receives only
/// its normalized canonical records; the cursor and durable record shape stay
/// outside the runtime-neutral boundary.
struct SyntheticDurableFixture {
    records: Vec<SyntheticDurableRecord>,
}

impl SyntheticDurableFixture {
    fn records_after(&self, cursor: u64) -> impl Iterator<Item = SyntheticDurableRecord> + '_ {
        self.records
            .iter()
            .copied()
            .filter(move |record| record.cursor > cursor)
    }

    fn normalize(record: SyntheticDurableRecord) -> CanonicalEvidence {
        let mut evidence = started(
            "synthetic_runtime_b",
            "durable_store",
            "persisted_invocation",
            "durable_event",
            "durable_log",
            EvidenceTransport::Ipc,
        );
        evidence.source_coverage = SourceCoverage::Durable;
        evidence.occurred_at_unix_ms = record.occurred_at_unix_ms;
        match record.lifecycle {
            SyntheticDurableLifecycle::Invoked => evidence,
            SyntheticDurableLifecycle::Result(terminal_status) => {
                evidence.lifecycle = hookstat::evidence::EvidenceLifecycle::Completed;
                evidence.terminal_status = Some(terminal_status);
                evidence.duration_ms = Some(7);
                evidence.invocation_coverage = InvocationCoverage::Complete;
                evidence
            }
        }
    }

    fn replay(
        &self,
        cursor: &mut u64,
        evidence_core: &mut RuntimeNeutralEvidenceCore,
        ledger: &mut Ledger,
    ) {
        for record in self.records_after(*cursor) {
            match evidence_core.ingest(Self::normalize(record)).unwrap() {
                CoreIngestOutcome::Produced(correlated) => {
                    ledger.ingest(&[resolved_invocation(&correlated)]).unwrap();
                }
                CoreIngestOutcome::Duplicate => {}
                outcome => panic!("durable fixture unexpectedly routed as {outcome:?}"),
            }
            *cursor = record.cursor;
        }
    }
}

#[test]
fn synthetic_runtime_a_live_lifecycle_is_ordered_and_idempotent() {
    let runtime_name = "synthetic_runtime_a";
    let event_name = "tool";
    let scope_name = "handler";
    let mut evidence_core = core(vec![DomainAuthority {
        domain: domain(runtime_name, event_name, scope_name),
        native_admission: NativeAdmissionState::Admitted,
    }]);
    let start = started(
        runtime_name,
        "live_instance",
        "invocation_a",
        event_name,
        scope_name,
        EvidenceTransport::Native,
    );
    let completion = completed(
        runtime_name,
        "live_instance",
        "invocation_a",
        event_name,
        scope_name,
        EvidenceTransport::Native,
        TerminalStatus::Completed,
    );
    let incomplete = produced(evidence_core.ingest(start.clone()).unwrap());
    assert_eq!(incomplete.terminal_status, TerminalStatus::Incomplete);
    assert_eq!(
        incomplete.invocation_coverage,
        InvocationCoverage::Incomplete
    );
    let complete = produced(evidence_core.ingest(completion.clone()).unwrap());
    assert_eq!(complete.terminal_status, TerminalStatus::Completed);
    assert_eq!(complete.invocation_coverage, InvocationCoverage::Complete);
    assert_eq!(
        evidence_core.ingest(start).unwrap(),
        CoreIngestOutcome::Duplicate
    );
    assert_eq!(
        evidence_core.ingest(completion).unwrap(),
        CoreIngestOutcome::Duplicate
    );
}

#[test]
fn completion_before_start_upgrades_best_effort_without_fabricating_success() {
    let runtime_name = "synthetic_runtime_a";
    let event_name = "tool";
    let scope_name = "handler";
    let mut evidence_core = core(vec![DomainAuthority {
        domain: domain(runtime_name, event_name, scope_name),
        native_admission: NativeAdmissionState::Admitted,
    }]);
    let best_effort = produced(
        evidence_core
            .ingest(completed(
                runtime_name,
                "live_instance",
                "invocation_a",
                event_name,
                scope_name,
                EvidenceTransport::Native,
                TerminalStatus::Failed,
            ))
            .unwrap(),
    );
    assert_eq!(
        best_effort.invocation_coverage,
        InvocationCoverage::BestEffort
    );
    let completed = produced(
        evidence_core
            .ingest(started(
                runtime_name,
                "live_instance",
                "invocation_a",
                event_name,
                scope_name,
                EvidenceTransport::Native,
            ))
            .unwrap(),
    );
    assert_eq!(completed.terminal_status, TerminalStatus::Failed);
    assert_eq!(completed.invocation_coverage, InvocationCoverage::Complete);
}

#[test]
fn distinct_runtime_instances_with_one_invocation_key_remain_distinct_in_the_ledger() {
    let runtime_name = "synthetic_runtime_a";
    let event_name = "tool";
    let scope_name = "handler";
    let mut evidence_core = core(vec![DomainAuthority {
        domain: domain(runtime_name, event_name, scope_name),
        native_admission: NativeAdmissionState::Admitted,
    }]);
    let mut ledger = Ledger::open_in_memory().unwrap();
    for instance_name in ["instance_one", "instance_two"] {
        let evidence = produced(
            evidence_core
                .ingest(completed(
                    runtime_name,
                    instance_name,
                    "same_invocation_key",
                    event_name,
                    scope_name,
                    EvidenceTransport::Native,
                    TerminalStatus::Completed,
                ))
                .unwrap(),
        );
        ledger.ingest(&[resolved_invocation(&evidence)]).unwrap();
    }
    let rows = ledger.invocations().unwrap();
    assert_eq!(rows.len(), 2);
    assert_ne!(rows[0].source_record_id, rows[1].source_record_id);
    let aggregate = aggregate(&rows, 2_000, TimeWindow::Last24Hours);
    assert_eq!(aggregate[0].runs, 2);
    assert_eq!(aggregate[0].failure_sample_count, 2);
}

#[test]
fn start_only_and_complete_only_preserve_truthful_missing_evidence() {
    let runtime_name = "synthetic_runtime_a";
    let event_name = "tool";
    let scope_name = "handler";
    let mut evidence_core = core(vec![DomainAuthority {
        domain: domain(runtime_name, event_name, scope_name),
        native_admission: NativeAdmissionState::Admitted,
    }]);
    let incomplete = produced(
        evidence_core
            .ingest(started(
                runtime_name,
                "one",
                "start_only",
                event_name,
                scope_name,
                EvidenceTransport::Native,
            ))
            .unwrap(),
    );
    assert_eq!(incomplete.terminal_status, TerminalStatus::Incomplete);
    assert_eq!(
        incomplete.legacy_coverage(),
        hookstat::domain::EvidenceCoverage::Unknown
    );
    let completion_only = produced(
        evidence_core
            .ingest(completed(
                runtime_name,
                "two",
                "complete_only",
                event_name,
                scope_name,
                EvidenceTransport::Native,
                TerminalStatus::Stopped,
            ))
            .unwrap(),
    );
    assert_eq!(
        completion_only.invocation_coverage,
        InvocationCoverage::BestEffort
    );
    assert_eq!(completion_only.terminal_status, TerminalStatus::Stopped);
}

#[test]
fn conflicting_terminal_duplicates_become_unknown_not_completed() {
    let runtime_name = "synthetic_runtime_a";
    let event_name = "tool";
    let scope_name = "handler";
    let mut evidence_core = core(vec![DomainAuthority {
        domain: domain(runtime_name, event_name, scope_name),
        native_admission: NativeAdmissionState::Admitted,
    }]);
    let first = completed(
        runtime_name,
        "live_instance",
        "invocation_a",
        event_name,
        scope_name,
        EvidenceTransport::Native,
        TerminalStatus::Completed,
    );
    let mut conflicting = first.clone();
    conflicting.terminal_status = Some(TerminalStatus::Failed);
    let _ = produced(evidence_core.ingest(first).unwrap());
    let conflict = produced(evidence_core.ingest(conflicting).unwrap());
    assert_eq!(conflict.terminal_status, TerminalStatus::Unknown);
    assert_eq!(conflict.invocation_coverage, InvocationCoverage::Unknown);
    assert_eq!(
        conflict.error_fingerprint(),
        Some(hookstat::evidence::CORRELATION_CONFLICT_FINGERPRINT)
    );
}

#[test]
fn synthetic_runtime_b_durable_replay_is_idempotent() {
    let authorities = vec![DomainAuthority {
        domain: domain("synthetic_runtime_b", "durable_event", "durable_log"),
        native_admission: NativeAdmissionState::Unavailable,
    }];
    let fixture = SyntheticDurableFixture {
        records: vec![
            SyntheticDurableRecord {
                cursor: 7,
                occurred_at_unix_ms: 1_000,
                lifecycle: SyntheticDurableLifecycle::Invoked,
            },
            SyntheticDurableRecord {
                cursor: 8,
                occurred_at_unix_ms: 1_007,
                lifecycle: SyntheticDurableLifecycle::Result(TerminalStatus::Failed),
            },
        ],
    };
    let mut ledger = Ledger::open_in_memory().unwrap();
    let mut first_cursor = 0;
    fixture.replay(
        &mut first_cursor,
        &mut core(authorities.clone()),
        &mut ledger,
    );
    assert_eq!(first_cursor, 8);
    let initial = ledger.invocations().unwrap();
    assert_eq!(initial.len(), 1);
    assert_eq!(initial[0].terminal_status, TerminalStatus::Failed);
    let initial_aggregate = aggregate(&initial, 2_000, TimeWindow::Last24Hours);
    assert_eq!(initial_aggregate[0].failure_sample_count, 1);
    assert_eq!(initial_aggregate[0].failed_runs, 1);

    // A saved cursor consumes nothing on a repeated pass. A fresh process
    // replaying the same durable records into the existing ledger also leaves
    // one row and one denominator sample through normal ledger idempotence.
    fixture.replay(
        &mut first_cursor,
        &mut core(authorities.clone()),
        &mut ledger,
    );
    let mut replay_cursor = 0;
    fixture.replay(&mut replay_cursor, &mut core(authorities), &mut ledger);
    assert_eq!(replay_cursor, 8);
    let replayed = ledger.invocations().unwrap();
    assert_eq!(replayed.len(), 1);
    let aggregate = aggregate(&replayed, 2_000, TimeWindow::Last24Hours);
    assert_eq!(aggregate[0].failure_sample_count, 1);
    assert_eq!(aggregate[0].failed_runs, 1);
}

#[test]
fn native_shadow_and_ipc_authority_do_not_double_count() {
    let runtime_name = "synthetic_runtime_c";
    let event_name = "fallback_event";
    let scope_name = "handler";
    let mut evidence_core = core(vec![DomainAuthority {
        domain: domain(runtime_name, event_name, scope_name),
        native_admission: NativeAdmissionState::Qualified,
    }]);
    assert_eq!(
        evidence_core
            .ingest(completed(
                runtime_name,
                "instance_c",
                "invocation_c",
                event_name,
                scope_name,
                EvidenceTransport::Native,
                TerminalStatus::Completed,
            ))
            .unwrap(),
        CoreIngestOutcome::Shadow
    );
    let completion = produced(
        evidence_core
            .ingest(completed(
                runtime_name,
                "instance_c",
                "invocation_c",
                event_name,
                scope_name,
                EvidenceTransport::Ipc,
                TerminalStatus::Completed,
            ))
            .unwrap(),
    );
    assert_eq!(completion.evidence_transport, EvidenceTransport::Ipc);
}

#[test]
fn native_authority_and_ipc_shadow_do_not_double_count() {
    let runtime_name = "synthetic_runtime_c";
    let event_name = "native_event";
    let scope_name = "handler";
    let mut evidence_core = core(vec![DomainAuthority {
        domain: domain(runtime_name, event_name, scope_name),
        native_admission: NativeAdmissionState::Admitted,
    }]);
    assert_eq!(
        evidence_core
            .ingest(completed(
                runtime_name,
                "instance_c",
                "invocation_c",
                event_name,
                scope_name,
                EvidenceTransport::Ipc,
                TerminalStatus::Completed,
            ))
            .unwrap(),
        CoreIngestOutcome::Shadow
    );
    let completion = produced(
        evidence_core
            .ingest(completed(
                runtime_name,
                "instance_c",
                "invocation_c",
                event_name,
                scope_name,
                EvidenceTransport::Native,
                TerminalStatus::Completed,
            ))
            .unwrap(),
    );
    assert_eq!(completion.evidence_transport, EvidenceTransport::Native);
}

#[test]
fn synthetic_runtime_c_routes_mixed_domains_independently() {
    let runtime_name = "synthetic_runtime_c";
    let scope_name = "handler";
    let mut evidence_core = core(vec![
        DomainAuthority {
            domain: domain(runtime_name, "native_event", scope_name),
            native_admission: NativeAdmissionState::Admitted,
        },
        DomainAuthority {
            domain: domain(runtime_name, "fallback_event", scope_name),
            native_admission: NativeAdmissionState::Degraded,
        },
    ]);
    let mut partial_native_start = started(
        runtime_name,
        "instance_c",
        "native_1",
        "native_event",
        scope_name,
        EvidenceTransport::Native,
    );
    partial_native_start.source_coverage = SourceCoverage::Partial;
    let _ = produced(evidence_core.ingest(partial_native_start).unwrap());
    let mut partial_native = completed(
        runtime_name,
        "instance_c",
        "native_1",
        "native_event",
        scope_name,
        EvidenceTransport::Native,
        TerminalStatus::Completed,
    );
    partial_native.source_coverage = SourceCoverage::Partial;
    let native = produced(evidence_core.ingest(partial_native).unwrap());
    let ipc = produced(
        evidence_core
            .ingest(completed(
                runtime_name,
                "instance_c",
                "fallback_1",
                "fallback_event",
                scope_name,
                EvidenceTransport::Ipc,
                TerminalStatus::Completed,
            ))
            .unwrap(),
    );
    assert_eq!(native.evidence_transport, EvidenceTransport::Native);
    assert_eq!(
        native.legacy_coverage(),
        hookstat::domain::EvidenceCoverage::Partial
    );
    assert_eq!(ipc.evidence_transport, EvidenceTransport::Ipc);
}

#[test]
fn only_admitted_native_is_authority_and_unconfigured_is_rejected() {
    for state in [
        NativeAdmissionState::Unavailable,
        NativeAdmissionState::Discovered,
        NativeAdmissionState::Qualified,
        NativeAdmissionState::Degraded,
        NativeAdmissionState::Revoked,
    ] {
        assert_eq!(
            DomainAuthority {
                domain: domain("runtime", "event", "scope"),
                native_admission: state,
            }
            .production_transport(),
            EvidenceTransport::Ipc
        );
    }
    let duplicate = domain("runtime", "event", "scope");
    assert!(
        AuthorityRouter::new(vec![
            DomainAuthority {
                domain: duplicate.clone(),
                native_admission: NativeAdmissionState::Admitted,
            },
            DomainAuthority {
                domain: duplicate,
                native_admission: NativeAdmissionState::Unavailable,
            },
        ])
        .is_err()
    );
    let mut evidence_core = core(Vec::new());
    assert_eq!(
        evidence_core
            .ingest(started(
                "runtime",
                "instance",
                "invocation",
                "event",
                "scope",
                EvidenceTransport::Ipc,
            ))
            .unwrap(),
        CoreIngestOutcome::Unconfigured
    );
}

#[test]
fn identity_is_resolved_after_correlation_and_denominator_semantics_stay_stable() {
    let runtime_name = "synthetic_runtime_a";
    let event_name = "tool";
    let scope_name = "handler";
    let mut evidence_core = core(vec![DomainAuthority {
        domain: domain(runtime_name, event_name, scope_name),
        native_admission: NativeAdmissionState::Admitted,
    }]);
    let start = produced(
        evidence_core
            .ingest(started(
                runtime_name,
                "instance",
                "failure_one",
                event_name,
                scope_name,
                EvidenceTransport::Native,
            ))
            .unwrap(),
    );
    assert_eq!(start.runtime_handler_ref.as_str(), "opaque_handler_a");
    let mut ledger = Ledger::open_in_memory().unwrap();
    ledger.ingest(&[resolved_invocation(&start)]).unwrap();
    let complete = produced(
        evidence_core
            .ingest(completed(
                runtime_name,
                "instance",
                "failure_one",
                event_name,
                scope_name,
                EvidenceTransport::Native,
                TerminalStatus::Failed,
            ))
            .unwrap(),
    );
    let receipt = ledger.ingest(&[resolved_invocation(&complete)]).unwrap();
    assert_eq!(receipt.upgraded, 1);
    let values = ledger.invocations().unwrap();
    let aggregate = aggregate(&values, 2_000, TimeWindow::Last24Hours);
    assert_eq!(aggregate[0].runs, 1);
    assert_eq!(aggregate[0].failure_sample_count, 1);
    assert_eq!(aggregate[0].failed_runs, 1);
    assert_eq!(aggregate[0].failure_rate_percent, 100.0);
}

#[test]
fn completion_before_start_corrects_legacy_occurrence_time_without_migration() {
    let runtime_name = "synthetic_runtime_a";
    let event_name = "tool";
    let scope_name = "handler";
    let mut evidence_core = core(vec![DomainAuthority {
        domain: domain(runtime_name, event_name, scope_name),
        native_admission: NativeAdmissionState::Admitted,
    }]);
    let completion = produced(
        evidence_core
            .ingest(completed(
                runtime_name,
                "instance",
                "out_of_order",
                event_name,
                scope_name,
                EvidenceTransport::Native,
                TerminalStatus::Failed,
            ))
            .unwrap(),
    );
    let mut ledger = Ledger::open_in_memory().unwrap();
    ledger.ingest(&[resolved_invocation(&completion)]).unwrap();
    let correlated = produced(
        evidence_core
            .ingest(started(
                runtime_name,
                "instance",
                "out_of_order",
                event_name,
                scope_name,
                EvidenceTransport::Native,
            ))
            .unwrap(),
    );
    let receipt = ledger.ingest(&[resolved_invocation(&correlated)]).unwrap();
    assert_eq!(receipt.upgraded, 1);
    let persisted = ledger.invocations().unwrap();
    assert_eq!(persisted[0].occurred_at_unix_ms, 1_000);
    assert_eq!(
        persisted[0].coverage,
        hookstat::domain::EvidenceCoverage::Complete
    );
}

#[test]
fn persisted_terminal_conflict_is_conservatively_removed_from_denominator() {
    let runtime_name = "synthetic_runtime_a";
    let event_name = "tool";
    let scope_name = "handler";
    let mut evidence_core = core(vec![DomainAuthority {
        domain: domain(runtime_name, event_name, scope_name),
        native_admission: NativeAdmissionState::Admitted,
    }]);
    let mut ledger = Ledger::open_in_memory().unwrap();
    let start = produced(
        evidence_core
            .ingest(started(
                runtime_name,
                "instance",
                "conflict",
                event_name,
                scope_name,
                EvidenceTransport::Native,
            ))
            .unwrap(),
    );
    ledger.ingest(&[resolved_invocation(&start)]).unwrap();
    let initial = produced(
        evidence_core
            .ingest(completed(
                runtime_name,
                "instance",
                "conflict",
                event_name,
                scope_name,
                EvidenceTransport::Native,
                TerminalStatus::Failed,
            ))
            .unwrap(),
    );
    ledger.ingest(&[resolved_invocation(&initial)]).unwrap();
    let conflicting = completed(
        runtime_name,
        "instance",
        "conflict",
        event_name,
        scope_name,
        EvidenceTransport::Native,
        TerminalStatus::Completed,
    );
    let conflict = produced(evidence_core.ingest(conflicting).unwrap());
    let receipt = ledger.ingest(&[resolved_invocation(&conflict)]).unwrap();
    assert_eq!(receipt.upgraded, 1);
    let values = ledger.invocations().unwrap();
    assert_eq!(values[0].terminal_status, TerminalStatus::Unknown);
    let aggregate = aggregate(&values, 2_000, TimeWindow::Last24Hours);
    assert_eq!(aggregate[0].failure_sample_count, 0);
    assert_eq!(aggregate[0].failed_runs, 0);
}

#[test]
fn canonical_evidence_is_bounded_and_serializes_no_private_content_fields() {
    let value = started(
        "synthetic_runtime_a",
        "instance_a",
        "invocation_a",
        "event_a",
        "scope_a",
        EvidenceTransport::Native,
    );
    value.validate().unwrap();
    let document = serde_json::to_value(value).unwrap();
    assert_no_private_content_keys(&document);
    assert!(RuntimeHandlerRef::new("unsafe raw handler reference").is_err());
}

#[test]
fn deserialized_opaque_references_are_revalidated_at_core_ingress() {
    let value = started(
        "synthetic_runtime_a",
        "instance_a",
        "invocation_a",
        "event_a",
        "scope_a",
        EvidenceTransport::Native,
    );
    let mut document = serde_json::to_value(value).unwrap();
    document["runtime_handler_ref"] = serde_json::Value::String("unsafe raw value".into());
    let deserialized: CanonicalEvidence = serde_json::from_value(document).unwrap();
    assert!(deserialized.validate().is_err());
    let mut evidence_core = core(vec![DomainAuthority {
        domain: domain("synthetic_runtime_a", "event_a", "scope_a"),
        native_admission: NativeAdmissionState::Admitted,
    }]);
    assert!(evidence_core.ingest(deserialized).is_err());
}

fn assert_no_private_content_keys(value: &serde_json::Value) {
    match value {
        serde_json::Value::Object(entries) => {
            for (key, value) in entries {
                assert!(
                    !matches!(
                        key.as_str(),
                        "prompt"
                            | "assistant_content"
                            | "tool_input"
                            | "tool_output"
                            | "stdin"
                            | "stdout"
                            | "stderr"
                            | "raw_command"
                            | "credential"
                    ),
                    "canonical evidence unexpectedly serializes private field {key}"
                );
                assert_no_private_content_keys(value);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                assert_no_private_content_keys(value);
            }
        }
        _ => {}
    }
}
