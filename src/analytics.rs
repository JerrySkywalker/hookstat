//! Deterministic, evidence-source-neutral reliability aggregates.

use crate::domain::{EvidenceCoverage, HandlerIdentity, HookEvent, HookInvocation, TerminalStatus};
use chrono::{DateTime, Local, LocalResult, TimeZone};
use serde::Serialize;
use std::collections::BTreeMap;

const HOUR_MS: i64 = 60 * 60 * 1_000;
const MIN_COMPARISON_SAMPLES: u64 = 5;
const CONFIDENCE_SAMPLE_SCALE: u64 = 9;
const MATERIAL_RATE_CHANGE_PERCENT: f64 = 5.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeWindow {
    Today,
    Last24Hours,
    Last7Days,
    Last30Days,
    All,
}

impl TimeWindow {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Today => "Today",
            Self::Last24Hours => "Last 24 hours",
            Self::Last7Days => "Last 7 days",
            Self::Last30Days => "Last 30 days",
            Self::All => "All time",
        }
    }

    pub(crate) const fn width_ms(self) -> Option<i64> {
        match self {
            Self::Today => None,
            Self::Last24Hours => Some(24 * HOUR_MS),
            Self::Last7Days => Some(7 * 24 * HOUR_MS),
            Self::Last30Days => Some(30 * 24 * HOUR_MS),
            Self::All => None,
        }
    }

    /// Resolves the exact current and predecessor interval once per request.
    /// `Today` uses the local civil calendar; its predecessor is the prior
    /// local calendar day, which can be 23 or 25 hours around DST changes.
    pub fn bounds_at(self, now_unix_ms: i64) -> TimeBounds {
        match self {
            Self::Today => local_today_bounds(now_unix_ms),
            Self::Last24Hours | Self::Last7Days | Self::Last30Days => {
                let width = self.width_ms().expect("rolling windows have a width");
                TimeBounds {
                    current_start_unix_ms: Some(now_unix_ms.saturating_sub(width)),
                    current_end_unix_ms: now_unix_ms,
                    previous_start_unix_ms: Some(now_unix_ms.saturating_sub(width * 2)),
                    previous_end_unix_ms: Some(now_unix_ms.saturating_sub(width)),
                }
            }
            Self::All => TimeBounds {
                current_start_unix_ms: None,
                current_end_unix_ms: now_unix_ms,
                previous_start_unix_ms: None,
                previous_end_unix_ms: None,
            },
        }
    }
}

/// Locale-neutral, epoch-millisecond intervals selected by a `TimeWindow`.
/// Ends of current periods are inclusive; predecessor ends are exclusive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimeBounds {
    pub current_start_unix_ms: Option<i64>,
    pub current_end_unix_ms: i64,
    pub previous_start_unix_ms: Option<i64>,
    pub previous_end_unix_ms: Option<i64>,
}

impl TimeBounds {
    /// Testable calendar adapter used by local-time implementations. Supplying
    /// civil midnights rather than a fixed duration keeps DST-day lengths
    /// explicit and prevents `Today` from becoming an alias for rolling 24h.
    pub const fn local_civil_day(
        current_start_unix_ms: i64,
        previous_start_unix_ms: i64,
        now_unix_ms: i64,
    ) -> Self {
        Self {
            current_start_unix_ms: Some(current_start_unix_ms),
            current_end_unix_ms: now_unix_ms,
            previous_start_unix_ms: Some(previous_start_unix_ms),
            previous_end_unix_ms: Some(current_start_unix_ms),
        }
    }

    pub const fn contains_current(self, timestamp: i64) -> bool {
        timestamp <= self.current_end_unix_ms
            && match self.current_start_unix_ms {
                Some(start) => timestamp >= start,
                None => true,
            }
    }

    pub const fn contains_previous(self, timestamp: i64) -> bool {
        match (self.previous_start_unix_ms, self.previous_end_unix_ms) {
            (Some(start), Some(end)) => timestamp >= start && timestamp < end,
            _ => false,
        }
    }
}

fn local_today_bounds(now_unix_ms: i64) -> TimeBounds {
    let now = Local
        .timestamp_millis_opt(now_unix_ms)
        .single()
        .unwrap_or_else(|| {
            Local
                .timestamp_opt(now_unix_ms.div_euclid(1_000), 0)
                .earliest()
                .expect("supported timestamp has a local representation")
        });
    local_today_bounds_for(now)
}

fn local_today_bounds_for(now: DateTime<Local>) -> TimeBounds {
    let current_start = local_midnight(now.date_naive());
    let previous_start = local_midnight(
        now.date_naive()
            .pred_opt()
            .expect("supported calendar date has a previous day"),
    );
    TimeBounds::local_civil_day(
        current_start.timestamp_millis(),
        previous_start.timestamp_millis(),
        now.timestamp_millis(),
    )
}

fn local_midnight(date: chrono::NaiveDate) -> DateTime<Local> {
    let midnight = date.and_hms_opt(0, 0, 0).expect("midnight is valid");
    match Local.from_local_datetime(&midnight) {
        LocalResult::Single(value) => value,
        // A few historical zones transition at midnight. Selecting the first
        // real local instant preserves the civil-day boundary without using a
        // fixed 24-hour assumption.
        LocalResult::Ambiguous(earliest, _) => earliest,
        LocalResult::None => Local
            .from_local_datetime(&(midnight + chrono::TimeDelta::hours(1)))
            .earliest()
            .expect("a local civil day has a first instant"),
    }
}

/// Terminal breakdown deliberately keeps control, fault, and missing-terminal
/// evidence distinct. `incomplete`/`unknown` are not counted as successes.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct TerminalBreakdown {
    pub completed: u64,
    pub failed: u64,
    pub blocked: u64,
    pub stopped: u64,
    pub timed_out: u64,
    pub protocol_failure: u64,
    pub incomplete: u64,
    pub unknown: u64,
}

impl TerminalBreakdown {
    fn observe(&mut self, status: TerminalStatus) {
        match status {
            TerminalStatus::Completed => self.completed += 1,
            TerminalStatus::Failed => self.failed += 1,
            TerminalStatus::Blocked => self.blocked += 1,
            TerminalStatus::Stopped => self.stopped += 1,
            TerminalStatus::TimedOut => self.timed_out += 1,
            TerminalStatus::ProtocolFailure => self.protocol_failure += 1,
            TerminalStatus::Incomplete => self.incomplete += 1,
            TerminalStatus::Unknown => self.unknown += 1,
        }
    }

    fn failures(&self) -> u64 {
        self.failed + self.timed_out + self.protocol_failure
    }

    fn terminal_samples(&self) -> u64 {
        self.completed
            + self.failed
            + self.blocked
            + self.stopped
            + self.timed_out
            + self.protocol_failure
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HandlerAggregate {
    pub handler: HandlerIdentity,
    /// Every started invocation seen in this window, including incomplete ones.
    pub runs: u64,
    /// The denominator used by `failure_rate_percent`; always render alongside it.
    pub failure_sample_count: u64,
    pub failed_runs: u64,
    pub failure_rate_percent: f64,
    pub previous_window_delta_percent: Option<f64>,
    pub terminal: TerminalBreakdown,
    pub p50_duration_ms: Option<u64>,
    pub p95_duration_ms: Option<u64>,
    pub p99_duration_ms: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RecentFailure {
    pub handler: HandlerIdentity,
    pub occurred_at_unix_ms: i64,
    pub terminal_status: TerminalStatus,
    pub error_fingerprint: Option<String>,
}

/// A count/rate pair for a real, bounded time period. It is deliberately not a
/// health claim: zero denominators remain explicit and are paired with the
/// availability state of its enclosing projection.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PeriodMetrics {
    pub runs: u64,
    pub failure_sample_count: u64,
    pub failed_runs: u64,
    pub failure_rate_percent: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IntelligenceAvailability {
    Available,
    InsufficientHistory,
    InsufficientSamples,
    CoverageLimited,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RegressionClassification {
    Regression,
    Improvement,
    Stable,
    InsufficientEvidence,
}

/// Current and immediately preceding, non-overlapping periods. The current
/// period is `[now - width, now]`; its predecessor is `[now - 2*width, now -
/// width)`. `All` has no fabricated predecessor.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TrendProjection {
    pub window: TimeWindow,
    pub current: PeriodMetrics,
    pub previous: Option<PeriodMetrics>,
    pub delta_failure_rate_percent: Option<f64>,
    pub availability: IntelligenceAvailability,
    pub classification: RegressionClassification,
}

/// A transparent, bounded prioritisation score. `score` is not a probability
/// or a health verdict. It combines the shown components so a one-run failure
/// remains visibly uncertain rather than automatically leading the ranking.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RiskScore {
    pub score: u8,
    pub failure_rate_component_points: f64,
    pub sample_confidence_percent: u8,
    pub recency_points: i8,
    pub trend_points: i8,
    pub impact_points: i8,
    pub coverage_multiplier_percent: u8,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureFingerprintKind {
    ExitNonzero,
    TimedOut,
    ProtocolFailure,
    ExecutionFailed,
}

/// Grouping uses only the already-admitted bounded status/fingerprint taxonomy.
/// It intentionally stores no message text, exit stream, command, or payload.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FailureFingerprintCluster {
    pub kind: FailureFingerprintKind,
    pub occurrences: u64,
    pub latest_occurred_at_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RevisionMetrics {
    pub revision: String,
    pub runs: u64,
    pub failure_sample_count: u64,
    pub failed_runs: u64,
    pub failure_rate_percent: f64,
}

/// Revision comparison is based exclusively on a stable handler key. The
/// current revision is the latest observed revision in a deterministic
/// `(timestamp, source key, source record)` timeline. The previous revision is
/// the immediately preceding contiguous revision epoch, never a name match.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RevisionComparison {
    pub current: RevisionMetrics,
    pub previous: Option<RevisionMetrics>,
    pub delta_failure_rate_percent: Option<f64>,
    pub availability: IntelligenceAvailability,
    pub classification: RegressionClassification,
}

/// Builds the `All` trend from a database aggregate without materializing all
/// historical invocations for a finite selected period.
pub fn all_time_trend(metrics: PeriodMetrics, coverage: EvidenceCoverage) -> TrendProjection {
    let availability = comparison_availability(coverage, &metrics, None);
    TrendProjection {
        window: TimeWindow::All,
        current: metrics,
        previous: None,
        delta_failure_rate_percent: None,
        classification: classify_change(availability, None),
        availability,
    }
}

/// Builds an exact revision projection from adjacent revision epochs returned
/// by specialized ledger aggregates. It preserves the released availability
/// and classification rules without requiring raw historical row loading.
pub fn revision_comparison_from_epochs(
    current: RevisionMetrics,
    previous: Option<RevisionMetrics>,
    coverage: EvidenceCoverage,
) -> RevisionComparison {
    let current_period = PeriodMetrics {
        runs: current.runs,
        failure_sample_count: current.failure_sample_count,
        failed_runs: current.failed_runs,
        failure_rate_percent: current.failure_rate_percent,
    };
    let previous_period = previous.as_ref().map(|metrics| PeriodMetrics {
        runs: metrics.runs,
        failure_sample_count: metrics.failure_sample_count,
        failed_runs: metrics.failed_runs,
        failure_rate_percent: metrics.failure_rate_percent,
    });
    let availability = comparison_availability(coverage, &current_period, previous_period.as_ref());
    let delta_failure_rate_percent = previous
        .as_ref()
        .map(|previous| current.failure_rate_percent - previous.failure_rate_percent);
    RevisionComparison {
        current,
        previous,
        delta_failure_rate_percent,
        classification: classify_change(availability, delta_failure_rate_percent),
        availability,
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HandlerIntelligence {
    pub handler_key: String,
    /// Includes 24h, 7d, 30d, and All in a fixed order.
    pub trends: Vec<TrendProjection>,
    pub risk: RiskScore,
    pub failure_fingerprints: Vec<FailureFingerprintCluster>,
    pub revision_comparison: RevisionComparison,
}

#[derive(Clone, Debug)]
struct AggregateBuilder {
    handler: HandlerIdentity,
    runs: u64,
    terminal: TerminalBreakdown,
    durations: Vec<u64>,
}

impl AggregateBuilder {
    fn new(handler: HandlerIdentity) -> Self {
        Self {
            handler,
            runs: 0,
            terminal: TerminalBreakdown::default(),
            durations: Vec::new(),
        }
    }

    fn observe(&mut self, value: &HookInvocation) {
        self.runs += 1;
        self.terminal.observe(value.terminal_status);
        if let Some(duration) = value.duration_ms {
            self.durations.push(duration);
        }
    }

    fn failure_rate_percent(&self) -> f64 {
        percentage(self.terminal.failures(), self.terminal.terminal_samples())
    }

    fn into_aggregate(self, previous: Option<&AggregateBuilder>) -> HandlerAggregate {
        let failure_rate_percent = self.failure_rate_percent();
        let previous_window_delta_percent =
            previous.map(|item| failure_rate_percent - item.failure_rate_percent());
        let terminal_samples = self.terminal.terminal_samples();
        let has_complete_duration_support =
            self.durations.len() as u64 == terminal_samples && self.runs == terminal_samples;
        let mut durations = self.durations;
        durations.sort_unstable();
        HandlerAggregate {
            handler: self.handler,
            runs: self.runs,
            failure_sample_count: terminal_samples,
            failed_runs: self.terminal.failures(),
            failure_rate_percent,
            previous_window_delta_percent,
            terminal: self.terminal,
            p50_duration_ms: has_complete_duration_support.then(|| percentile(&durations, 0.50)),
            p95_duration_ms: has_complete_duration_support.then(|| percentile(&durations, 0.95)),
            p99_duration_ms: has_complete_duration_support.then(|| percentile(&durations, 0.99)),
        }
    }
}

pub fn aggregate(
    values: &[HookInvocation],
    now_unix_ms: i64,
    window: TimeWindow,
) -> Vec<HandlerAggregate> {
    let current = collect(values, now_unix_ms, window);
    let previous = collect_previous(values, now_unix_ms, window);
    let mut result = current
        .into_values()
        .map(|builder| {
            let key = builder.handler.key.clone();
            builder.into_aggregate(previous.get(&key))
        })
        .collect::<Vec<_>>();
    result.sort_by(|left, right| {
        right
            .failure_rate_percent
            .total_cmp(&left.failure_rate_percent)
            .then_with(|| left.handler.key.cmp(&right.handler.key))
    });
    result
}

/// Builds privacy-preserving intelligence from canonical invocations. This is
/// intentionally independent of Codex discovery, receipt storage, and TUI
/// rendering so future runtimes can provide the same normalized records.
pub fn reliability_intelligence(
    values: &[HookInvocation],
    now_unix_ms: i64,
    selected_window: TimeWindow,
    coverage: EvidenceCoverage,
) -> Vec<HandlerIntelligence> {
    let mut per_handler = BTreeMap::<String, Vec<&HookInvocation>>::new();
    for value in values {
        per_handler
            .entry(value.handler.key.clone())
            .or_default()
            .push(value);
    }

    per_handler
        .into_iter()
        .map(|(handler_key, values)| {
            let trends = [
                TimeWindow::Today,
                TimeWindow::Last24Hours,
                TimeWindow::Last7Days,
                TimeWindow::Last30Days,
                TimeWindow::All,
            ]
            .into_iter()
            .map(|window| trend_projection(&values, now_unix_ms, window, coverage))
            .collect::<Vec<_>>();
            let selected = trends
                .iter()
                .find(|trend| trend.window == selected_window)
                .expect("fixed trend windows include selected window");
            let handler = latest_handler(&values);
            HandlerIntelligence {
                handler_key,
                risk: risk_score(
                    &values,
                    now_unix_ms,
                    selected_window,
                    coverage,
                    selected,
                    handler.event,
                ),
                failure_fingerprints: failure_fingerprint_clusters(
                    &values,
                    now_unix_ms,
                    selected_window,
                ),
                revision_comparison: revision_comparison(&values, coverage),
                trends,
            }
        })
        .collect()
}

pub fn recent_failures(
    values: &[HookInvocation],
    now_unix_ms: i64,
    window: TimeWindow,
    limit: usize,
) -> Vec<RecentFailure> {
    let mut failures = values
        .iter()
        .filter(|item| {
            in_current_window(item.occurred_at_unix_ms, now_unix_ms, window)
                && item.terminal_status.is_execution_failure()
        })
        .map(|item| RecentFailure {
            handler: item.handler.clone(),
            occurred_at_unix_ms: item.occurred_at_unix_ms,
            terminal_status: item.terminal_status,
            error_fingerprint: bounded_error_fingerprint(item.error_fingerprint.as_deref()),
        })
        .collect::<Vec<_>>();
    failures.sort_by_key(|item| std::cmp::Reverse(item.occurred_at_unix_ms));
    failures.truncate(limit);
    failures
}

fn trend_projection(
    values: &[&HookInvocation],
    now: i64,
    window: TimeWindow,
    coverage: EvidenceCoverage,
) -> TrendProjection {
    let current_values = values
        .iter()
        .copied()
        .filter(|value| in_current_window(value.occurred_at_unix_ms, now, window))
        .collect::<Vec<_>>();
    let current = period_metrics(&current_values);
    let previous_values = previous_values(values, now, window);
    let previous = previous_values
        .as_ref()
        .filter(|items| !items.is_empty())
        .map(|items| period_metrics(items));
    let delta_failure_rate_percent = previous
        .as_ref()
        .map(|previous| current.failure_rate_percent - previous.failure_rate_percent);
    let availability = comparison_availability(coverage, &current, previous.as_ref());
    TrendProjection {
        window,
        current,
        previous,
        delta_failure_rate_percent,
        classification: classify_change(availability, delta_failure_rate_percent),
        availability,
    }
}

fn previous_values<'a>(
    values: &'a [&'a HookInvocation],
    now: i64,
    window: TimeWindow,
) -> Option<Vec<&'a HookInvocation>> {
    let bounds = window.bounds_at(now);
    bounds.previous_start_unix_ms?;
    Some(
        values
            .iter()
            .copied()
            .filter(|value| bounds.contains_previous(value.occurred_at_unix_ms))
            .collect(),
    )
}

fn comparison_availability(
    coverage: EvidenceCoverage,
    current: &PeriodMetrics,
    previous: Option<&PeriodMetrics>,
) -> IntelligenceAvailability {
    if !coverage_is_sufficient(coverage) {
        IntelligenceAvailability::CoverageLimited
    } else if previous.is_none() {
        IntelligenceAvailability::InsufficientHistory
    } else if current.failure_sample_count < MIN_COMPARISON_SAMPLES
        || previous.is_some_and(|metrics| metrics.failure_sample_count < MIN_COMPARISON_SAMPLES)
    {
        IntelligenceAvailability::InsufficientSamples
    } else {
        IntelligenceAvailability::Available
    }
}

fn classify_change(
    availability: IntelligenceAvailability,
    delta_failure_rate_percent: Option<f64>,
) -> RegressionClassification {
    if availability != IntelligenceAvailability::Available {
        return RegressionClassification::InsufficientEvidence;
    }
    match delta_failure_rate_percent.expect("available comparison has a previous period") {
        delta if delta >= MATERIAL_RATE_CHANGE_PERCENT => RegressionClassification::Regression,
        delta if delta <= -MATERIAL_RATE_CHANGE_PERCENT => RegressionClassification::Improvement,
        _ => RegressionClassification::Stable,
    }
}

fn risk_score(
    values: &[&HookInvocation],
    now: i64,
    selected_window: TimeWindow,
    coverage: EvidenceCoverage,
    selected_trend: &TrendProjection,
    event: HookEvent,
) -> RiskScore {
    let current_values = values
        .iter()
        .copied()
        .filter(|value| in_current_window(value.occurred_at_unix_ms, now, selected_window))
        .collect::<Vec<_>>();
    let current = period_metrics(&current_values);
    let confidence = (current.failure_sample_count * 100
        / (current.failure_sample_count + CONFIDENCE_SAMPLE_SCALE)) as u8;
    let coverage_multiplier_percent = coverage_multiplier_percent(coverage);
    let failure_rate_component_points = current.failure_rate_percent * f64::from(confidence)
        / 100.0
        * f64::from(coverage_multiplier_percent)
        / 100.0;
    let recency_points = current_values
        .iter()
        .filter(|value| value.terminal_status.is_execution_failure())
        .map(|value| now.saturating_sub(value.occurred_at_unix_ms))
        .min()
        .map_or(0, recency_points);
    let trend_points = match selected_trend.classification {
        RegressionClassification::Regression => 15,
        RegressionClassification::Improvement => -5,
        RegressionClassification::Stable | RegressionClassification::InsufficientEvidence => 0,
    };
    let impact_points = impact_points(event);
    let score = (failure_rate_component_points
        + f64::from(recency_points)
        + f64::from(trend_points)
        + f64::from(impact_points))
    .round()
    .clamp(0.0, 100.0) as u8;
    RiskScore {
        score,
        failure_rate_component_points,
        sample_confidence_percent: confidence,
        recency_points,
        trend_points,
        impact_points,
        coverage_multiplier_percent,
    }
}

fn recency_points(age_ms: i64) -> i8 {
    if age_ms <= 24 * HOUR_MS {
        15
    } else if age_ms <= 7 * 24 * HOUR_MS {
        10
    } else if age_ms <= 30 * 24 * HOUR_MS {
        5
    } else {
        0
    }
}

fn impact_points(event: HookEvent) -> i8 {
    match event {
        HookEvent::Stop | HookEvent::SessionEnd | HookEvent::PermissionRequest => 15,
        HookEvent::SessionStart
        | HookEvent::PreToolUse
        | HookEvent::PostToolUse
        | HookEvent::SubagentStop => 10,
        HookEvent::UserPromptSubmit
        | HookEvent::PreCompact
        | HookEvent::PostCompact
        | HookEvent::SubagentStart => 5,
    }
}

fn coverage_multiplier_percent(coverage: EvidenceCoverage) -> u8 {
    match coverage {
        EvidenceCoverage::Complete | EvidenceCoverage::SyntheticFixture => 100,
        EvidenceCoverage::SyncOnly => 70,
        EvidenceCoverage::Partial => 65,
        EvidenceCoverage::BestEffort => 55,
        EvidenceCoverage::Unknown => 35,
        EvidenceCoverage::NotAdmitted => 0,
    }
}

fn coverage_is_sufficient(coverage: EvidenceCoverage) -> bool {
    matches!(
        coverage,
        EvidenceCoverage::Complete | EvidenceCoverage::SyntheticFixture
    )
}

fn failure_fingerprint_clusters(
    values: &[&HookInvocation],
    now: i64,
    window: TimeWindow,
) -> Vec<FailureFingerprintCluster> {
    let mut clusters = BTreeMap::<FailureFingerprintKind, (u64, i64)>::new();
    for value in values.iter().copied().filter(|value| {
        in_current_window(value.occurred_at_unix_ms, now, window)
            && value.terminal_status.is_execution_failure()
    }) {
        let entry = clusters
            .entry(failure_fingerprint_kind(value))
            .or_insert((0, value.occurred_at_unix_ms));
        entry.0 += 1;
        entry.1 = entry.1.max(value.occurred_at_unix_ms);
    }
    let mut result = clusters
        .into_iter()
        .map(
            |(kind, (occurrences, latest_occurred_at_unix_ms))| FailureFingerprintCluster {
                kind,
                occurrences,
                latest_occurred_at_unix_ms,
            },
        )
        .collect::<Vec<_>>();
    result.sort_by(|left, right| {
        right
            .occurrences
            .cmp(&left.occurrences)
            .then_with(|| left.kind.cmp(&right.kind))
    });
    result
}

fn failure_fingerprint_kind(value: &HookInvocation) -> FailureFingerprintKind {
    match bounded_error_fingerprint(value.error_fingerprint.as_deref()).as_deref() {
        Some("exit_nonzero") => FailureFingerprintKind::ExitNonzero,
        _ => match value.terminal_status {
            TerminalStatus::TimedOut => FailureFingerprintKind::TimedOut,
            TerminalStatus::ProtocolFailure => FailureFingerprintKind::ProtocolFailure,
            TerminalStatus::Failed => FailureFingerprintKind::ExecutionFailed,
            _ => FailureFingerprintKind::ExecutionFailed,
        },
    }
}

fn bounded_error_fingerprint(value: Option<&str>) -> Option<String> {
    match value {
        Some("exit_nonzero") => Some("exit_nonzero".into()),
        _ => None,
    }
}

fn revision_comparison(
    values: &[&HookInvocation],
    coverage: EvidenceCoverage,
) -> RevisionComparison {
    let mut timeline = values.to_vec();
    timeline.sort_by(|left, right| {
        left.occurred_at_unix_ms
            .cmp(&right.occurred_at_unix_ms)
            .then_with(|| left.source_key.cmp(&right.source_key))
            .then_with(|| left.source_record_id.cmp(&right.source_record_id))
    });
    let current_revision = timeline
        .last()
        .expect("handler intelligence has at least one invocation")
        .handler
        .revision
        .clone();
    let current_start = timeline
        .iter()
        .rposition(|value| value.handler.revision != current_revision)
        .map_or(0, |index| index + 1);
    let current_values = &timeline[current_start..];
    let previous = if current_start == 0 {
        None
    } else {
        let previous_revision = timeline[current_start - 1].handler.revision.clone();
        let previous_start = timeline[..current_start]
            .iter()
            .rposition(|value| value.handler.revision != previous_revision)
            .map_or(0, |index| index + 1);
        Some(revision_metrics(
            previous_revision,
            &timeline[previous_start..current_start],
        ))
    };
    let current = revision_metrics(current_revision, current_values);
    revision_comparison_from_epochs(current, previous, coverage)
}

fn revision_metrics(revision: String, values: &[&HookInvocation]) -> RevisionMetrics {
    let metrics = period_metrics(values);
    RevisionMetrics {
        revision,
        runs: metrics.runs,
        failure_sample_count: metrics.failure_sample_count,
        failed_runs: metrics.failed_runs,
        failure_rate_percent: metrics.failure_rate_percent,
    }
}

fn latest_handler<'a>(values: &'a [&'a HookInvocation]) -> &'a HandlerIdentity {
    let latest = values
        .iter()
        .copied()
        .max_by(|left, right| {
            left.occurred_at_unix_ms
                .cmp(&right.occurred_at_unix_ms)
                .then_with(|| left.source_key.cmp(&right.source_key))
                .then_with(|| left.source_record_id.cmp(&right.source_record_id))
        })
        .expect("handler intelligence has at least one invocation");
    &latest.handler
}

fn period_metrics(values: &[&HookInvocation]) -> PeriodMetrics {
    let mut terminal = TerminalBreakdown::default();
    for value in values {
        terminal.observe(value.terminal_status);
    }
    PeriodMetrics {
        runs: values.len() as u64,
        failure_sample_count: terminal.terminal_samples(),
        failed_runs: terminal.failures(),
        failure_rate_percent: percentage(terminal.failures(), terminal.terminal_samples()),
    }
}

fn collect(
    values: &[HookInvocation],
    now: i64,
    window: TimeWindow,
) -> BTreeMap<String, AggregateBuilder> {
    let mut result = BTreeMap::new();
    for item in values
        .iter()
        .filter(|item| in_current_window(item.occurred_at_unix_ms, now, window))
    {
        result
            .entry(item.handler.key.clone())
            .or_insert_with(|| AggregateBuilder::new(item.handler.clone()))
            .observe(item);
    }
    result
}

fn collect_previous(
    values: &[HookInvocation],
    now: i64,
    window: TimeWindow,
) -> BTreeMap<String, AggregateBuilder> {
    let bounds = window.bounds_at(now);
    if bounds.previous_start_unix_ms.is_none() {
        return BTreeMap::new();
    }
    let mut result = BTreeMap::new();
    for item in values
        .iter()
        .filter(|item| bounds.contains_previous(item.occurred_at_unix_ms))
    {
        result
            .entry(item.handler.key.clone())
            .or_insert_with(|| AggregateBuilder::new(item.handler.clone()))
            .observe(item);
    }
    result
}

fn in_current_window(timestamp: i64, now: i64, window: TimeWindow) -> bool {
    window.bounds_at(now).contains_current(timestamp)
}

fn percentage(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 * 100.0 / denominator as f64
    }
}

fn percentile(sorted: &[u64], percentile: f64) -> u64 {
    debug_assert!(!sorted.is_empty());
    sorted[((sorted.len() - 1) as f64 * percentile).ceil() as usize]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{EvidenceKind, ExecutionMode, Runtime};
    use chrono::{TimeZone, Timelike};
    use chrono_tz::America::New_York;

    fn handler(key: &str) -> HandlerIdentity {
        HandlerIdentity {
            key: key.into(),
            revision: "r1".into(),
            label: key.into(),
            source_kind: "fixture".into(),
            event: HookEvent::Stop,
            matcher_identity: "any".into(),
            structural_identity: "g0:h0".into(),
            execution_mode: ExecutionMode::Sync,
        }
    }

    fn invocation(id: &str, key: &str, status: TerminalStatus, timestamp: i64) -> HookInvocation {
        HookInvocation {
            source_key: "fixture".into(),
            source_record_id: id.into(),
            runtime: Runtime::Codex,
            evidence_kind: EvidenceKind::SyntheticFixture,
            coverage: EvidenceCoverage::SyntheticFixture,
            handler: handler(key),
            occurred_at_unix_ms: timestamp,
            terminal_status: status,
            duration_ms: None,
            error_fingerprint: None,
        }
    }

    fn repeated(
        prefix: &str,
        key: &str,
        status: TerminalStatus,
        start: i64,
        count: usize,
    ) -> Vec<HookInvocation> {
        (0..count)
            .map(|index| {
                invocation(
                    &format!("{prefix}-{index}"),
                    key,
                    status,
                    start + index as i64,
                )
            })
            .collect()
    }

    #[test]
    fn failure_rate_excludes_blocked_stopped_and_incomplete() {
        let now = 1_000_000;
        let report = aggregate(
            &[
                invocation("1", "one", TerminalStatus::Completed, now),
                invocation("2", "one", TerminalStatus::Failed, now),
                invocation("3", "one", TerminalStatus::Blocked, now),
                invocation("4", "one", TerminalStatus::Stopped, now),
                invocation("5", "one", TerminalStatus::Incomplete, now),
            ],
            now,
            TimeWindow::Last24Hours,
        );
        assert_eq!(report[0].runs, 5);
        assert_eq!(report[0].failure_sample_count, 4);
        assert_eq!(report[0].failed_runs, 1);
        assert_eq!(report[0].failure_rate_percent, 25.0);
        assert_eq!(report[0].terminal.incomplete, 1);
    }

    #[test]
    fn separate_handlers_on_same_event_are_never_conflated() {
        let now = 1_000_000;
        let report = aggregate(
            &[
                invocation("1", "a", TerminalStatus::Completed, now),
                invocation("2", "b", TerminalStatus::Failed, now),
            ],
            now,
            TimeWindow::Last24Hours,
        );
        assert_eq!(report.len(), 2);
    }

    #[test]
    fn incomplete_duration_support_does_not_invent_percentiles() {
        let now = 1_000_000;
        let mut first = invocation("1", "one", TerminalStatus::Completed, now);
        first.duration_ms = Some(5);
        let second = invocation("2", "one", TerminalStatus::Incomplete, now);
        assert_eq!(
            aggregate(&[first, second], now, TimeWindow::Last24Hours)[0].p50_duration_ms,
            None
        );
    }

    #[test]
    fn window_edges_are_non_overlapping_and_previous_delta_is_exact() {
        let now = 1_000_000_000;
        let width = TimeWindow::Last24Hours.width_ms().unwrap();
        let report = aggregate(
            &[
                invocation("old-edge", "one", TerminalStatus::Failed, now - 2 * width),
                invocation(
                    "previous-end",
                    "one",
                    TerminalStatus::Completed,
                    now - width - 1,
                ),
                invocation("current-start", "one", TerminalStatus::Failed, now - width),
                invocation("current-end", "one", TerminalStatus::Completed, now),
                invocation("future", "one", TerminalStatus::Failed, now + 1),
            ],
            now,
            TimeWindow::Last24Hours,
        );
        assert_eq!(report[0].runs, 2);
        assert_eq!(report[0].failure_sample_count, 2);
        assert_eq!(report[0].failure_rate_percent, 50.0);
        assert_eq!(report[0].previous_window_delta_percent, Some(0.0));
        let all = aggregate(
            &[
                invocation("known", "all", TerminalStatus::Completed, now),
                invocation("future", "all", TerminalStatus::Failed, now + 1),
            ],
            now,
            TimeWindow::All,
        );
        assert_eq!(all[0].runs, 1);
    }

    #[test]
    fn today_is_local_civil_day_not_rolling_24_hours_at_midnight() {
        let now = New_York
            .with_ymd_and_hms(2026, 8, 22, 0, 30, 0)
            .single()
            .unwrap();
        let current_start = New_York
            .with_ymd_and_hms(2026, 8, 22, 0, 0, 0)
            .single()
            .unwrap();
        let previous_start = New_York
            .with_ymd_and_hms(2026, 8, 21, 0, 0, 0)
            .single()
            .unwrap();
        let today = TimeBounds::local_civil_day(
            current_start.timestamp_millis(),
            previous_start.timestamp_millis(),
            now.timestamp_millis(),
        );
        let rolling = TimeWindow::Last24Hours.bounds_at(now.timestamp_millis());
        let yesterday_2330 = New_York
            .with_ymd_and_hms(2026, 8, 21, 23, 30, 0)
            .single()
            .unwrap()
            .timestamp_millis();
        assert!(!today.contains_current(yesterday_2330));
        assert!(rolling.contains_current(yesterday_2330));
        assert!(today.contains_previous(yesterday_2330));
        assert_eq!(now.hour(), 0);
    }

    #[test]
    fn today_civil_boundaries_support_dst_short_days() {
        let now = New_York
            .with_ymd_and_hms(2026, 3, 8, 12, 0, 0)
            .single()
            .unwrap();
        let current_start = New_York
            .with_ymd_and_hms(2026, 3, 8, 0, 0, 0)
            .single()
            .unwrap();
        let previous_start = New_York
            .with_ymd_and_hms(2026, 3, 7, 0, 0, 0)
            .single()
            .unwrap();
        let bounds = TimeBounds::local_civil_day(
            current_start.timestamp_millis(),
            previous_start.timestamp_millis(),
            now.timestamp_millis(),
        );
        assert_eq!(
            bounds.previous_end_unix_ms.unwrap() - bounds.previous_start_unix_ms.unwrap(),
            24 * HOUR_MS
        );
        let next_start = New_York
            .with_ymd_and_hms(2026, 3, 9, 0, 0, 0)
            .single()
            .unwrap();
        assert_eq!(
            next_start.timestamp_millis() - current_start.timestamp_millis(),
            23 * HOUR_MS
        );
    }

    #[test]
    fn seven_and_thirty_day_trends_have_real_previous_periods() {
        let now = 90 * 24 * HOUR_MS;
        let seven = 7 * 24 * HOUR_MS;
        let thirty = 30 * 24 * HOUR_MS;
        let mut values = Vec::new();
        values.extend(repeated(
            "seven-previous",
            "one",
            TerminalStatus::Completed,
            now - 2 * seven,
            5,
        ));
        values.extend(repeated(
            "seven-current",
            "one",
            TerminalStatus::Failed,
            now - seven,
            5,
        ));
        values.extend(repeated(
            "thirty-previous",
            "two",
            TerminalStatus::Completed,
            now - 2 * thirty,
            5,
        ));
        values.extend(repeated(
            "thirty-current",
            "two",
            TerminalStatus::Failed,
            now - thirty,
            5,
        ));
        let intelligence = reliability_intelligence(
            &values,
            now,
            TimeWindow::Last7Days,
            EvidenceCoverage::SyntheticFixture,
        );
        let one = intelligence
            .iter()
            .find(|item| item.handler_key == "one")
            .unwrap();
        let seven = one
            .trends
            .iter()
            .find(|trend| trend.window == TimeWindow::Last7Days)
            .unwrap();
        assert_eq!(seven.classification, RegressionClassification::Regression);
        assert_eq!(seven.previous.as_ref().unwrap().runs, 5);
        let two = intelligence
            .iter()
            .find(|item| item.handler_key == "two")
            .unwrap();
        let thirty = two
            .trends
            .iter()
            .find(|trend| trend.window == TimeWindow::Last30Days)
            .unwrap();
        assert_eq!(thirty.classification, RegressionClassification::Regression);
    }

    #[test]
    fn low_samples_and_partial_coverage_are_never_classified_as_regressions() {
        let now = 20 * 24 * HOUR_MS;
        let width = 7 * 24 * HOUR_MS;
        let values = vec![
            invocation(
                "previous",
                "one",
                TerminalStatus::Completed,
                now - width - 1,
            ),
            invocation("current", "one", TerminalStatus::Failed, now),
        ];
        let intelligence = reliability_intelligence(
            &values,
            now,
            TimeWindow::Last7Days,
            EvidenceCoverage::SyntheticFixture,
        );
        let trend = intelligence[0]
            .trends
            .iter()
            .find(|trend| trend.window == TimeWindow::Last7Days)
            .unwrap();
        assert_eq!(
            trend.availability,
            IntelligenceAvailability::InsufficientSamples
        );
        assert_eq!(
            trend.classification,
            RegressionClassification::InsufficientEvidence
        );
        let coverage_limited = reliability_intelligence(
            &values,
            now,
            TimeWindow::Last7Days,
            EvidenceCoverage::Partial,
        );
        let trend = coverage_limited[0]
            .trends
            .iter()
            .find(|trend| trend.window == TimeWindow::Last7Days)
            .unwrap();
        assert_eq!(
            trend.availability,
            IntelligenceAvailability::CoverageLimited
        );
    }

    #[test]
    fn mature_failure_outranks_one_of_one_and_equal_rates_keep_denominators() {
        let now = 20 * 24 * HOUR_MS;
        let mut values = vec![invocation("one-of-one", "one", TerminalStatus::Failed, now)];
        values.extend(repeated(
            "mature-fail",
            "mature",
            TerminalStatus::Failed,
            now - 180,
            20,
        ));
        values.extend(repeated(
            "mature-success",
            "mature",
            TerminalStatus::Completed,
            now - 80,
            80,
        ));
        values.extend(repeated(
            "same-rate-small",
            "small",
            TerminalStatus::Failed,
            now - 20,
            2,
        ));
        values.extend(repeated(
            "same-rate-small-ok",
            "small",
            TerminalStatus::Completed,
            now - 10,
            8,
        ));
        let intelligence = reliability_intelligence(
            &values,
            now,
            TimeWindow::Last7Days,
            EvidenceCoverage::SyntheticFixture,
        );
        let one = intelligence
            .iter()
            .find(|item| item.handler_key == "one")
            .unwrap();
        let mature = intelligence
            .iter()
            .find(|item| item.handler_key == "mature")
            .unwrap();
        let small = intelligence
            .iter()
            .find(|item| item.handler_key == "small")
            .unwrap();
        assert!(mature.risk.score > one.risk.score);
        assert!(mature.risk.sample_confidence_percent > small.risk.sample_confidence_percent);
        let mature_rate = mature
            .trends
            .iter()
            .find(|trend| trend.window == TimeWindow::Last7Days)
            .unwrap()
            .current
            .failure_rate_percent;
        let small_rate = small
            .trends
            .iter()
            .find(|trend| trend.window == TimeWindow::Last7Days)
            .unwrap()
            .current
            .failure_rate_percent;
        assert_eq!(mature_rate, small_rate);
        assert!(
            mature.risk.failure_rate_component_points > small.risk.failure_rate_component_points
        );
    }

    #[test]
    fn failure_clusters_use_only_known_bounded_taxonomy() {
        let now = 20 * 24 * HOUR_MS;
        let mut exit = invocation("exit", "one", TerminalStatus::Failed, now);
        exit.error_fingerprint = Some("exit_nonzero".into());
        let mut private_like = invocation("private", "one", TerminalStatus::Failed, now - 1);
        private_like.error_fingerprint = Some("must not appear in output".into());
        let timeout = invocation("timeout", "one", TerminalStatus::TimedOut, now - 2);
        let clusters = &reliability_intelligence(
            &[exit, private_like.clone(), timeout.clone()],
            now,
            TimeWindow::Last7Days,
            EvidenceCoverage::SyntheticFixture,
        )[0]
        .failure_fingerprints;
        assert!(
            clusters
                .iter()
                .any(|cluster| cluster.kind == FailureFingerprintKind::ExitNonzero)
        );
        assert!(
            clusters
                .iter()
                .any(|cluster| cluster.kind == FailureFingerprintKind::TimedOut)
        );
        assert!(
            clusters
                .iter()
                .any(|cluster| cluster.kind == FailureFingerprintKind::ExecutionFailed)
        );
        assert!(!format!("{clusters:?}").contains("must not appear"));
        let recent = recent_failures(&[private_like, timeout], now, TimeWindow::Last7Days, 10);
        assert!(
            recent
                .iter()
                .all(|failure| failure.error_fingerprint.is_none())
        );
    }

    #[test]
    fn revision_comparison_uses_adjacent_epochs_and_never_invents_previous() {
        let now = 20 * 24 * HOUR_MS;
        let mut values = repeated("old", "one", TerminalStatus::Failed, now - 10, 5);
        for value in &mut values {
            value.handler.revision = "old".into();
        }
        let mut current = repeated("current", "one", TerminalStatus::Completed, now, 5);
        for value in &mut current {
            value.handler.revision = "current".into();
        }
        values.extend(current);
        let comparison = &reliability_intelligence(
            &values,
            now + 10,
            TimeWindow::Last7Days,
            EvidenceCoverage::SyntheticFixture,
        )[0]
        .revision_comparison;
        assert_eq!(comparison.current.revision, "current");
        assert_eq!(comparison.previous.as_ref().unwrap().revision, "old");
        assert_eq!(
            comparison.classification,
            RegressionClassification::Improvement
        );
        let no_previous = reliability_intelligence(
            &values[5..],
            now + 10,
            TimeWindow::Last7Days,
            EvidenceCoverage::SyntheticFixture,
        );
        assert_eq!(
            no_previous[0].revision_comparison.availability,
            IntelligenceAvailability::InsufficientHistory
        );
        assert!(no_previous[0].revision_comparison.previous.is_none());
    }

    #[test]
    fn zero_samples_are_visible_as_insufficient_evidence() {
        let now = 20 * 24 * HOUR_MS;
        let values = vec![
            invocation(
                "previous",
                "one",
                TerminalStatus::Incomplete,
                now - 8 * 24 * HOUR_MS,
            ),
            invocation("current", "one", TerminalStatus::Unknown, now),
        ];
        let intelligence = reliability_intelligence(
            &values,
            now,
            TimeWindow::Last7Days,
            EvidenceCoverage::SyntheticFixture,
        );
        let trend = &intelligence[0]
            .trends
            .iter()
            .find(|trend| trend.window == TimeWindow::Last7Days)
            .unwrap();
        assert_eq!(trend.current.failure_sample_count, 0);
        assert_eq!(
            trend.availability,
            IntelligenceAvailability::InsufficientSamples
        );
    }
}
