use super::MessageKey;

pub const fn text(key: MessageKey) -> &'static str {
    match key {
        MessageKey::AppTitle => "HookStat Reliability Center",
        MessageKey::NavOverview => "Overview",
        MessageKey::NavHooks => "Hooks",
        MessageKey::NavDiagnostics => "Diagnostics",
        MessageKey::ViewOverview => "Reliability overview",
        MessageKey::ViewHooks => "Hook reliability",
        MessageKey::ViewHookDetail => "Hook detail",
        MessageKey::ViewDiagnostics => "Read-only diagnostics",
        MessageKey::SectionNavigation => "Navigation",
        MessageKey::SectionRuntimeSummary => "Runtime summary",
        MessageKey::SectionRiskyHooks => "Risky hooks",
        MessageKey::SectionRecentFailures => "Recent failures",
        MessageKey::SectionTerminalBreakdown => "Terminal breakdown",
        MessageKey::SectionTimeline => "Timeline",
        MessageKey::SectionDiagnostics => "Operational checks",
        MessageKey::FieldRuntime => "Runtime",
        MessageKey::FieldCoverage => "Coverage",
        MessageKey::FieldTotalRuns => "Total runs",
        MessageKey::FieldRunCount => "Run count",
        MessageKey::FieldFailureRate => "Failure rate",
        MessageKey::FieldHealth => "Health",
        MessageKey::FieldEvent => "Event",
        MessageKey::FieldInternalIdentity => "Internal identity",
        MessageKey::FieldRevision => "Revision",
        MessageKey::FieldSuccesses => "Successes",
        MessageKey::FieldFailures => "Failures",
        MessageKey::FieldSamples => "Terminal samples",
        MessageKey::FieldWindow => "Window",
        MessageKey::FieldSearch => "Search",
        MessageKey::FieldFilter => "Filter",
        MessageKey::FieldSort => "Sort",
        MessageKey::FieldIncompleteReceipts => "Incomplete receipts",
        MessageKey::FieldMalformedReceipts => "Malformed receipts",
        MessageKey::ColumnName => "Name",
        MessageKey::ColumnEvent => "Event",
        MessageKey::ColumnRuntime => "Runtime",
        MessageKey::ColumnFailureRate => "Failure rate",
        MessageKey::ColumnTrend => "Trend",
        MessageKey::StateLoading => "Loading accepted reliability data…",
        MessageKey::StateEmpty => "No admitted receipt rows yet. This is not 0.00% healthy.",
        MessageKey::StateEmptySearch => "No hooks match the active search or filter.",
        MessageKey::StateRefreshFailed => "Refresh failed; accepted history retained.",
        MessageKey::StateTimelineUnavailable => {
            "Timeline is unavailable until reliability intelligence is admitted."
        }
        MessageKey::StateNoRecentFailures => "No recent execution failures in this window.",
        MessageKey::StatusHealthy => "✓ Healthy",
        MessageKey::StatusDegraded => "! Degraded",
        MessageKey::StatusCoverageLimited => "! Coverage limited",
        MessageKey::StatusNoTerminalSamples => "! No terminal samples",
        MessageKey::StatusUnavailable => "Unavailable",
        MessageKey::DiagnosticHealthy => "Pass",
        MessageKey::DiagnosticWarning => "Attention",
        MessageKey::DiagnosticUnavailable => "Not inspected",
        MessageKey::DiagnosticRuntimeSnapshot => "Runtime snapshot",
        MessageKey::DiagnosticEvidenceCoverage => "Evidence coverage",
        MessageKey::DiagnosticReceiptIntegrity => "Receipt integrity",
        MessageKey::DiagnosticInstrumentation => "Instrumentation",
        MessageKey::DiagnosticTrust => "Trust",
        MessageKey::DiagnosticReceiptStorage => "Receipt storage",
        MessageKey::DiagnosticRuntimeSnapshotExplanation => {
            "Shown from the accepted reliability snapshot; no runtime probe is run."
        }
        MessageKey::DiagnosticEvidenceCoverageExplanation => {
            "Coverage is reported as admitted and is never treated as healthy zero by default."
        }
        MessageKey::DiagnosticReceiptIntegrityExplanation => {
            "Counts come from the accepted snapshot; malformed or incomplete receipts remain visible."
        }
        MessageKey::DiagnosticInstrumentationExplanation => {
            "Not inspected in G01. This view does not read or change Codex configuration."
        }
        MessageKey::DiagnosticTrustExplanation => {
            "Not inspected in G01. This view does not query or change Codex trust."
        }
        MessageKey::DiagnosticReceiptStorageExplanation => {
            "Storage probing is deferred to G04; this view performs no filesystem inspection."
        }
        MessageKey::CoverageComplete => "Complete",
        MessageKey::CoveragePartial => "Partial",
        MessageKey::CoverageSyncOnly => "Sync only",
        MessageKey::CoverageBestEffort => "Best effort",
        MessageKey::CoverageUnknown => "Unknown",
        MessageKey::CoverageNotAdmitted => "Not admitted",
        MessageKey::CoverageSyntheticFixture => "Synthetic fixture",
        MessageKey::WindowLast24Hours => "Last 24 hours",
        MessageKey::WindowLast7Days => "Last 7 days",
        MessageKey::WindowLast30Days => "Last 30 days",
        MessageKey::WindowAll => "All time",
        MessageKey::RuntimeCodex => "Codex",
        MessageKey::RuntimeDeepSeekHarness => "DeepSeek Harness",
        MessageKey::RuntimeOpenCode => "OpenCode",
        MessageKey::EventSessionStart => "Session start",
        MessageKey::EventSessionEnd => "Session end",
        MessageKey::EventUserPromptSubmit => "Prompt submit",
        MessageKey::EventPreToolUse => "Before tool use",
        MessageKey::EventPostToolUse => "After tool use",
        MessageKey::EventPermissionRequest => "Permission request",
        MessageKey::EventPreCompact => "Before compaction",
        MessageKey::EventPostCompact => "After compaction",
        MessageKey::EventStop => "Stop",
        MessageKey::EventSubagentStart => "Subagent start",
        MessageKey::EventSubagentStop => "Subagent stop",
        MessageKey::TerminalCompleted => "Completed",
        MessageKey::TerminalFailed => "Failed",
        MessageKey::TerminalBlocked => "Blocked",
        MessageKey::TerminalStopped => "Stopped",
        MessageKey::TerminalTimedOut => "Timed out",
        MessageKey::TerminalProtocolFailure => "Protocol failure",
        MessageKey::TerminalIncomplete => "Incomplete",
        MessageKey::TerminalUnknown => "Unknown",
        MessageKey::SortFailureRate => "Failure rate",
        MessageKey::SortName => "Name",
        MessageKey::SortRuns => "Run count",
        MessageKey::FilterFailuresOnly => "Failures only",
        MessageKey::FilterAllHooks => "All hooks",
        MessageKey::IdentityHook => "hook",
        MessageKey::RateWithSample => "{rate}% ({samples})",
        MessageKey::SampleCount => "n={count}",
        MessageKey::FooterNavigate => "navigate",
        MessageKey::FooterOpen => "open",
        MessageKey::FooterBack => "back",
        MessageKey::FooterRefresh => "refresh",
        MessageKey::FooterQuit => "quit",
        MessageKey::FooterFocusContent => "content",
        MessageKey::FooterFocusNavigation => "navigation",
        MessageKey::FooterSearch => "search",
        MessageKey::FooterFilter => "filter",
        MessageKey::FooterSort => "sort",
        MessageKey::MinimumTerminal => "Resize to at least 24x10",
    }
}
