//! Deterministic JSON report assembly. Reports carry their admission state so a
//! synthetic fixture can never masquerade as owner runtime history.

use crate::analytics::{HandlerAggregate, RecentFailure, TimeWindow, aggregate, recent_failures};
use crate::domain::{
    EvidenceCoverage, EvidenceKind, HandlerIdentity, HookEvent, HookInvocation, Runtime,
    SourceQualification, TerminalStatus,
};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportKind {
    BlockedNoAdmittedEvidence,
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
}

impl MachineReport {
    pub fn to_pretty_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// The only report available without an admitted source. It has no rate rows.
pub fn blocked_report(now_unix_ms: i64) -> MachineReport {
    MachineReport {
        schema_version: 1,
        report_kind: ReportKind::BlockedNoAdmittedEvidence,
        generated_at_unix_ms: now_unix_ms,
        window: TimeWindow::Last7Days,
        qualification: SourceQualification::blocked(),
        handlers: Vec::new(),
        recent_failures: Vec::new(),
    }
}

/// A stable development fixture that proves the canonical aggregate path only.
/// It is explicitly labelled synthetic in both JSON and terminal rendering.
pub fn synthetic_fixture_report(now_unix_ms: i64) -> MachineReport {
    let records = synthetic_fixture_invocations(now_unix_ms);
    let window = TimeWindow::Last7Days;
    MachineReport {
        schema_version: 1,
        report_kind: ReportKind::SyntheticFixture,
        generated_at_unix_ms: now_unix_ms,
        window,
        qualification: SourceQualification::synthetic_fixture(),
        handlers: aggregate(&records, now_unix_ms, window),
        recent_failures: recent_failures(&records, now_unix_ms, window, 5),
    }
}

fn synthetic_fixture_invocations(now_unix_ms: i64) -> Vec<HookInvocation> {
    let alpha = HandlerIdentity {
        key: "fixture:stop:alpha".to_owned(),
        label: "fixture alpha".to_owned(),
        event: HookEvent::Stop,
    };
    let beta = HandlerIdentity {
        key: "fixture:stop:beta".to_owned(),
        label: "fixture beta".to_owned(),
        event: HookEvent::Stop,
    };
    [
        (
            "fixture-001",
            alpha.clone(),
            TerminalStatus::Completed,
            None,
        ),
        (
            "fixture-002",
            alpha.clone(),
            TerminalStatus::Failed,
            Some("E_FIXTURE"),
        ),
        ("fixture-003", alpha, TerminalStatus::Blocked, None),
        ("fixture-004", beta.clone(), TerminalStatus::Completed, None),
        ("fixture-005", beta, TerminalStatus::Stopped, None),
    ]
    .into_iter()
    .enumerate()
    .map(
        |(index, (source_record_id, handler, terminal_status, error_fingerprint))| HookInvocation {
            source_key: "synthetic-fixture-v1".to_owned(),
            source_record_id: source_record_id.to_owned(),
            runtime: Runtime::Codex,
            evidence_kind: EvidenceKind::SyntheticFixture,
            coverage: EvidenceCoverage::SyntheticFixture,
            handler,
            occurred_at_unix_ms: now_unix_ms - (index as i64 * 60_000),
            terminal_status,
            duration_ms: None,
            error_fingerprint: error_fingerprint.map(str::to_owned),
        },
    )
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::EvidenceAdmission;

    #[test]
    fn blocked_report_has_no_healthy_zero_rows() {
        let report = blocked_report(1_000_000);
        assert!(report.handlers.is_empty());
        assert_eq!(
            report.qualification.admission,
            EvidenceAdmission::BlockedDataSourceDecisionRequired
        );
        assert!(
            report
                .to_pretty_json()
                .unwrap()
                .contains("blocked_no_admitted_evidence")
        );
    }

    #[test]
    fn synthetic_report_labels_its_provenance_and_preserves_handler_split() {
        let report = synthetic_fixture_report(1_000_000);
        assert_eq!(report.handlers.len(), 2);
        assert_eq!(report.report_kind, ReportKind::SyntheticFixture);
        assert!(
            report
                .to_pretty_json()
                .unwrap()
                .contains("synthetic_fixture")
        );
    }
}
