use super::MessageKey;

pub const fn text(key: MessageKey) -> &'static str {
    match key {
        MessageKey::AppTitle => "HookStat Reliability Center",
        MessageKey::NavOverview => "Overview",
        MessageKey::NavHooks => "Hooks",
        MessageKey::NavChanges => "Changes",
        MessageKey::NavDiagnostics => "Diagnostics",
        MessageKey::NavSettings => "Settings",
        MessageKey::ViewOverview => "Reliability overview",
        MessageKey::ViewHooks => "Hook catalog",
        MessageKey::ViewChanges => "Changes and history",
        MessageKey::ViewChangeDetail => "Change evidence",
        MessageKey::ViewHookDetail => "Hook detail",
        MessageKey::ViewFailureClusters => "Failure clusters",
        MessageKey::ViewFailureClusterDetail => "Failure cluster detail",
        MessageKey::ViewDiagnostics => "Read-only diagnostics",
        MessageKey::ViewSettings => "Interface settings",
        MessageKey::SectionNavigation => "Sections",
        MessageKey::SectionRuntimeSummary => "Runtime summary",
        MessageKey::SectionRiskyHooks => "Risky hooks",
        MessageKey::SectionChanges => "Recent changes",
        MessageKey::SectionRecentFailures => "Recent failures",
        MessageKey::SectionTerminalBreakdown => "Terminal breakdown",
        MessageKey::SectionTimeline => "Timeline",
        MessageKey::SectionIntelligence => "Reliability intelligence",
        MessageKey::SectionTrends => "Trends",
        MessageKey::SectionRevisionComparison => "Revision comparison",
        MessageKey::SectionFailureFingerprints => "Failure fingerprints",
        MessageKey::SectionAlias => "Human alias",
        MessageKey::SectionDiagnostics => "Operational checks",
        MessageKey::SectionInterface => "Human interface",
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
        MessageKey::FieldLanguage => "Language",
        MessageKey::FieldSavedLanguage => "Saved language",
        MessageKey::FieldColor => "Color",
        MessageKey::FieldSavedColor => "Saved color",
        MessageKey::FieldRisk => "Risk",
        MessageKey::FieldRiskScore => "Risk score",
        MessageKey::FieldConfidence => "Sample confidence",
        MessageKey::FieldClassification => "Classification",
        MessageKey::FieldPreviousPeriod => "Previous period",
        MessageKey::FieldRecency => "Recency",
        MessageKey::FieldImpact => "Impact",
        MessageKey::FieldFirstSeen => "First seen",
        MessageKey::FieldLastSeen => "Last seen",
        MessageKey::FieldLatestEvidence => "Latest evidence",
        MessageKey::FieldAlias => "Alias",
        MessageKey::FieldAffectedHooks => "Affected hooks",
        MessageKey::FieldOccurrences => "Occurrences",
        MessageKey::FieldDataFreshness => "Data freshness",
        MessageKey::FieldRevisionCount => "Historical revisions",
        MessageKey::FieldObservationStatus => "Observation status",
        MessageKey::FieldMetricScope => "Metric scope",
        MessageKey::FieldChangeOccurred => "Change occurred",
        MessageKey::FieldReason => "Reason",
        MessageKey::ColumnName => "Name",
        MessageKey::ColumnEvent => "Event",
        MessageKey::ColumnRuntime => "Runtime",
        MessageKey::ColumnFailureRate => "Failure rate",
        MessageKey::ColumnTrend => "Trend",
        MessageKey::ColumnRisk => "Risk",
        MessageKey::StateLoading => "Loading accepted reliability data…",
        MessageKey::StateEmpty => "No admitted receipt rows yet. This is not 0.00% healthy.",
        MessageKey::StateEmptySearch => "No hooks match the active search or filter.",
        MessageKey::StateRefreshFailed => "Refresh failed; accepted history retained.",
        MessageKey::StateTimelineUnavailable => {
            "Timeline is unavailable until reliability intelligence is admitted."
        }
        MessageKey::StateInsufficientHistory => "Insufficient history",
        MessageKey::StateInsufficientSamples => "Insufficient samples",
        MessageKey::StateCoverageLimited => "Coverage limited",
        MessageKey::StateNoRecentFailures => "No recent execution failures in this window.",
        MessageKey::StateHistoricalOnly => {
            "Historical outside this period; this is not an inactive or removed-hook claim."
        }
        MessageKey::StatePreferenceClean => "No pending language change.",
        MessageKey::StatePreferenceDirty => "Language change is staged; apply to persist it.",
        MessageKey::StatePreferenceSaved => "Language preference saved locally.",
        MessageKey::StatePreferenceConflict => {
            "Language preference changed elsewhere; apply was refused."
        }
        MessageKey::StatePreferenceSaveFailed => "Language preference could not be saved safely.",
        MessageKey::StateAliasClean => "No pending alias change.",
        MessageKey::StateAliasDirty => "Alias change is staged; apply to persist it.",
        MessageKey::StateAliasSaved => "Alias saved in HookStat presentation metadata.",
        MessageKey::StateAliasConflict => "Alias changed elsewhere; apply was rejected.",
        MessageKey::StateAliasSaveFailed => "Alias could not be saved safely.",
        MessageKey::StateObservedInSelectedPeriod => "Observed in selected period",
        MessageKey::StatusHealthy => "✓ Healthy",
        MessageKey::StatusDegraded => "! Degraded",
        MessageKey::StatusCoverageLimited => "! Coverage limited",
        MessageKey::StatusNoTerminalSamples => "! No terminal samples",
        MessageKey::StatusUnavailable => "Unavailable",
        MessageKey::StatusRegression => "↑ Regression",
        MessageKey::StatusImprovement => "↓ Improvement",
        MessageKey::StatusStable => "→ Stable",
        MessageKey::StatusInsufficientEvidence => "Insufficient evidence",
        MessageKey::ChangeRegression => "Regression",
        MessageKey::ChangeRecovery => "Recovery",
        MessageKey::ChangeRevision => "Revision change",
        MessageKey::ChangeNewHook => "New admitted hook",
        MessageKey::DiagnosticPass => "Pass",
        MessageKey::DiagnosticWarning => "Attention",
        MessageKey::DiagnosticFail => "Fail",
        MessageKey::DiagnosticUnknown => "Unknown",
        MessageKey::DiagnosticUnsupported => "Unsupported",
        MessageKey::DiagnosticHookStatBinary => "HookStat binary",
        MessageKey::DiagnosticCodexBinary => "Codex binary",
        MessageKey::DiagnosticEffectiveRuntime => "Effective runtime",
        MessageKey::DiagnosticInstrumentation => "Instrumentation",
        MessageKey::DiagnosticTrust => "Trust",
        MessageKey::DiagnosticReceiptSpool => "Receipt spool",
        MessageKey::DiagnosticLedger => "SQLite ledger",
        MessageKey::DiagnosticReceiptIntegrity => "Receipt integrity",
        MessageKey::DiagnosticEvidenceCoverage => "Evidence coverage",
        MessageKey::DiagnosticPathIdentity => "Windows PATH identity",
        MessageKey::DiagnosticEvidenceFreshness => "Latest evidence",
        MessageKey::DiagnosticHookStatBinaryExplanation => {
            "Confirms the running HookStat build only."
        }
        MessageKey::DiagnosticCodexBinaryExplanation => {
            "Runs a bounded Codex version check; no configuration is changed."
        }
        MessageKey::DiagnosticEffectiveRuntimeExplanation => {
            "Uses Codex's read-only effective hook view when available."
        }
        MessageKey::DiagnosticInstrumentationExplanation => {
            "Reads supported configuration only; it never applies or repairs hooks."
        }
        MessageKey::DiagnosticTrustExplanation => {
            "Reports the read-only effective trust state and never writes trust."
        }
        MessageKey::DiagnosticReceiptSpoolExplanation => {
            "Checks an existing spool without creating, repairing, or writing records."
        }
        MessageKey::DiagnosticLedgerExplanation => {
            "Opens an existing ledger read-only; missing state is never treated as healthy."
        }
        MessageKey::DiagnosticReceiptIntegrityExplanation => {
            "Malformed and incomplete receipt counts remain explicit."
        }
        MessageKey::DiagnosticEvidenceCoverageExplanation => {
            "Supported and unsupported coverage stays visible; unknown is never healthy."
        }
        MessageKey::DiagnosticPathIdentityExplanation => {
            "Relevant only for Windows proxy execution; other platforms are unsupported."
        }
        MessageKey::DiagnosticEvidenceFreshnessExplanation => {
            "Measures only the latest sanitized evidence timestamp when one exists."
        }
        MessageKey::DiagnosticHandlerCounts => {
            "{discovered} discovered / {instrumented} instrumented / {unsupported} unsupported"
        }
        MessageKey::DiagnosticEvidenceAgeMinutes => "{minutes} min ago",
        MessageKey::LanguageAuto => "Auto",
        MessageKey::LanguageEnUs => "English (en-US)",
        MessageKey::LanguageZhCn => "Simplified Chinese (zh-CN)",
        MessageKey::ColorAuto => "Auto",
        MessageKey::ColorAlways => "Always",
        MessageKey::ColorNever => "Never",
        MessageKey::CoverageComplete => "Complete",
        MessageKey::CoveragePartial => "Partial",
        MessageKey::CoverageSyncOnly => "Sync only",
        MessageKey::CoverageBestEffort => "Best effort",
        MessageKey::CoverageUnknown => "Unknown",
        MessageKey::CoverageNotAdmitted => "Not admitted",
        MessageKey::CoverageSyntheticFixture => "Synthetic fixture",
        MessageKey::WindowToday => "Today",
        MessageKey::WindowLast24Hours => "Last 24 hours",
        MessageKey::WindowLast7Days => "Last 7 days",
        MessageKey::WindowLast30Days => "Last 30 days",
        MessageKey::WindowAll => "All time",
        MessageKey::PeriodAll => "All",
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
        MessageKey::SortRisk => "Risk score",
        MessageKey::FilterFailuresOnly => "Failures only",
        MessageKey::FilterAllHooks => "All hooks",
        MessageKey::IdentityHook => "hook",
        MessageKey::RateWithSample => "{rate}% ({samples})",
        MessageKey::SampleCount => "n={count}",
        MessageKey::FingerprintExitNonzero => "Non-zero exit",
        MessageKey::FingerprintTimedOut => "Timed out",
        MessageKey::FingerprintProtocolFailure => "Protocol failure",
        MessageKey::FingerprintExecutionFailed => "Execution failed",
        MessageKey::FooterNavigate => "navigate",
        MessageKey::FooterPage => "page",
        MessageKey::FooterOpen => "open",
        MessageKey::FooterHelp => "help",
        MessageKey::FooterSelect => "select",
        MessageKey::FooterEdit => "edit",
        MessageKey::FooterBack => "back",
        MessageKey::FooterRefresh => "refresh",
        MessageKey::FooterQuit => "quit",
        MessageKey::FooterFocusContent => "content",
        MessageKey::FooterFocusNavigation => "navigation",
        MessageKey::FooterSearch => "search",
        MessageKey::FooterFilter => "filter",
        MessageKey::FooterSort => "sort",
        MessageKey::FooterChange => "change",
        MessageKey::FooterApply => "apply",
        MessageKey::FooterRevert => "revert",
        MessageKey::FooterCancel => "cancel",
        MessageKey::FooterDiscard => "discard",
        MessageKey::FooterDismiss => "dismiss",
        MessageKey::HelpTitle => "Help",
        MessageKey::HelpNavigation => {
            "↑↓ or j/k changes pages directly. Enter opens the current local interaction."
        }
        MessageKey::HelpPeriods => "Periods: t Today, 1 24h, 7 7d, 3 30d, a All.",
        MessageKey::HelpHooks => {
            "Hook Catalog: Enter selects rows; / search, f filter, s sort; e edits a Human alias in detail."
        }
        MessageKey::HelpChanges => {
            "Changes: Enter selects events, then opens evidence and the ordered revision timeline. Historical rows never prove a hook was removed."
        }
        MessageKey::HelpDetail => {
            "Hook detail: f opens safe failure clusters; ↑↓ scroll, PgUp/PgDn page, Esc returns to Catalog."
        }
        MessageKey::HelpSettings => {
            "Settings: Enter edit/done, ↑↓ field, ←→ draft, a Apply, r Revert; q confirms dirty discard."
        }
        MessageKey::HelpRefresh => {
            "r refreshes reliability pages. All displayed failure rates include sample counts."
        }
        MessageKey::MinimumTerminal => "Resize to at least 24x10",
        MessageKey::FailureRateUnavailableZeroSamples => {
            "unavailable (0 terminal samples; no terminal samples)"
        }
        MessageKey::ScopeSelectedAllRevisions => "Selected {period}, all revisions",
        MessageKey::ScopePeriodAllRevisions => "{period}, all revisions",
        MessageKey::ScopeAllObservedAllRevisions => "All observed time, all revisions",
        MessageKey::ScopeAllObservedRevisionComparison => {
            "All observed time, current/previous revision"
        }
        MessageKey::ScopeTerminalSamples => "{samples} in selected scope",
        MessageKey::CoverageExplanationComplete => "Terminal evidence is complete in this scope.",
        MessageKey::CoverageExplanationPartial => {
            "Some evidence is observed; terminal coverage is incomplete."
        }
        MessageKey::CoverageExplanationSyncOnly => "Only synchronous observations are covered.",
        MessageKey::CoverageExplanationBestEffort => {
            "Evidence is best effort and may be incomplete."
        }
        MessageKey::CoverageExplanationUnknown => {
            "Coverage is unknown; reliability is not a health claim."
        }
        MessageKey::CoverageExplanationNotAdmitted => {
            "No admitted evidence source covers this hook."
        }
        MessageKey::CoverageExplanationSyntheticFixture => {
            "Synthetic fixture coverage; not live runtime evidence."
        }
        MessageKey::RiskLow => "Low risk",
        MessageKey::RiskGuarded => "Guarded risk",
        MessageKey::RiskElevated => "Elevated risk",
        MessageKey::RiskHigh => "High risk",
        MessageKey::RiskReasonFailures => "observed execution failures in selected scope.",
        MessageKey::RiskReasonNoTerminalSamples => {
            "no terminal samples; this is not a healthy result."
        }
        MessageKey::RiskReasonComplete => "no observed failures in selected scope.",
        MessageKey::RiskReasonIncomplete => {
            "no observed failures; terminal coverage is incomplete."
        }
        MessageKey::TimeUnavailable => "Time unavailable",
        MessageKey::TimeJustNow => "just now",
        MessageKey::TimeMinuteAgo => "1 minute ago",
        MessageKey::TimeMinutesAgo => "{count} minutes ago",
        MessageKey::TimeHourAgo => "1 hour ago",
        MessageKey::TimeHoursAgo => "{count} hours ago",
        MessageKey::TimeDayAgo => "1 day ago",
        MessageKey::TimeDaysAgo => "{count} days ago",
    }
}
