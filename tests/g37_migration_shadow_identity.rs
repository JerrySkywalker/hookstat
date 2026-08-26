use hookstat::admission::IpcAdmissionState;
use hookstat::analytics::TimeWindow;
use hookstat::domain::{
    EvidenceCoverage, EvidenceGeneration, EvidenceKind, ExecutionMode, HandlerIdentity, HookEvent,
    HookInvocation, Runtime, TerminalStatus,
};
use hookstat::evidence::{
    AuthorityRouter, CanonicalEvidence, CoreIngestOutcome, CorrelationKey, CoverageDomain,
    DomainAuthority, DurationSemantics, EventFamily, EvidenceLifecycle, EvidenceTransport,
    InvocationCoverage, InvocationKey, NativeAdmissionState, RevisionRef, RuntimeHandlerRef,
    RuntimeId, RuntimeInstance, RuntimeNeutralEvidenceCore, ShadowComparisonStatus, ShadowMismatch,
    ShadowObservation, ShadowPromotionDecision, ShadowPromotionGate, SourceCoverage, SourceScope,
};
use hookstat::identity::{
    DisplayIdentitySource, DisplayName, HandlerOwnershipProvenance, ObservationIntegration,
    resolve_display_identity, resolve_display_identity_with_stable_key,
};
use hookstat::ledger::Ledger;
use rusqlite::{Connection, params};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn create_v03_ledger(path: &std::path::Path, malformed: bool) {
    let connection = Connection::open(path).unwrap();
    connection
        .execute_batch(
            "
            CREATE TABLE hookstat_schema (version INTEGER PRIMARY KEY);
            INSERT INTO hookstat_schema (version) VALUES (3);
            CREATE TABLE hook_invocations (
                source_key TEXT NOT NULL, source_record_id TEXT NOT NULL,
                runtime TEXT NOT NULL, evidence_kind TEXT NOT NULL, coverage TEXT NOT NULL,
                handler_key TEXT NOT NULL, handler_revision TEXT NOT NULL,
                handler_label TEXT NOT NULL, handler_source_kind TEXT NOT NULL,
                handler_event TEXT NOT NULL, handler_matcher_identity TEXT NOT NULL,
                handler_structural_identity TEXT NOT NULL, handler_execution_mode TEXT NOT NULL,
                occurred_at_unix_ms INTEGER NOT NULL, terminal_status TEXT NOT NULL,
                duration_ms INTEGER, error_fingerprint TEXT,
                PRIMARY KEY (source_key, source_record_id)
            );
            CREATE TABLE handler_annotations (
                runtime TEXT NOT NULL, handler_key TEXT NOT NULL,
                display_name TEXT NOT NULL, updated_at_unix_ms INTEGER NOT NULL,
                PRIMARY KEY (runtime, handler_key)
            );
            INSERT INTO handler_annotations VALUES
                ('codex', 'hk_tabbeacon', 'TabBeacon', 1000);
            ",
        )
        .unwrap();

    for (id, revision, status, timestamp) in [
        ("completed", "legacy-r1", "completed", 1_000_i64),
        ("failed", "legacy-r1", "failed", 2_000),
        ("incomplete", "legacy-r2", "incomplete", 3_000),
    ] {
        connection
            .execute(
                "INSERT INTO hook_invocations VALUES
                 (?1, ?2, 'codex', 'codex_instrumented_receipt', 'partial',
                  'hk_tabbeacon', ?3, 'TabBeacon', 'user_hooks', 'stop',
                  'any', 'g0:h0', 'sync', ?4, ?5, NULL, NULL)",
                params![
                    "codex_instrumented_receipts_v1",
                    id,
                    revision,
                    timestamp,
                    status
                ],
            )
            .unwrap();
    }
    if malformed {
        connection
            .execute(
                "INSERT INTO hook_invocations VALUES
                 ('legacy', 'malformed', 'codex', 'codex_instrumented_receipt', 'partial',
                  'hk_malformed', 'legacy-r1', 'Malformed fixture', 'user_hooks', 'stop',
                  'any', 'g0:h1', 'sync', 4000, 'not_a_terminal_status', NULL, NULL)",
                [],
            )
            .unwrap();
    }
}

fn create_clean_v03_ledger(path: &std::path::Path) {
    create_v03_ledger(path, false);
    let connection = Connection::open(path).unwrap();
    connection
        .execute("DELETE FROM hook_invocations", [])
        .unwrap();
    connection
        .execute("DELETE FROM handler_annotations", [])
        .unwrap();
}

fn v031_invocation(id: &str, generation: EvidenceGeneration) -> HookInvocation {
    HookInvocation {
        source_key: "v031_controlled".into(),
        source_record_id: id.into(),
        runtime: Runtime::Codex,
        evidence_kind: match generation {
            EvidenceGeneration::V031Native => EvidenceKind::CodexAppServerLive,
            EvidenceGeneration::V031CooperativeIpc => EvidenceKind::RuntimeNeutralIpc,
            _ => EvidenceKind::SyntheticFixture,
        },
        evidence_generation: generation,
        coverage: EvidenceCoverage::Complete,
        handler: HandlerIdentity {
            key: "hk_tabbeacon".into(),
            revision: "v031-r1".into(),
            label: "TabBeacon".into(),
            source_kind: "cooperative_integration".into(),
            event: HookEvent::Stop,
            matcher_identity: "any".into(),
            structural_identity: "g0:h0".into(),
            execution_mode: ExecutionMode::Sync,
        },
        occurred_at_unix_ms: 5_000,
        terminal_status: TerminalStatus::Completed,
        duration_ms: Some(12),
        error_fingerprint: None,
    }
}

#[test]
fn v03_fixtures_migrate_additively_and_reopen_idempotently() {
    let temporary = tempdir().unwrap();
    let read_only_path = temporary.path().join("read-only.sqlite3");
    create_v03_ledger(&read_only_path, false);
    let read_only = Ledger::open_read_only(&read_only_path).unwrap();
    assert!(
        read_only
            .invocations()
            .unwrap()
            .iter()
            .all(|value| value.evidence_generation == EvidenceGeneration::LegacyV03Proxy)
    );
    drop(read_only);
    assert!(
        !Connection::open(&read_only_path)
            .unwrap()
            .prepare("SELECT evidence_generation FROM hook_invocations")
            .is_ok()
    );

    let empty_path = temporary.path().join("empty.sqlite3");
    create_clean_v03_ledger(&empty_path);
    let empty = Ledger::open_path(&empty_path).unwrap();
    assert_eq!(empty.invocation_count().unwrap(), 0);
    assert!(empty.handler_aliases().unwrap().is_empty());

    let clean_path = temporary.path().join("clean.sqlite3");
    create_v03_ledger(&clean_path, false);

    let mut ledger = Ledger::open_path(&clean_path).unwrap();
    let legacy = ledger.invocations().unwrap();
    assert_eq!(legacy.len(), 3);
    assert_eq!(
        legacy
            .iter()
            .map(|value| value.terminal_status)
            .collect::<Vec<_>>(),
        vec![
            TerminalStatus::Completed,
            TerminalStatus::Failed,
            TerminalStatus::Incomplete
        ]
    );
    assert!(
        legacy
            .iter()
            .all(|value| value.evidence_generation == EvidenceGeneration::LegacyV03Proxy)
    );
    assert_eq!(legacy[0].handler.revision, "legacy-r1");
    assert_eq!(legacy[2].handler.revision, "legacy-r2");
    let epochs = ledger
        .revision_epoch_metrics(&["hk_tabbeacon".into()])
        .unwrap();
    assert_eq!(epochs["hk_tabbeacon"].current.revision, "legacy-r2");
    assert_eq!(
        epochs["hk_tabbeacon"].previous.as_ref().unwrap().revision,
        "legacy-r1"
    );
    assert_eq!(
        ledger.handler_aliases().unwrap()[0].display_name,
        "TabBeacon"
    );

    ledger
        .ingest(&[
            v031_invocation("native", EvidenceGeneration::V031Native),
            v031_invocation("ipc", EvidenceGeneration::V031CooperativeIpc),
        ])
        .unwrap();
    drop(ledger);

    let reopened = Ledger::open_path(&clean_path).unwrap();
    let mixed = reopened.invocations().unwrap();
    assert_eq!(mixed.len(), 5);
    assert_eq!(
        mixed
            .iter()
            .filter(|value| value.evidence_generation == EvidenceGeneration::LegacyV03Proxy)
            .count(),
        3
    );
    assert!(
        mixed
            .iter()
            .any(|value| value.evidence_generation == EvidenceGeneration::V031Native)
    );
    assert!(
        mixed
            .iter()
            .any(|value| { value.evidence_generation == EvidenceGeneration::V031CooperativeIpc })
    );
    assert_eq!(
        reopened.handler_aliases().unwrap()[0].display_name,
        "TabBeacon"
    );

    let mut legacy_json = serde_json::to_value(v031_invocation(
        "old-serialized",
        EvidenceGeneration::V031Native,
    ))
    .unwrap();
    legacy_json
        .as_object_mut()
        .unwrap()
        .remove("evidence_generation");
    let deserialized: HookInvocation = serde_json::from_value(legacy_json).unwrap();
    assert_eq!(
        deserialized.evidence_generation,
        EvidenceGeneration::LegacyV03Proxy
    );
    drop(reopened);

    let connection = Connection::open(&clean_path).unwrap();
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM hookstat_schema WHERE version = 4",
                [],
                |row| { row.get::<_, i64>(0) }
            )
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM pragma_table_info('hook_invocations') WHERE name = 'evidence_generation'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        1
    );
}

#[test]
fn malformed_legacy_taxonomy_is_preserved_and_isolated() {
    let temporary = tempdir().unwrap();
    let path = temporary.path().join("malformed.sqlite3");
    create_v03_ledger(&path, true);
    let ledger = Ledger::open_path(&path).unwrap();
    assert_eq!(ledger.migration_issue_count().unwrap(), 1);
    let canonical = ledger.invocations().unwrap();
    assert_eq!(canonical.len(), 3);
    assert!(
        canonical
            .iter()
            .all(|value| value.handler.key == "hk_tabbeacon")
    );
    assert_eq!(ledger.invocation_count().unwrap(), 3);
    let reliability = ledger
        .invocations_for_reliability(10_000, TimeWindow::All)
        .unwrap();
    assert_eq!(reliability.rows_materialized, 3);
    assert_eq!(reliability.invocations, canonical);
    let all_time = ledger.all_time_period_metrics(10_000).unwrap();
    assert_eq!(all_time.len(), 1);
    assert_eq!(all_time["hk_tabbeacon"].runs, 3);
    let revision_metrics = ledger
        .revision_epoch_metrics(&["hk_tabbeacon".into(), "hk_malformed".into()])
        .unwrap();
    assert!(revision_metrics.contains_key("hk_tabbeacon"));
    assert!(!revision_metrics.contains_key("hk_malformed"));
    drop(ledger);

    let connection = Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM hook_invocations", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        4
    );
    let terminal: String = connection
        .query_row(
            "SELECT terminal_status FROM hook_invocations WHERE source_record_id = 'malformed'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(terminal, "not_a_terminal_status");
    let generation: String = connection
        .query_row(
            "SELECT evidence_generation FROM hook_invocations WHERE source_record_id = 'malformed'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(generation, "legacy_v03_proxy");
}

fn runtime_id() -> RuntimeId {
    RuntimeId::new("codex").unwrap()
}

fn domain() -> CoverageDomain {
    CoverageDomain {
        runtime: runtime_id(),
        event: EventFamily::new("stop").unwrap(),
        source_scope: SourceScope::new("user_hooks").unwrap(),
    }
}

fn correlation_key(id: &str) -> CorrelationKey {
    CorrelationKey {
        runtime: runtime_id(),
        runtime_instance: RuntimeInstance::new("controlled_instance").unwrap(),
        invocation_key: InvocationKey::new(id).unwrap(),
    }
}

fn observation(id: &str) -> ShadowObservation {
    ShadowObservation {
        domain: domain(),
        correlation_key: correlation_key(id),
        handler_ref: RuntimeHandlerRef::new("tabbeacon_stop").unwrap(),
        revision_ref: Some(RevisionRef::new("revision_1").unwrap()),
        terminal_status: TerminalStatus::Completed,
        duration_semantics: DurationSemantics::OriginalHandlerInterval,
        source_coverage: SourceCoverage::Complete,
        invocation_coverage: InvocationCoverage::Complete,
    }
}

#[test]
fn shadow_match_is_eligible_but_mismatch_or_absence_blocks_promotion() {
    let gate = ShadowPromotionGate;
    let authoritative = observation("one");
    let comparison = gate
        .compare(
            std::slice::from_ref(&authoritative),
            std::slice::from_ref(&authoritative),
        )
        .unwrap();
    assert_eq!(comparison.status, ShadowComparisonStatus::Match);
    assert_eq!(
        gate.promotion_decision(&comparison),
        ShadowPromotionDecision::Eligible
    );

    let mut cases = Vec::new();
    let mut different_handler = authoritative.clone();
    different_handler.handler_ref = RuntimeHandlerRef::new("different_handler").unwrap();
    cases.push(different_handler);
    let mut different_revision = authoritative.clone();
    different_revision.revision_ref = Some(RevisionRef::new("revision_2").unwrap());
    cases.push(different_revision);
    let mut different_outcome = authoritative.clone();
    different_outcome.terminal_status = TerminalStatus::Failed;
    cases.push(different_outcome);
    let mut different_duration = authoritative.clone();
    different_duration.duration_semantics = DurationSemantics::EndToEndInvocation;
    cases.push(different_duration);
    let mut different_coverage = authoritative.clone();
    different_coverage.source_coverage = SourceCoverage::Partial;
    cases.push(different_coverage);
    let mut different_domain = authoritative.clone();
    different_domain.domain.source_scope = SourceScope::new("project_hooks").unwrap();
    cases.push(different_domain);
    for candidate in cases {
        let comparison = gate
            .compare(std::slice::from_ref(&authoritative), &[candidate])
            .unwrap();
        assert_eq!(comparison.status, ShadowComparisonStatus::Mismatch);
        assert_eq!(
            gate.promotion_decision(&comparison),
            ShadowPromotionDecision::BlockedMismatch
        );
    }

    let production_only = gate
        .compare(std::slice::from_ref(&authoritative), &[])
        .unwrap();
    assert!(
        production_only
            .mismatches
            .contains(&ShadowMismatch::ProductionOnly(correlation_key("one")))
    );
    let candidate_only = gate
        .compare(&[], std::slice::from_ref(&authoritative))
        .unwrap();
    assert!(
        candidate_only
            .mismatches
            .contains(&ShadowMismatch::CandidateOnly(correlation_key("one")))
    );
    let insufficient = gate.compare(&[], &[]).unwrap();
    assert_eq!(
        gate.promotion_decision(&insufficient),
        ShadowPromotionDecision::BlockedInsufficientEvidence
    );

    let duplicate = gate
        .compare(
            &[authoritative.clone(), authoritative.clone()],
            std::slice::from_ref(&authoritative),
        )
        .unwrap();
    assert_eq!(duplicate.status, ShadowComparisonStatus::Mismatch);
}

fn canonical(transport: EvidenceTransport) -> CanonicalEvidence {
    CanonicalEvidence {
        schema_version: 1,
        runtime: runtime_id(),
        runtime_instance: RuntimeInstance::new("controlled_instance").unwrap(),
        invocation_key: InvocationKey::new("denominator_one").unwrap(),
        runtime_handler_ref: RuntimeHandlerRef::new("tabbeacon_stop").unwrap(),
        event: EventFamily::new("stop").unwrap(),
        lifecycle: EvidenceLifecycle::Completed,
        occurred_at_unix_ms: 1_000,
        terminal_status: Some(TerminalStatus::Completed),
        duration_ms: Some(10),
        source_scope: SourceScope::new("user_hooks").unwrap(),
        revision_ref: Some(RevisionRef::new("revision_1").unwrap()),
        evidence_transport: transport,
        source_coverage: SourceCoverage::Complete,
        invocation_coverage: InvocationCoverage::BestEffort,
    }
}

#[test]
fn shadow_route_has_zero_denominator_contribution() {
    let router = AuthorityRouter::new([DomainAuthority {
        domain: domain(),
        native_admission: NativeAdmissionState::Admitted,
        ipc_admission: IpcAdmissionState::Admitted,
    }])
    .unwrap();
    let mut core = RuntimeNeutralEvidenceCore::new(router);
    assert!(matches!(
        core.ingest(canonical(EvidenceTransport::Native)).unwrap(),
        CoreIngestOutcome::Produced(_)
    ));
    assert_eq!(
        core.ingest(canonical(EvidenceTransport::Ipc)).unwrap(),
        CoreIngestOutcome::Shadow
    );

    let ipc_router = AuthorityRouter::new([DomainAuthority {
        domain: domain(),
        native_admission: NativeAdmissionState::Unavailable,
        ipc_admission: IpcAdmissionState::Admitted,
    }])
    .unwrap();
    let mut ipc_core = RuntimeNeutralEvidenceCore::new(ipc_router);
    assert!(matches!(
        ipc_core.ingest(canonical(EvidenceTransport::Ipc)).unwrap(),
        CoreIngestOutcome::Produced(_)
    ));
    assert_eq!(
        ipc_core
            .ingest(canonical(EvidenceTransport::Native))
            .unwrap(),
        CoreIngestOutcome::Shadow
    );

    let not_admitted_router = AuthorityRouter::new([DomainAuthority {
        domain: domain(),
        native_admission: NativeAdmissionState::Unavailable,
        ipc_admission: IpcAdmissionState::QualifiedNotAdmittedPerformance,
    }])
    .unwrap();
    let mut not_admitted_core = RuntimeNeutralEvidenceCore::new(not_admitted_router);
    assert_eq!(
        not_admitted_core
            .ingest(canonical(EvidenceTransport::Ipc))
            .unwrap(),
        CoreIngestOutcome::NotAdmitted
    );
}

#[test]
fn cooperative_tabbeacon_provenance_and_human_identity_preserve_original_owner() {
    let provenance = HandlerOwnershipProvenance {
        original_handler_owner: "TabBeacon".into(),
        original_definition_identity: "tabbeacon:stop:1".into(),
        hookstat_observation_integration: ObservationIntegration::CooperativeIpc,
        effective_revision: "revision_1".into(),
    };
    provenance.validate().unwrap();
    let serialized = serde_json::to_string(&provenance).unwrap();
    assert!(!serialized.contains("command"));
    assert!(!serialized.contains("\\"));
    assert!(!serialized.contains("/"));

    let resolved = resolve_display_identity(
        None,
        Some("Hookstat Exe"),
        Some("TabBeacon Stop"),
        Some("hookstat-hook"),
        HookEvent::Stop,
        "cooperative_ipc",
    );
    assert_eq!(resolved.name, DisplayName::Literal("TabBeacon Stop".into()));
    assert_eq!(resolved.source, DisplayIdentitySource::ScriptFilename);

    let aliased = resolve_display_identity(
        Some("Owner TabBeacon Alias"),
        Some("Hookstat Exe"),
        Some("TabBeacon Stop"),
        None,
        HookEvent::Stop,
        "cooperative_ipc",
    );
    assert_eq!(
        aliased.name,
        DisplayName::Literal("Owner TabBeacon Alias".into())
    );

    let stable_fallback = resolve_display_identity_with_stable_key(
        None,
        Some("Hookstat Exe"),
        None,
        "tabbeacon_stop",
        HookEvent::Stop,
        "cooperative_ipc",
    );
    assert_eq!(
        stable_fallback.name,
        DisplayName::Literal("tabbeacon_stop".into())
    );
    assert_eq!(stable_fallback.source, DisplayIdentitySource::StableKey);

    let invalid = HandlerOwnershipProvenance {
        original_handler_owner: "TabBeacon".into(),
        original_definition_identity: "C:\\private\\handler".into(),
        hookstat_observation_integration: ObservationIntegration::CooperativeIpc,
        effective_revision: "revision_1".into(),
    };
    assert!(invalid.validate().is_err());
}

#[test]
fn disposable_legacy_restore_is_exact_drift_aware_and_never_bypasses_trust() {
    let temporary = tempdir().unwrap();
    let config = temporary.path().join("hooks.json");
    let original = br#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"fixture-handler","timeout":4}]}]}}"#;
    fs::write(&config, original).unwrap();
    let discovery = hookstat::codex::discover_paths(std::slice::from_ref(&config)).unwrap();
    let state = temporary.path().join("hookstat-state");
    let applied = hookstat::codex::apply(&discovery, &state, Path::new("hookstat-test")).unwrap();
    assert_eq!(applied.applied, 1);
    assert!(applied.trust_review_required);
    assert_ne!(fs::read(&config).unwrap(), original);
    assert_eq!(
        hookstat::codex::restore(&config, &state).unwrap().restored,
        1
    );
    assert_eq!(fs::read(&config).unwrap(), original);

    let discovery = hookstat::codex::discover_paths(std::slice::from_ref(&config)).unwrap();
    hookstat::codex::apply(&discovery, &state, Path::new("hookstat-test")).unwrap();
    fs::write(&config, b"{}\n").unwrap();
    assert_eq!(
        hookstat::codex::restore(&config, &state)
            .unwrap()
            .drift_detected,
        1
    );
}
