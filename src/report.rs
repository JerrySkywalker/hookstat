//! Deterministic machine-readable report assembly.

use crate::analytics::{HandlerAggregate, RecentFailure, TimeWindow, aggregate, recent_failures};
use crate::domain::{
    EvidenceCoverage, EvidenceKind, ExecutionMode, HandlerIdentity, HookEvent, HookInvocation,
    Runtime, SourceQualification, TerminalStatus,
};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportKind {
    InstrumentedCodex,
    SyntheticFixture,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MachineReport {
    pub schema_version: u8,
    pub report_kind: ReportKind,
    pub generated_at_unix_ms: i64,
    pub window: TimeWindow,
    pub qualification: SourceQualification,
    pub handlers: Vec<HandlerAggregate>,
    pub recent_failures: Vec<RecentFailure>,
    pub malformed_receipts: u64,
    pub incomplete_receipts: u64,
}

impl MachineReport {
    pub fn to_pretty_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

pub fn instrumented_report(
    values: &[HookInvocation],
    now: i64,
    window: TimeWindow,
    malformed_receipts: u64,
    incomplete_receipts: u64,
) -> MachineReport {
    MachineReport {
        schema_version: 1,
        report_kind: ReportKind::InstrumentedCodex,
        generated_at_unix_ms: now,
        window,
        qualification: SourceQualification::instrumented(),
        handlers: aggregate(values, now, window),
        recent_failures: recent_failures(values, now, window, 10),
        malformed_receipts,
        incomplete_receipts,
    }
}

pub fn synthetic_fixture_report(now: i64) -> MachineReport {
    let values = synthetic_fixture_invocations(now);
    MachineReport {
        schema_version: 1,
        report_kind: ReportKind::SyntheticFixture,
        generated_at_unix_ms: now,
        window: TimeWindow::Last7Days,
        qualification: SourceQualification::synthetic_fixture(),
        handlers: aggregate(&values, now, TimeWindow::Last7Days),
        recent_failures: recent_failures(&values, now, TimeWindow::Last7Days, 5),
        malformed_receipts: 0,
        incomplete_receipts: 0,
    }
}

fn fixture_handler(key: &str, event: HookEvent) -> HandlerIdentity {
    HandlerIdentity {
        key: key.into(),
        revision: "fixture-revision".into(),
        label: format!("fixture {key}"),
        source_kind: "synthetic_fixture".into(),
        event,
        matcher_identity: "any".into(),
        structural_identity: "fixture".into(),
        execution_mode: ExecutionMode::Sync,
    }
}
fn synthetic_fixture_invocations(now: i64) -> Vec<HookInvocation> {
    [
        (
            "fixture-001",
            fixture_handler("alpha", HookEvent::Stop),
            TerminalStatus::Completed,
        ),
        (
            "fixture-002",
            fixture_handler("alpha", HookEvent::Stop),
            TerminalStatus::Failed,
        ),
        (
            "fixture-003",
            fixture_handler("alpha", HookEvent::Stop),
            TerminalStatus::Blocked,
        ),
        (
            "fixture-004",
            fixture_handler("beta", HookEvent::Stop),
            TerminalStatus::Completed,
        ),
        (
            "fixture-005",
            fixture_handler("beta", HookEvent::Stop),
            TerminalStatus::Stopped,
        ),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (id, handler, status))| HookInvocation {
        source_key: "synthetic-fixture-v1".into(),
        source_record_id: id.into(),
        runtime: Runtime::Codex,
        evidence_kind: EvidenceKind::SyntheticFixture,
        coverage: EvidenceCoverage::SyntheticFixture,
        handler,
        occurred_at_unix_ms: now - index as i64 * 60_000,
        terminal_status: status,
        duration_ms: None,
        error_fingerprint: status
            .is_execution_failure()
            .then_some("fixture_failure".into()),
    })
    .collect()
}

#[cfg(test)]
pub(crate) fn synthetic_fixture_invocations_for_tui(now: i64) -> Vec<HookInvocation> {
    synthetic_fixture_invocations(now)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn instrumented_json_has_provenance_and_sample_denominator() {
        let report = instrumented_report(&[], 1_000, TimeWindow::All, 2, 1);
        let json = report.to_pretty_json().unwrap();
        assert!(json.contains("instrumented_codex"));
        assert!(json.contains("admitted_instrumented"));
    }
    #[test]
    fn fixture_keeps_same_event_handlers_distinct() {
        assert_eq!(synthetic_fixture_report(1_000).handlers.len(), 2);
    }
}
