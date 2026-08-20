//! Deterministic, evidence-source-neutral reliability aggregates.

use crate::domain::{HandlerIdentity, HookInvocation, TerminalStatus};
use serde::Serialize;
use std::collections::BTreeMap;

const HOUR_MS: i64 = 60 * 60 * 1_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeWindow {
    Last24Hours,
    Last7Days,
    Last30Days,
    All,
}

impl TimeWindow {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Last24Hours => "Last 24 hours",
            Self::Last7Days => "Last 7 days",
            Self::Last30Days => "Last 30 days",
            Self::All => "All time",
        }
    }
    const fn width_ms(self) -> Option<i64> {
        match self {
            Self::Last24Hours => Some(24 * HOUR_MS),
            Self::Last7Days => Some(7 * 24 * HOUR_MS),
            Self::Last30Days => Some(30 * 24 * HOUR_MS),
            Self::All => None,
        }
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
            error_fingerprint: item.error_fingerprint.clone(),
        })
        .collect::<Vec<_>>();
    failures.sort_by_key(|item| std::cmp::Reverse(item.occurred_at_unix_ms));
    failures.truncate(limit);
    failures
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
    let Some(width) = window.width_ms() else {
        return BTreeMap::new();
    };
    let current_start = now - width;
    let previous_start = current_start - width;
    let mut result = BTreeMap::new();
    for item in values.iter().filter(|item| {
        item.occurred_at_unix_ms >= previous_start && item.occurred_at_unix_ms < current_start
    }) {
        result
            .entry(item.handler.key.clone())
            .or_insert_with(|| AggregateBuilder::new(item.handler.clone()))
            .observe(item);
    }
    result
}
fn in_current_window(timestamp: i64, now: i64, window: TimeWindow) -> bool {
    window
        .width_ms()
        .is_none_or(|width| timestamp >= now - width && timestamp <= now)
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
    use crate::domain::{EvidenceCoverage, EvidenceKind, ExecutionMode, HookEvent, Runtime};
    fn handler(key: &str) -> HandlerIdentity {
        HandlerIdentity {
            key: key.into(),
            revision: "r".into(),
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
        // The previous window has two terminal samples, one failure: 50%, so
        // a boundary-correct current window produces no artificial delta.
        assert_eq!(report[0].previous_window_delta_percent, Some(0.0));
    }
}
