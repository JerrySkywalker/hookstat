//! Deterministic reliability aggregates over already-admitted canonical records.

use crate::domain::{HandlerIdentity, HookInvocation, TerminalStatus};
use serde::Serialize;
use std::collections::BTreeMap;

const HOUR_MS: i64 = 60 * 60 * 1_000;

/// Report windows promised by v0.1 once a source is admitted.
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

/// Breakdown retains control-flow outcomes separately from failures.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct TerminalBreakdown {
    pub completed: u64,
    pub failed: u64,
    pub blocked: u64,
    pub stopped: u64,
    pub timed_out: u64,
    pub protocol_failure: u64,
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
        }
    }

    fn failures(&self) -> u64 {
        self.failed + self.timed_out + self.protocol_failure
    }
}

/// Exact per-handler aggregate. Every percentage is accompanied by `runs`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HandlerAggregate {
    pub handler: HandlerIdentity,
    pub runs: u64,
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

    fn observe(&mut self, invocation: &HookInvocation) {
        self.runs += 1;
        self.terminal.observe(invocation.terminal_status);
        if let Some(duration_ms) = invocation.duration_ms {
            self.durations.push(duration_ms);
        }
    }

    fn failure_rate_percent(&self) -> f64 {
        percentage(self.terminal.failures(), self.runs)
    }

    fn into_aggregate(self, previous: Option<&AggregateBuilder>) -> HandlerAggregate {
        let failure_rate_percent = self.failure_rate_percent();
        let previous_window_delta_percent = previous.map(|aggregate| {
            // Both rates are explicit percentages, so their subtraction is a
            // percentage-point delta rather than a fabricated ratio.
            failure_rate_percent - aggregate.failure_rate_percent()
        });
        let has_complete_duration_support = self.durations.len() as u64 == self.runs;
        let mut durations = self.durations;
        durations.sort_unstable();
        HandlerAggregate {
            handler: self.handler,
            runs: self.runs,
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

/// Produces a stable ranking: higher failure rate first, then handler key.
pub fn aggregate(
    invocations: &[HookInvocation],
    now_unix_ms: i64,
    window: TimeWindow,
) -> Vec<HandlerAggregate> {
    let current = collect(invocations, now_unix_ms, window);
    let previous = collect_previous(invocations, now_unix_ms, window);
    let mut result = current
        .into_values()
        .map(|builder| {
            let previous_builder = previous.get(&builder.handler.key);
            builder.into_aggregate(previous_builder)
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

/// Returns the newest bounded set of true execution failures. Blocked and
/// stopped records remain visible in terminal breakdowns but never appear here.
pub fn recent_failures(
    invocations: &[HookInvocation],
    now_unix_ms: i64,
    window: TimeWindow,
    limit: usize,
) -> Vec<RecentFailure> {
    let mut failures = invocations
        .iter()
        .filter(|invocation| {
            in_current_window(invocation.occurred_at_unix_ms, now_unix_ms, window)
                && invocation.terminal_status.is_execution_failure()
        })
        .map(|invocation| RecentFailure {
            handler: invocation.handler.clone(),
            occurred_at_unix_ms: invocation.occurred_at_unix_ms,
            terminal_status: invocation.terminal_status,
            error_fingerprint: invocation.error_fingerprint.clone(),
        })
        .collect::<Vec<_>>();
    failures.sort_by_key(|failure| std::cmp::Reverse(failure.occurred_at_unix_ms));
    failures.truncate(limit);
    failures
}

fn collect(
    invocations: &[HookInvocation],
    now_unix_ms: i64,
    window: TimeWindow,
) -> BTreeMap<String, AggregateBuilder> {
    let mut result = BTreeMap::new();
    for invocation in invocations
        .iter()
        .filter(|invocation| in_current_window(invocation.occurred_at_unix_ms, now_unix_ms, window))
    {
        result
            .entry(invocation.handler.key.clone())
            .or_insert_with(|| AggregateBuilder::new(invocation.handler.clone()))
            .observe(invocation);
    }
    result
}

fn collect_previous(
    invocations: &[HookInvocation],
    now_unix_ms: i64,
    window: TimeWindow,
) -> BTreeMap<String, AggregateBuilder> {
    let Some(width_ms) = window.width_ms() else {
        return BTreeMap::new();
    };
    let current_start = now_unix_ms - width_ms;
    let previous_start = current_start - width_ms;
    let mut result = BTreeMap::new();
    for invocation in invocations.iter().filter(|invocation| {
        invocation.occurred_at_unix_ms >= previous_start
            && invocation.occurred_at_unix_ms < current_start
    }) {
        result
            .entry(invocation.handler.key.clone())
            .or_insert_with(|| AggregateBuilder::new(invocation.handler.clone()))
            .observe(invocation);
    }
    result
}

fn in_current_window(timestamp_ms: i64, now_unix_ms: i64, window: TimeWindow) -> bool {
    match window.width_ms() {
        Some(width_ms) => timestamp_ms >= now_unix_ms - width_ms && timestamp_ms <= now_unix_ms,
        None => timestamp_ms <= now_unix_ms,
    }
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
    let index = ((sorted.len() - 1) as f64 * percentile).ceil() as usize;
    sorted[index]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{EvidenceCoverage, EvidenceKind, HookEvent, Runtime};

    fn invocation(
        id: &str,
        handler_key: &str,
        status: TerminalStatus,
        timestamp_ms: i64,
    ) -> HookInvocation {
        HookInvocation {
            source_key: "fixture-source".to_owned(),
            source_record_id: id.to_owned(),
            runtime: Runtime::Codex,
            evidence_kind: EvidenceKind::SyntheticFixture,
            coverage: EvidenceCoverage::SyntheticFixture,
            handler: HandlerIdentity {
                key: handler_key.to_owned(),
                label: handler_key.to_owned(),
                event: HookEvent::Stop,
            },
            occurred_at_unix_ms: timestamp_ms,
            terminal_status: status,
            duration_ms: None,
            error_fingerprint: None,
        }
    }

    #[test]
    fn failure_rate_excludes_blocked_and_stopped() {
        let now = 1_000_000;
        let records = vec![
            invocation("1", "one", TerminalStatus::Completed, now),
            invocation("2", "one", TerminalStatus::Failed, now),
            invocation("3", "one", TerminalStatus::Blocked, now),
            invocation("4", "one", TerminalStatus::Stopped, now),
        ];
        let report = aggregate(&records, now, TimeWindow::Last24Hours);
        assert_eq!(report[0].runs, 4);
        assert_eq!(report[0].failed_runs, 1);
        assert_eq!(report[0].failure_rate_percent, 25.0);
        assert_eq!(report[0].terminal.blocked, 1);
        assert_eq!(report[0].terminal.stopped, 1);
    }

    #[test]
    fn separate_handlers_on_same_event_are_never_conflated() {
        let now = 1_000_000;
        let records = vec![
            invocation("1", "handler-a", TerminalStatus::Completed, now),
            invocation("2", "handler-b", TerminalStatus::Failed, now),
        ];
        let report = aggregate(&records, now, TimeWindow::Last24Hours);
        assert_eq!(report.len(), 2);
        assert_eq!(report.iter().map(|item| item.runs).sum::<u64>(), 2);
    }

    #[test]
    fn malformed_or_partial_duration_support_does_not_invent_percentiles() {
        let now = 1_000_000;
        let mut first = invocation("1", "handler", TerminalStatus::Completed, now);
        first.duration_ms = Some(5);
        let second = invocation("2", "handler", TerminalStatus::Completed, now);
        let report = aggregate(&[first, second], now, TimeWindow::Last24Hours);
        assert_eq!(report[0].p50_duration_ms, None);
    }
}
