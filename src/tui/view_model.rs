//! Locale-neutral Reliability Center projections.
//!
//! This module is the one-way boundary between HookStat's existing analytics
//! report and terminal presentation. It deliberately owns no terminal types,
//! database handles, receipt paths, or raw runtime payloads.

use crate::analytics::{
    FailureFingerprintCluster, HandlerAggregate, HandlerIntelligence, RecentFailure,
    RevisionComparison, RiskScore, TerminalBreakdown, TimeWindow, TrendProjection,
};
pub use crate::diagnostics::{
    DiagnosticCheck as DiagnosticCheckViewModel, DiagnosticCheckId, DiagnosticFact,
    DiagnosticStatus, DiagnosticsReport as DiagnosticsViewModel,
};
use crate::domain::{EvidenceCoverage, HookEvent, Runtime, TerminalStatus};
use crate::report::MachineReport;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandlerRef {
    pub runtime: Runtime,
    pub handler_key: String,
}

impl HandlerRef {
    fn from_aggregate(runtime: Runtime, aggregate: &HandlerAggregate) -> Self {
        Self {
            runtime,
            handler_key: aggregate.handler.key.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisplayIdentity {
    ExistingMetadata(String),
    EventFallback(HookEvent),
}

impl DisplayIdentity {
    pub fn searchable_text(&self) -> String {
        match self {
            Self::ExistingMetadata(value) => value.to_ascii_lowercase(),
            Self::EventFallback(event) => event.as_storage().replace('_', " "),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Health {
    Healthy,
    Degraded,
    CoverageLimited,
    NoTerminalSamples,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeSummaryViewModel {
    pub runtime: Runtime,
    pub coverage: EvidenceCoverage,
    pub total_runs: u64,
    pub terminal_sample_count: u64,
    pub failed_runs: u64,
    pub failure_rate_percent: f64,
    pub health: Health,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HookRowViewModel {
    pub internal_ref: HandlerRef,
    pub display_identity: DisplayIdentity,
    pub display_disambiguator: Option<usize>,
    pub event: HookEvent,
    pub coverage: EvidenceCoverage,
    pub runs: u64,
    pub failed_runs: u64,
    pub sample_count: u64,
    pub failure_rate_percent: f64,
    pub trend: TrendProjection,
    pub risk: RiskScore,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OverviewViewModel {
    pub window: TimeWindow,
    pub runtime_summaries: Vec<RuntimeSummaryViewModel>,
    pub highest_risk_hooks: Vec<HookRowViewModel>,
    pub incomplete_receipts: u64,
    pub malformed_receipts: u64,
    pub generated_at_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HooksViewModel {
    pub window: TimeWindow,
    pub rows: Vec<HookRowViewModel>,
    pub total_before_filter: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecentFailureViewModel {
    pub occurred_at_unix_ms: i64,
    pub status: TerminalStatus,
    pub bounded_fingerprint: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HookDetailViewModel {
    pub internal_ref: HandlerRef,
    pub display_identity: DisplayIdentity,
    pub display_disambiguator: Option<usize>,
    pub revision: String,
    pub event: HookEvent,
    pub coverage: EvidenceCoverage,
    pub window: TimeWindow,
    pub runs: u64,
    pub failed_runs: u64,
    pub sample_count: u64,
    pub failure_rate_percent: f64,
    pub terminal_breakdown: TerminalBreakdown,
    pub recent_failures: Vec<RecentFailureViewModel>,
    pub trends: Vec<TrendProjection>,
    pub risk: RiskScore,
    pub failure_fingerprints: Vec<FailureFingerprintCluster>,
    pub revision_comparison: RevisionComparison,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HookSort {
    Risk,
    FailureRate,
    Name,
    Runs,
}

impl HookSort {
    pub const fn next(self) -> Self {
        match self {
            Self::Risk => Self::FailureRate,
            Self::FailureRate => Self::Name,
            Self::Name => Self::Runs,
            Self::Runs => Self::Risk,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HooksQuery {
    pub search: String,
    pub failures_only: bool,
    pub sort: HookSort,
}

impl Default for HooksQuery {
    fn default() -> Self {
        Self {
            search: String::new(),
            failures_only: false,
            sort: HookSort::Risk,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReliabilityCenterViewModel {
    pub overview: OverviewViewModel,
    pub hooks: HooksViewModel,
    pub diagnostics: DiagnosticsViewModel,
    details: Vec<HookDetailViewModel>,
}

impl ReliabilityCenterViewModel {
    pub fn from_report(report: MachineReport) -> Self {
        // v0.1 has an admitted Codex-only implementation. The projection does
        // not invent rows for other runtimes and keeps the runtime attached to
        // the stable UI reference for the later multi-runtime boundary.
        let runtime = Runtime::Codex;
        let coverage = report.qualification.coverage;
        let intelligence_by_handler = report
            .intelligence
            .iter()
            .map(|intelligence| (intelligence.handler_key.as_str(), intelligence))
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut rows = report
            .handlers
            .iter()
            .map(|aggregate| {
                hook_row(
                    runtime,
                    coverage,
                    aggregate,
                    intelligence_by_handler
                        .get(aggregate.handler.key.as_str())
                        .expect("report intelligence is keyed by every handler"),
                    report.window,
                )
            })
            .collect::<Vec<_>>();
        let mut details = report
            .handlers
            .iter()
            .map(|aggregate| {
                hook_detail(
                    runtime,
                    coverage,
                    report.window,
                    aggregate,
                    intelligence_by_handler
                        .get(aggregate.handler.key.as_str())
                        .expect("report intelligence is keyed by every handler"),
                    &report.recent_failures,
                )
            })
            .collect::<Vec<_>>();
        assign_fallback_disambiguators(&mut rows, &mut details);
        let total_runs = rows.iter().map(|row| row.runs).sum();
        let terminal_sample_count = rows.iter().map(|row| row.sample_count).sum();
        let failed_runs = rows.iter().map(|row| row.failed_runs).sum();
        let failure_rate_percent = percentage(failed_runs, terminal_sample_count);
        rows.sort_by(risk_order);
        let overview = OverviewViewModel {
            window: report.window,
            runtime_summaries: vec![RuntimeSummaryViewModel {
                runtime,
                coverage,
                total_runs,
                terminal_sample_count,
                failed_runs,
                failure_rate_percent,
                health: health(coverage, failed_runs, terminal_sample_count),
            }],
            highest_risk_hooks: rows.iter().take(5).cloned().collect(),
            incomplete_receipts: report.incomplete_receipts,
            malformed_receipts: report.malformed_receipts,
            generated_at_unix_ms: report.generated_at_unix_ms,
        };
        let diagnostics = diagnostics(
            runtime,
            coverage,
            overview.runtime_summaries[0].health,
            report.incomplete_receipts,
            report.malformed_receipts,
            report.generated_at_unix_ms,
        );
        Self {
            overview,
            hooks: HooksViewModel {
                window: report.window,
                total_before_filter: rows.len(),
                rows,
            },
            diagnostics,
            details,
        }
    }

    pub fn detail(&self, reference: &HandlerRef) -> Option<&HookDetailViewModel> {
        self.details
            .iter()
            .find(|detail| detail.internal_ref == *reference)
    }

    pub fn filtered_hooks(&self, query: &HooksQuery) -> Vec<HookRowViewModel> {
        let needle = query.search.trim().to_ascii_lowercase();
        let mut rows = self
            .hooks
            .rows
            .iter()
            .filter(|row| {
                (!query.failures_only || row.failed_runs > 0)
                    && (needle.is_empty()
                        || row.display_identity.searchable_text().contains(&needle)
                        || row.event.as_storage().contains(&needle)
                        || row.internal_ref.handler_key.contains(&needle))
            })
            .cloned()
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| match query.sort {
            HookSort::Risk => risk_order(left, right),
            HookSort::FailureRate => right
                .failure_rate_percent
                .total_cmp(&left.failure_rate_percent)
                .then_with(|| right.failed_runs.cmp(&left.failed_runs))
                .then_with(|| {
                    left.internal_ref
                        .handler_key
                        .cmp(&right.internal_ref.handler_key)
                }),
            HookSort::Name => left
                .display_identity
                .searchable_text()
                .cmp(&right.display_identity.searchable_text())
                .then_with(|| {
                    left.internal_ref
                        .handler_key
                        .cmp(&right.internal_ref.handler_key)
                }),
            HookSort::Runs => right
                .runs
                .cmp(&left.runs)
                .then_with(|| {
                    right
                        .failure_rate_percent
                        .total_cmp(&left.failure_rate_percent)
                })
                .then_with(|| {
                    left.internal_ref
                        .handler_key
                        .cmp(&right.internal_ref.handler_key)
                }),
        });
        rows
    }
}

fn diagnostics(
    runtime: Runtime,
    coverage: EvidenceCoverage,
    summary_health: Health,
    incomplete_receipts: u64,
    malformed_receipts: u64,
    refreshed_at_unix_ms: i64,
) -> DiagnosticsViewModel {
    let coverage_status = if coverage == EvidenceCoverage::Complete {
        DiagnosticStatus::Pass
    } else {
        DiagnosticStatus::Warning
    };
    let receipt_status = if incomplete_receipts == 0 && malformed_receipts == 0 {
        DiagnosticStatus::Pass
    } else {
        DiagnosticStatus::Warning
    };
    let overall_status = if matches!(summary_health, Health::Healthy)
        && coverage_status == DiagnosticStatus::Pass
        && receipt_status == DiagnosticStatus::Pass
    {
        DiagnosticStatus::Pass
    } else {
        DiagnosticStatus::Warning
    };
    DiagnosticsViewModel {
        schema_version: crate::diagnostics::DIAGNOSTICS_SCHEMA_VERSION,
        read_only: true,
        overall_status,
        checks: vec![
            DiagnosticCheckViewModel {
                id: DiagnosticCheckId::EffectiveRuntime,
                status: DiagnosticStatus::Pass,
                facts: vec![DiagnosticFact::Runtime { runtime }],
            },
            DiagnosticCheckViewModel {
                id: DiagnosticCheckId::EvidenceCoverage,
                status: coverage_status,
                facts: vec![DiagnosticFact::Coverage { coverage }],
            },
            DiagnosticCheckViewModel {
                id: DiagnosticCheckId::ReceiptIntegrity,
                status: receipt_status,
                facts: vec![DiagnosticFact::ReceiptIntegrity {
                    incomplete: incomplete_receipts,
                    malformed: malformed_receipts,
                }],
            },
            unavailable_diagnostic(DiagnosticCheckId::Instrumentation),
            unavailable_diagnostic(DiagnosticCheckId::Trust),
            unavailable_diagnostic(DiagnosticCheckId::ReceiptSpool),
        ],
        generated_at_unix_ms: refreshed_at_unix_ms,
    }
}

fn unavailable_diagnostic(id: DiagnosticCheckId) -> DiagnosticCheckViewModel {
    DiagnosticCheckViewModel {
        id,
        status: DiagnosticStatus::Unknown,
        facts: Vec::new(),
    }
}

fn hook_row(
    runtime: Runtime,
    coverage: EvidenceCoverage,
    aggregate: &HandlerAggregate,
    intelligence: &HandlerIntelligence,
    window: TimeWindow,
) -> HookRowViewModel {
    HookRowViewModel {
        internal_ref: HandlerRef::from_aggregate(runtime, aggregate),
        display_identity: resolve_display_identity(aggregate),
        display_disambiguator: None,
        event: aggregate.handler.event,
        coverage,
        runs: aggregate.runs,
        failed_runs: aggregate.failed_runs,
        sample_count: aggregate.failure_sample_count,
        failure_rate_percent: aggregate.failure_rate_percent,
        trend: intelligence
            .trends
            .iter()
            .find(|trend| trend.window == window)
            .expect("report intelligence includes the selected window")
            .clone(),
        risk: intelligence.risk.clone(),
    }
}

fn hook_detail(
    runtime: Runtime,
    coverage: EvidenceCoverage,
    window: TimeWindow,
    aggregate: &HandlerAggregate,
    intelligence: &HandlerIntelligence,
    recent_failures: &[RecentFailure],
) -> HookDetailViewModel {
    let internal_ref = HandlerRef::from_aggregate(runtime, aggregate);
    HookDetailViewModel {
        internal_ref: internal_ref.clone(),
        display_identity: resolve_display_identity(aggregate),
        display_disambiguator: None,
        revision: aggregate.handler.revision.clone(),
        event: aggregate.handler.event,
        coverage,
        window,
        runs: aggregate.runs,
        failed_runs: aggregate.failed_runs,
        sample_count: aggregate.failure_sample_count,
        failure_rate_percent: aggregate.failure_rate_percent,
        terminal_breakdown: aggregate.terminal.clone(),
        recent_failures: recent_failures
            .iter()
            .filter(|failure| failure.handler.key == internal_ref.handler_key)
            .map(recent_failure)
            .collect(),
        trends: intelligence.trends.clone(),
        risk: intelligence.risk.clone(),
        failure_fingerprints: intelligence.failure_fingerprints.clone(),
        revision_comparison: intelligence.revision_comparison.clone(),
    }
}

fn risk_order(left: &HookRowViewModel, right: &HookRowViewModel) -> std::cmp::Ordering {
    right
        .risk
        .score
        .cmp(&left.risk.score)
        .then_with(|| right.failed_runs.cmp(&left.failed_runs))
        .then_with(|| {
            right
                .risk
                .sample_confidence_percent
                .cmp(&left.risk.sample_confidence_percent)
        })
        .then_with(|| {
            left.internal_ref
                .handler_key
                .cmp(&right.internal_ref.handler_key)
        })
}

fn assign_fallback_disambiguators(
    rows: &mut [HookRowViewModel],
    details: &mut [HookDetailViewModel],
) {
    let mut totals = std::collections::BTreeMap::<String, usize>::new();
    for row in rows.iter() {
        if matches!(row.display_identity, DisplayIdentity::EventFallback(_)) {
            *totals.entry(row.event.as_storage().to_owned()).or_default() += 1;
        }
    }
    let mut seen = std::collections::BTreeMap::<String, usize>::new();
    let mut by_handler = std::collections::BTreeMap::<String, usize>::new();
    for row in rows.iter_mut() {
        let event = row.event.as_storage().to_owned();
        if matches!(row.display_identity, DisplayIdentity::EventFallback(_))
            && totals.get(&event).copied().unwrap_or_default() > 1
        {
            let index = seen.entry(event).or_default();
            *index += 1;
            row.display_disambiguator = Some(*index);
            by_handler.insert(row.internal_ref.handler_key.clone(), *index);
        }
    }
    for detail in details {
        detail.display_disambiguator = by_handler.get(&detail.internal_ref.handler_key).copied();
    }
}

fn recent_failure(value: &RecentFailure) -> RecentFailureViewModel {
    RecentFailureViewModel {
        occurred_at_unix_ms: value.occurred_at_unix_ms,
        status: value.terminal_status,
        bounded_fingerprint: value.error_fingerprint.clone(),
    }
}

fn resolve_display_identity(aggregate: &HandlerAggregate) -> DisplayIdentity {
    let label = aggregate.handler.label.trim();
    let short_identity = aggregate
        .handler
        .key
        .strip_prefix("hk_")
        .unwrap_or(&aggregate.handler.key);
    let generated = label.is_empty()
        || label == aggregate.handler.key
        || label.contains(&aggregate.handler.key)
        || (!short_identity.is_empty() && label.contains(short_identity))
        || label.starts_with("Codex /");
    if generated {
        DisplayIdentity::EventFallback(aggregate.handler.event)
    } else {
        DisplayIdentity::ExistingMetadata(label.to_owned())
    }
}

fn health(coverage: EvidenceCoverage, failed_runs: u64, samples: u64) -> Health {
    if failed_runs > 0 {
        Health::Degraded
    } else if samples == 0 {
        Health::NoTerminalSamples
    } else if coverage == EvidenceCoverage::Complete {
        Health::Healthy
    } else {
        Health::CoverageLimited
    }
}

fn percentage(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 * 100.0 / denominator as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics::TimeWindow;
    use crate::domain::{
        EvidenceCoverage, EvidenceKind, ExecutionMode, HandlerIdentity, HookInvocation, Runtime,
    };
    use crate::report::{instrumented_report, synthetic_fixture_report};

    fn invocation(label: &str) -> HookInvocation {
        HookInvocation {
            source_key: "fixture".into(),
            source_record_id: "one".into(),
            runtime: Runtime::Codex,
            evidence_kind: EvidenceKind::SyntheticFixture,
            coverage: EvidenceCoverage::SyntheticFixture,
            handler: HandlerIdentity {
                key: "hk_12345678".into(),
                revision: "revision".into(),
                label: label.into(),
                source_kind: "fixture".into(),
                event: HookEvent::Stop,
                matcher_identity: "fixture".into(),
                structural_identity: "fixture".into(),
                execution_mode: ExecutionMode::Sync,
            },
            occurred_at_unix_ms: 1_000,
            terminal_status: TerminalStatus::Completed,
            duration_ms: None,
            error_fingerprint: None,
        }
    }

    fn classified_invocation(
        id: &str,
        key: &str,
        status: TerminalStatus,
        occurred_at_unix_ms: i64,
    ) -> HookInvocation {
        let mut value = invocation("Readable hook");
        value.source_record_id = id.into();
        value.handler.key = key.into();
        value.terminal_status = status;
        value.occurred_at_unix_ms = occurred_at_unix_ms;
        value
    }

    #[test]
    fn overview_keeps_failure_rate_and_terminal_denominator_together() {
        let view = ReliabilityCenterViewModel::from_report(synthetic_fixture_report(1_000));
        let summary = &view.overview.runtime_summaries[0];
        assert_eq!(summary.total_runs, 5);
        assert_eq!(summary.terminal_sample_count, 5);
        assert_eq!(summary.failed_runs, 1);
        assert_eq!(summary.failure_rate_percent, 20.0);
        assert_eq!(summary.health, Health::Degraded);
    }

    #[test]
    fn partial_zero_terminal_samples_are_not_healthy() {
        let view = ReliabilityCenterViewModel::from_report(instrumented_report(
            &[],
            1_000,
            TimeWindow::All,
            0,
            0,
        ));
        assert_eq!(
            view.overview.runtime_summaries[0].health,
            Health::NoTerminalSamples
        );
    }

    #[test]
    fn generated_handler_labels_use_event_fallback_not_internal_identity() {
        let view = ReliabilityCenterViewModel::from_report(instrumented_report(
            &[invocation("Codex / Stop / 12345678")],
            1_000,
            TimeWindow::All,
            0,
            0,
        ));
        let row = &view.hooks.rows[0];
        assert!(matches!(
            row.display_identity,
            DisplayIdentity::EventFallback(HookEvent::Stop)
        ));
        assert_eq!(row.internal_ref.handler_key, "hk_12345678");
    }

    #[test]
    fn readable_existing_metadata_is_preserved_without_changing_the_stable_key() {
        let view = ReliabilityCenterViewModel::from_report(instrumented_report(
            &[invocation("Clean workspace")],
            1_000,
            TimeWindow::All,
            0,
            0,
        ));
        let row = &view.hooks.rows[0];
        assert_eq!(
            row.display_identity,
            DisplayIdentity::ExistingMetadata("Clean workspace".into())
        );
        assert_eq!(row.internal_ref.handler_key, "hk_12345678");
    }

    #[test]
    fn filtering_and_sorting_are_deterministic() {
        let view = ReliabilityCenterViewModel::from_report(synthetic_fixture_report(1_000));
        let query = HooksQuery {
            search: "stop".into(),
            failures_only: true,
            sort: HookSort::FailureRate,
        };
        let rows = view.filtered_hooks(&query);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].internal_ref.handler_key, "alpha");
    }

    #[test]
    fn risk_is_the_default_human_ranking_and_detail_keeps_intelligence_visible() {
        let now = 1_000;
        let mut values = vec![classified_invocation(
            "one",
            "hk_one",
            TerminalStatus::Failed,
            now,
        )];
        for index in 0..20 {
            values.push(classified_invocation(
                &format!("mature-failure-{index}"),
                "hk_mature",
                TerminalStatus::Failed,
                now,
            ));
        }
        for index in 0..80 {
            values.push(classified_invocation(
                &format!("mature-success-{index}"),
                "hk_mature",
                TerminalStatus::Completed,
                now,
            ));
        }
        let view = ReliabilityCenterViewModel::from_report(instrumented_report(
            &values,
            now,
            TimeWindow::All,
            0,
            0,
        ));
        assert_eq!(view.hooks.rows[0].internal_ref.handler_key, "hk_mature");
        assert_eq!(view.hooks.rows[0].trend.window, TimeWindow::All);
        let detail = view.detail(&view.hooks.rows[0].internal_ref).unwrap();
        assert_eq!(detail.trends.len(), 4);
        assert!(detail.risk.sample_confidence_percent > 50);
        assert!(detail.revision_comparison.previous.is_none());
    }

    #[test]
    fn same_event_fallbacks_receive_stable_human_disambiguators() {
        let mut first = invocation("Codex / Stop / first");
        first.handler.key = "hk_first".into();
        first.source_record_id = "first".into();
        let mut second = invocation("Codex / Stop / second");
        second.handler.key = "hk_second".into();
        second.source_record_id = "second".into();
        let view = ReliabilityCenterViewModel::from_report(instrumented_report(
            &[first, second],
            1_000,
            TimeWindow::All,
            0,
            0,
        ));
        assert_eq!(view.hooks.rows[0].display_disambiguator, Some(1));
        assert_eq!(view.hooks.rows[1].display_disambiguator, Some(2));
        assert!(
            view.hooks
                .rows
                .iter()
                .all(|row| !row.display_identity.searchable_text().contains("hk_"))
        );
    }

    #[test]
    fn diagnostics_are_snapshot_only_and_do_not_claim_unchecked_owner_state() {
        let view = ReliabilityCenterViewModel::from_report(instrumented_report(
            &[],
            1_000,
            TimeWindow::All,
            2,
            1,
        ));
        assert_eq!(view.diagnostics.overall_status, DiagnosticStatus::Warning);
        assert!(view.diagnostics.checks.iter().any(|check| {
            check.id == DiagnosticCheckId::ReceiptIntegrity
                && check.facts
                    == vec![DiagnosticFact::ReceiptIntegrity {
                        incomplete: 1,
                        malformed: 2,
                    }]
        }));
        for id in [
            DiagnosticCheckId::Instrumentation,
            DiagnosticCheckId::Trust,
            DiagnosticCheckId::ReceiptSpool,
        ] {
            assert!(view.diagnostics.checks.iter().any(|check| {
                check.id == id
                    && check.status == DiagnosticStatus::Unknown
                    && check.facts.is_empty()
            }));
        }
    }
}
