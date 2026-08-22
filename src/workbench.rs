//! Historical, evidence-backed workbench projections.
//!
//! The workbench consumes only canonical `HookInvocation` fields already
//! admitted to the ledger. It never asks for or retains command text, output,
//! prompts, tool payloads, or an inferred disappearance time.

use crate::analytics::{
    IntelligenceAvailability, PeriodMetrics, RegressionClassification, TimeWindow,
};
use crate::domain::{EvidenceCoverage, HandlerIdentity, HookInvocation, TerminalStatus};
use std::collections::BTreeMap;

const MIN_COMPARISON_SAMPLES: u64 = 5;
const MATERIAL_RATE_CHANGE_PERCENT: f64 = 5.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangeKind {
    Regression,
    Recovery,
    RevisionChange,
    NewAdmittedHook,
    /// A hook has admitted history but no invocation in the selected finite
    /// period. This deliberately says nothing about whether the runtime still
    /// has the hook configured or active.
    HistoricalOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoricalStatus {
    ObservedInSelectedPeriod,
    HistoricalOutsideSelectedPeriod,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RevisionTimelineEpoch {
    pub revision: String,
    pub first_seen_unix_ms: i64,
    pub last_seen_unix_ms: i64,
    pub metrics: PeriodMetrics,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HandlerHistory {
    pub handler: HandlerIdentity,
    pub first_seen_unix_ms: i64,
    pub last_seen_unix_ms: i64,
    pub latest_evidence_unix_ms: i64,
    pub selected_period: PeriodMetrics,
    pub previous_period: Option<PeriodMetrics>,
    pub availability: IntelligenceAvailability,
    pub classification: RegressionClassification,
    pub historical_status: HistoricalStatus,
    pub revision_timeline: Vec<RevisionTimelineEpoch>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChangeEvent {
    pub kind: ChangeKind,
    pub handler_key: String,
    pub occurred_at_unix_ms: i64,
    pub current: PeriodMetrics,
    pub previous: Option<PeriodMetrics>,
    pub availability: IntelligenceAvailability,
    pub revision: Option<String>,
    pub historical_status: HistoricalStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChangesWorkbench {
    pub window: TimeWindow,
    pub coverage: EvidenceCoverage,
    pub generated_at_unix_ms: i64,
    pub handlers: Vec<HandlerHistory>,
    pub events: Vec<ChangeEvent>,
}

/// Builds a deterministic, historical projection from an already admitted
/// snapshot. Callers may obtain the snapshot lazily so finite reliability
/// refreshes remain bounded independently of this long-history surface.
pub fn changes_workbench(
    values: &[HookInvocation],
    generated_at_unix_ms: i64,
    window: TimeWindow,
    coverage: EvidenceCoverage,
) -> ChangesWorkbench {
    let mut by_handler = BTreeMap::<String, Vec<&HookInvocation>>::new();
    for value in values {
        if value.occurred_at_unix_ms <= generated_at_unix_ms {
            by_handler
                .entry(value.handler.key.clone())
                .or_default()
                .push(value);
        }
    }

    let mut handlers = Vec::with_capacity(by_handler.len());
    let mut events = Vec::new();
    for (handler_key, mut invocations) in by_handler {
        invocations.sort_by(timeline_order);
        let history = handler_history(
            invocations.as_slice(),
            generated_at_unix_ms,
            window,
            coverage,
        );
        events.extend(events_for_history(
            &handler_key,
            &history,
            window,
            generated_at_unix_ms,
        ));
        handlers.push(history);
    }
    handlers.sort_by(|left, right| {
        right
            .latest_evidence_unix_ms
            .cmp(&left.latest_evidence_unix_ms)
            .then_with(|| left.handler.key.cmp(&right.handler.key))
    });
    events.sort_by(|left, right| {
        right
            .occurred_at_unix_ms
            .cmp(&left.occurred_at_unix_ms)
            .then_with(|| change_kind_order(left.kind).cmp(&change_kind_order(right.kind)))
            .then_with(|| left.handler_key.cmp(&right.handler_key))
    });
    ChangesWorkbench {
        window,
        coverage,
        generated_at_unix_ms,
        handlers,
        events,
    }
}

fn handler_history(
    invocations: &[&HookInvocation],
    now_unix_ms: i64,
    window: TimeWindow,
    coverage: EvidenceCoverage,
) -> HandlerHistory {
    let bounds = window.bounds_at(now_unix_ms);
    let current = invocations
        .iter()
        .copied()
        .filter(|value| bounds.contains_current(value.occurred_at_unix_ms))
        .collect::<Vec<_>>();
    let previous = invocations
        .iter()
        .copied()
        .filter(|value| bounds.contains_previous(value.occurred_at_unix_ms))
        .collect::<Vec<_>>();
    let selected_period = period_metrics(current.as_slice());
    let previous_period = bounds
        .previous_start_unix_ms
        .is_some()
        .then(|| period_metrics(previous.as_slice()));
    let availability =
        comparison_availability(coverage, &selected_period, previous_period.as_ref());
    let classification = classify(availability, previous_period.as_ref(), &selected_period);
    let latest = invocations
        .last()
        .expect("a handler history has an invocation");
    HandlerHistory {
        handler: latest.handler.clone(),
        first_seen_unix_ms: invocations[0].occurred_at_unix_ms,
        last_seen_unix_ms: latest.occurred_at_unix_ms,
        latest_evidence_unix_ms: latest.occurred_at_unix_ms,
        selected_period,
        previous_period,
        availability,
        classification,
        historical_status: if current.is_empty() {
            HistoricalStatus::HistoricalOutsideSelectedPeriod
        } else {
            HistoricalStatus::ObservedInSelectedPeriod
        },
        revision_timeline: revision_timeline(invocations),
    }
}

fn events_for_history(
    handler_key: &str,
    history: &HandlerHistory,
    window: TimeWindow,
    now_unix_ms: i64,
) -> Vec<ChangeEvent> {
    let mut events = Vec::new();
    let base = || ChangeEvent {
        kind: ChangeKind::Regression,
        handler_key: handler_key.to_owned(),
        occurred_at_unix_ms: history.latest_evidence_unix_ms,
        current: history.selected_period.clone(),
        previous: history.previous_period.clone(),
        availability: history.availability,
        revision: None,
        historical_status: history.historical_status,
    };
    match history.classification {
        RegressionClassification::Regression => {
            let mut event = base();
            event.kind = ChangeKind::Regression;
            events.push(event);
        }
        RegressionClassification::Improvement => {
            let mut event = base();
            event.kind = ChangeKind::Recovery;
            events.push(event);
        }
        RegressionClassification::Stable | RegressionClassification::InsufficientEvidence => {}
    }
    let bounds = window.bounds_at(now_unix_ms);
    if window != TimeWindow::All && bounds.contains_current(history.first_seen_unix_ms) {
        let mut event = base();
        event.kind = ChangeKind::NewAdmittedHook;
        event.occurred_at_unix_ms = history.first_seen_unix_ms;
        events.push(event);
    }
    if window != TimeWindow::All
        && history.historical_status == HistoricalStatus::HistoricalOutsideSelectedPeriod
    {
        let mut event = base();
        event.kind = ChangeKind::HistoricalOnly;
        events.push(event);
    }
    for epoch in history.revision_timeline.iter().skip(1) {
        if bounds.contains_current(epoch.first_seen_unix_ms) {
            let mut event = base();
            event.kind = ChangeKind::RevisionChange;
            event.occurred_at_unix_ms = epoch.first_seen_unix_ms;
            event.revision = Some(epoch.revision.clone());
            events.push(event);
        }
    }
    events
}

fn revision_timeline(invocations: &[&HookInvocation]) -> Vec<RevisionTimelineEpoch> {
    let mut epochs = Vec::<RevisionTimelineEpoch>::new();
    let mut start = 0;
    for index in 1..=invocations.len() {
        let boundary = index == invocations.len()
            || invocations[index].handler.revision != invocations[start].handler.revision;
        if boundary {
            let epoch = &invocations[start..index];
            epochs.push(RevisionTimelineEpoch {
                revision: epoch[0].handler.revision.clone(),
                first_seen_unix_ms: epoch[0].occurred_at_unix_ms,
                last_seen_unix_ms: epoch[epoch.len() - 1].occurred_at_unix_ms,
                metrics: period_metrics(epoch),
            });
            start = index;
        }
    }
    epochs
}

fn comparison_availability(
    coverage: EvidenceCoverage,
    current: &PeriodMetrics,
    previous: Option<&PeriodMetrics>,
) -> IntelligenceAvailability {
    if coverage != EvidenceCoverage::Complete {
        IntelligenceAvailability::CoverageLimited
    } else if previous.is_none() {
        IntelligenceAvailability::InsufficientHistory
    } else if current.failure_sample_count < MIN_COMPARISON_SAMPLES
        || previous.is_some_and(|value| value.failure_sample_count < MIN_COMPARISON_SAMPLES)
    {
        IntelligenceAvailability::InsufficientSamples
    } else {
        IntelligenceAvailability::Available
    }
}

fn classify(
    availability: IntelligenceAvailability,
    previous: Option<&PeriodMetrics>,
    current: &PeriodMetrics,
) -> RegressionClassification {
    if availability != IntelligenceAvailability::Available {
        return RegressionClassification::InsufficientEvidence;
    }
    let delta =
        current.failure_rate_percent - previous.expect("available has prior").failure_rate_percent;
    if delta >= MATERIAL_RATE_CHANGE_PERCENT {
        RegressionClassification::Regression
    } else if delta <= -MATERIAL_RATE_CHANGE_PERCENT {
        RegressionClassification::Improvement
    } else {
        RegressionClassification::Stable
    }
}

fn period_metrics(values: &[&HookInvocation]) -> PeriodMetrics {
    let terminal_samples = values
        .iter()
        .filter(|value| is_terminal(value.terminal_status))
        .count() as u64;
    let failed_runs = values
        .iter()
        .filter(|value| value.terminal_status.is_execution_failure())
        .count() as u64;
    PeriodMetrics {
        runs: values.len() as u64,
        failure_sample_count: terminal_samples,
        failed_runs,
        failure_rate_percent: percentage(failed_runs, terminal_samples),
    }
}

fn is_terminal(status: TerminalStatus) -> bool {
    matches!(
        status,
        TerminalStatus::Completed
            | TerminalStatus::Failed
            | TerminalStatus::Blocked
            | TerminalStatus::Stopped
            | TerminalStatus::TimedOut
            | TerminalStatus::ProtocolFailure
    )
}

fn percentage(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 * 100.0 / denominator as f64
    }
}

fn timeline_order(left: &&HookInvocation, right: &&HookInvocation) -> std::cmp::Ordering {
    left.occurred_at_unix_ms
        .cmp(&right.occurred_at_unix_ms)
        .then_with(|| left.source_key.cmp(&right.source_key))
        .then_with(|| left.source_record_id.cmp(&right.source_record_id))
}

fn change_kind_order(kind: ChangeKind) -> u8 {
    match kind {
        ChangeKind::Regression => 0,
        ChangeKind::Recovery => 1,
        ChangeKind::RevisionChange => 2,
        ChangeKind::NewAdmittedHook => 3,
        ChangeKind::HistoricalOnly => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{EvidenceKind, ExecutionMode, HookEvent, Runtime};

    const HOUR: i64 = 60 * 60 * 1_000;

    fn invocation(
        id: usize,
        key: &str,
        revision: &str,
        status: TerminalStatus,
        at: i64,
    ) -> HookInvocation {
        HookInvocation {
            source_key: "fixture".into(),
            source_record_id: format!("{key}-{id}"),
            runtime: Runtime::Codex,
            evidence_kind: EvidenceKind::SyntheticFixture,
            coverage: EvidenceCoverage::Complete,
            handler: HandlerIdentity {
                key: key.into(),
                revision: revision.into(),
                label: format!("Readable {key}"),
                source_kind: "fixture".into(),
                event: HookEvent::Stop,
                matcher_identity: "fixture".into(),
                structural_identity: "fixture".into(),
                execution_mode: ExecutionMode::Sync,
            },
            occurred_at_unix_ms: at,
            terminal_status: status,
            duration_ms: None,
            error_fingerprint: None,
        }
    }

    fn repeated(
        start: usize,
        key: &str,
        revision: &str,
        status: TerminalStatus,
        at: i64,
        count: usize,
    ) -> Vec<HookInvocation> {
        (0..count)
            .map(|offset| invocation(start + offset, key, revision, status, at + offset as i64))
            .collect()
    }

    #[test]
    fn classifies_regression_recovery_and_same_rate_only_with_proven_samples() {
        let now = 100 * HOUR;
        let mut values = repeated(
            0,
            "regression",
            "r1",
            TerminalStatus::Completed,
            now - 25 * HOUR,
            10,
        );
        values.extend(repeated(
            10,
            "regression",
            "r1",
            TerminalStatus::Failed,
            now - HOUR,
            10,
        ));
        values.extend(repeated(
            20,
            "recovery",
            "r1",
            TerminalStatus::Failed,
            now - 25 * HOUR,
            10,
        ));
        values.extend(repeated(
            30,
            "recovery",
            "r1",
            TerminalStatus::Completed,
            now - HOUR,
            10,
        ));
        values.extend(repeated(
            40,
            "same",
            "r1",
            TerminalStatus::Failed,
            now - 25 * HOUR,
            5,
        ));
        values.extend(repeated(
            45,
            "same",
            "r1",
            TerminalStatus::Failed,
            now - HOUR,
            5,
        ));
        let workbench = changes_workbench(
            &values,
            now,
            TimeWindow::Last24Hours,
            EvidenceCoverage::Complete,
        );
        assert!(
            workbench
                .events
                .iter()
                .any(|event| event.kind == ChangeKind::Regression
                    && event.handler_key == "regression")
        );
        assert!(
            workbench
                .events
                .iter()
                .any(|event| event.kind == ChangeKind::Recovery && event.handler_key == "recovery")
        );
        assert!(
            !workbench
                .events
                .iter()
                .any(|event| event.handler_key == "same"
                    && matches!(event.kind, ChangeKind::Regression | ChangeKind::Recovery))
        );
    }

    #[test]
    fn keeps_insufficient_history_and_partial_coverage_non_claiming() {
        let now = 100 * HOUR;
        let values = repeated(0, "new", "r1", TerminalStatus::Completed, now - HOUR, 2);
        let complete = changes_workbench(
            &values,
            now,
            TimeWindow::Last24Hours,
            EvidenceCoverage::Complete,
        );
        assert_eq!(
            complete.handlers[0].availability,
            IntelligenceAvailability::InsufficientSamples
        );
        assert!(
            complete
                .events
                .iter()
                .any(|event| event.kind == ChangeKind::NewAdmittedHook)
        );
        let partial = changes_workbench(
            &values,
            now,
            TimeWindow::Last24Hours,
            EvidenceCoverage::Partial,
        );
        assert_eq!(
            partial.handlers[0].availability,
            IntelligenceAvailability::CoverageLimited
        );
        assert!(
            !partial
                .events
                .iter()
                .any(|event| matches!(event.kind, ChangeKind::Regression | ChangeKind::Recovery))
        );
        let all_time = changes_workbench(&values, now, TimeWindow::All, EvidenceCoverage::Complete);
        assert_eq!(
            all_time.handlers[0].availability,
            IntelligenceAvailability::InsufficientHistory
        );
    }

    #[test]
    fn preserves_ordered_revisions_and_labels_historical_rows_without_disappearance_claim() {
        let now = 100 * HOUR;
        let mut values = repeated(
            0,
            "timeline",
            "r1",
            TerminalStatus::Completed,
            now - 30 * HOUR,
            6,
        );
        values.extend(repeated(
            10,
            "timeline",
            "r2",
            TerminalStatus::Completed,
            now - HOUR,
            6,
        ));
        values.extend(repeated(
            20,
            "historical",
            "r1",
            TerminalStatus::Completed,
            now - 30 * HOUR,
            6,
        ));
        let workbench = changes_workbench(
            &values,
            now,
            TimeWindow::Last24Hours,
            EvidenceCoverage::Complete,
        );
        let timeline = workbench
            .handlers
            .iter()
            .find(|history| history.handler.key == "timeline")
            .unwrap();
        assert_eq!(
            timeline
                .revision_timeline
                .iter()
                .map(|epoch| epoch.revision.as_str())
                .collect::<Vec<_>>(),
            vec!["r1", "r2"]
        );
        assert!(workbench.events.iter().any(
            |event| event.kind == ChangeKind::RevisionChange && event.handler_key == "timeline"
        ));
        let historical = workbench
            .handlers
            .iter()
            .find(|history| history.handler.key == "historical")
            .unwrap();
        assert_eq!(
            historical.historical_status,
            HistoricalStatus::HistoricalOutsideSelectedPeriod
        );
        assert!(
            workbench
                .events
                .iter()
                .any(|event| event.kind == ChangeKind::HistoricalOnly
                    && event.handler_key == "historical")
        );
    }

    #[test]
    fn period_changes_reclassify_without_fabricating_history() {
        let now = 100 * HOUR;
        let mut values = repeated(
            0,
            "period",
            "r1",
            TerminalStatus::Completed,
            now - 26 * HOUR,
            6,
        );
        values.extend(repeated(
            10,
            "period",
            "r1",
            TerminalStatus::Failed,
            now - HOUR,
            6,
        ));
        let day = changes_workbench(
            &values,
            now,
            TimeWindow::Last24Hours,
            EvidenceCoverage::Complete,
        );
        let week = changes_workbench(
            &values,
            now,
            TimeWindow::Last7Days,
            EvidenceCoverage::Complete,
        );
        assert!(
            day.events
                .iter()
                .any(|event| event.handler_key == "period" && event.kind == ChangeKind::Regression)
        );
        assert_eq!(
            week.handlers[0].historical_status,
            HistoricalStatus::ObservedInSelectedPeriod
        );
    }

    #[test]
    fn projects_ten_thousand_invocations_many_hooks_and_revision_epochs_deterministically() {
        let now = 90 * 24 * HOUR;
        let values = (0..10_240)
            .map(|id| {
                let key = format!("hook-{:02}", id % 64);
                let revision = format!("r{}", id / (64 * 20));
                invocation(
                    id,
                    &key,
                    &revision,
                    if id % 9 == 0 {
                        TerminalStatus::Failed
                    } else {
                        TerminalStatus::Completed
                    },
                    now - (id as i64 * 60_000),
                )
            })
            .collect::<Vec<_>>();
        let workbench = changes_workbench(
            &values,
            now,
            TimeWindow::Last7Days,
            EvidenceCoverage::Complete,
        );
        assert_eq!(workbench.handlers.len(), 64);
        assert!(
            workbench
                .handlers
                .iter()
                .all(|history| history.revision_timeline.len() >= 7)
        );
        assert!(
            workbench
                .events
                .iter()
                .any(|event| event.kind == ChangeKind::RevisionChange)
        );
        assert!(
            workbench
                .events
                .windows(2)
                .all(|pair| { pair[0].occurred_at_unix_ms >= pair[1].occurred_at_unix_ms })
        );
    }
}
