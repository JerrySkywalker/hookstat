//! Frozen-style text rendering used for deterministic preview tests. The
//! interactive Ratatui shell remains deferred until the evidence plane is real.

use crate::report::{MachineReport, ReportKind};

pub fn render_home(report: &MachineReport, width: usize) -> String {
    match report.report_kind {
        ReportKind::BlockedNoAdmittedEvidence => render_blocked(report),
        ReportKind::SyntheticFixture => render_synthetic(report, width),
    }
}

fn render_blocked(report: &MachineReport) -> String {
    format!(
        "Hook Reliability                          {}\n\n\
Codex historical evidence: NOT ADMITTED\n\
Status: BLOCKED_DATA_SOURCE_DECISION_REQUIRED\n\
Coverage: not admitted; no denominator or terminal-status claim\n\n\
No admitted Codex hook evidence was found. HookStat will not render 0.00%\n\
healthy or ingest owner session history until a durable per-handler source is\n\
qualified.\n",
        report.window.label()
    )
}

fn render_synthetic(report: &MachineReport, width: usize) -> String {
    let compact = width < 72;
    let mut output = format!(
        "Hook Reliability                          {}\n\n\
SYNTHETIC FIXTURE ONLY — NOT LOCAL CODEX HISTORY\n\
Coverage: synthetic fixture; no owner-runtime claim\n\n",
        report.window.label()
    );
    if compact {
        output.push_str("Handler                         Failed / Runs    Failure\n");
    } else {
        output.push_str("Handler                         Runs       Failed      Failure\n");
        output.push_str("────────────────────────────────────────────────────────────\n");
    }
    for aggregate in &report.handlers {
        let failure = format!(
            "{:.2}% (n={})",
            aggregate.failure_rate_percent, aggregate.runs
        );
        if compact {
            output.push_str(&format!(
                "{} / {}\n  {} / {}    {}\n",
                aggregate.handler.label,
                aggregate.handler.event.label(),
                aggregate.failed_runs,
                aggregate.runs,
                failure
            ));
        } else {
            output.push_str(&format!(
                "{:<31} {:>6} {:>12} {:>13}\n",
                format!(
                    "{} / {}",
                    aggregate.handler.label,
                    aggregate.handler.event.label()
                ),
                aggregate.runs,
                aggregate.failed_runs,
                failure
            ));
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{blocked_report, synthetic_fixture_report};

    #[test]
    fn blocked_state_explains_missing_admission_without_a_rate() {
        let rendered = render_home(&blocked_report(1_000_000), 80);
        assert!(rendered.contains("BLOCKED_DATA_SOURCE_DECISION_REQUIRED"));
        assert!(rendered.contains("will not render 0.00%"));
        assert!(rendered.contains("healthy or ingest owner session history"));
        assert!(!rendered.contains("Runs       Failed"));
    }

    #[test]
    fn synthetic_normal_and_small_renderings_keep_sample_counts() {
        let report = synthetic_fixture_report(1_000_000);
        for width in [80, 48] {
            let rendered = render_home(&report, width);
            assert!(rendered.contains("SYNTHETIC FIXTURE ONLY"));
            assert!(rendered.contains("n="));
        }
    }
}
