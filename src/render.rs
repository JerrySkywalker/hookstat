//! Deterministic frozen-style text rendering shared by CLI tests and TUI data.

use crate::report::{MachineReport, ReportKind};

pub fn render_home(report: &MachineReport, width: usize) -> String {
    let compact = width < 72;
    let provenance = match report.report_kind {
        ReportKind::InstrumentedCodex => "INSTRUMENTED CODEX RECEIPTS — opt-in, partial coverage",
        ReportKind::SyntheticFixture => "SYNTHETIC FIXTURE ONLY — not owner runtime history",
    };
    let mut output = format!(
        "Hook Reliability                          {}\n\n{}\nCoverage: {:?}; incomplete={} malformed={}\n\n",
        report.window.label(),
        provenance,
        report.qualification.coverage,
        report.incomplete_receipts,
        report.malformed_receipts
    );
    if report.handlers.is_empty() {
        output.push_str(
            "No admitted receipt rows yet. Partial or absent coverage is not 0.00% healthy.\n",
        );
        return output;
    }
    if compact {
        output.push_str("Handler                         Failed / Samples    Failure\n");
    } else {
        output.push_str("Handler                         Runs       Failed      Failure\n────────────────────────────────────────────────────────────\n");
    }
    for item in &report.handlers {
        let failure = format!(
            "{:.2}% (n={})",
            item.failure_rate_percent, item.failure_sample_count
        );
        let name = format!("{} / {}", item.handler.label, item.handler.event.label());
        if compact {
            output.push_str(&format!(
                "{name}\n  {} / {}    {failure}\n",
                item.failed_runs, item.failure_sample_count
            ));
        } else {
            output.push_str(&format!(
                "{name:<31} {:>6} {:>12} {:>13}\n",
                item.runs, item.failed_runs, failure
            ));
        }
        if item.terminal.incomplete > 0 || item.terminal.unknown > 0 {
            output.push_str(&format!(
                "  coverage: incomplete={} unknown={}\n",
                item.terminal.incomplete, item.terminal.unknown
            ));
        }
    }
    output
}

pub fn render_detail(report: &MachineReport, index: usize) -> String {
    let Some(item) = report.handlers.get(index) else {
        return "No handler selected.\n".into();
    };
    let mut output = format!(
        "{} · {} · Codex\n\n",
        item.handler.label,
        item.handler.event.label()
    );
    output.push_str(&format!(
        "{}       {} / {}        {:.2}% (n={})\n",
        report.window.label(),
        item.failed_runs,
        item.failure_sample_count,
        item.failure_rate_percent,
        item.failure_sample_count
    ));
    output.push_str(&format!(
        "\nCompleted={} Failed={} Blocked={} Stopped={} TimedOut={} ProtocolFailure={}\n",
        item.terminal.completed,
        item.terminal.failed,
        item.terminal.blocked,
        item.terminal.stopped,
        item.terminal.timed_out,
        item.terminal.protocol_failure
    ));
    if item.terminal.incomplete > 0 || item.terminal.unknown > 0 {
        output.push_str(&format!(
            "Coverage warning: incomplete={} unknown={}; not healthy zero.\n",
            item.terminal.incomplete, item.terminal.unknown
        ));
    }
    if let Some(p50) = item.p50_duration_ms {
        output.push_str(&format!(
            "\np50 {p50} ms\np95 {} ms\np99 {} ms\n",
            item.p95_duration_ms.unwrap_or(p50),
            item.p99_duration_ms.unwrap_or(p50)
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics::TimeWindow;
    use crate::report::{instrumented_report, synthetic_fixture_report};
    #[test]
    fn empty_instrumented_state_is_not_healthy_zero() {
        let rendered = render_home(&instrumented_report(&[], 1_000, TimeWindow::All, 0, 0), 80);
        assert!(rendered.contains("not 0.00% healthy"));
    }
    #[test]
    fn sample_counts_survive_normal_and_small_rendering() {
        let report = synthetic_fixture_report(1_000_000);
        for width in [80, 48] {
            assert!(render_home(&report, width).contains("n="));
        }
    }
}
