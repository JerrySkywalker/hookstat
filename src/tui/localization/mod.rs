//! Typed locale lookup for the Reliability Center's Human presentation.

mod en_us;
mod zh_cn;

use crate::analytics::{
    FailureFingerprintKind, IntelligenceAvailability, RegressionClassification, TimeWindow,
};
use crate::domain::{EvidenceCoverage, HookEvent, Runtime, TerminalStatus};
use crate::tui::view_model::{DiagnosticCheckId, DiagnosticStatus, Health, HookSort};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InterfaceLanguage {
    Auto,
    EnUs,
    ZhCn,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolvedLocale {
    EnUs,
    ZhCn,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocaleSource {
    Explicit,
    Environment,
    Preference,
    System,
    Fallback,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LanguageState {
    pub requested: InterfaceLanguage,
    pub resolved: ResolvedLocale,
    pub source: LocaleSource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MessageKey {
    AppTitle,
    NavOverview,
    NavHooks,
    NavDiagnostics,
    NavSettings,
    ViewOverview,
    ViewHooks,
    ViewHookDetail,
    ViewDiagnostics,
    ViewSettings,
    SectionNavigation,
    SectionRuntimeSummary,
    SectionRiskyHooks,
    SectionRecentFailures,
    SectionTerminalBreakdown,
    SectionTimeline,
    SectionIntelligence,
    SectionTrends,
    SectionRevisionComparison,
    SectionFailureFingerprints,
    SectionDiagnostics,
    SectionInterface,
    FieldRuntime,
    FieldCoverage,
    FieldTotalRuns,
    FieldRunCount,
    FieldFailureRate,
    FieldHealth,
    FieldEvent,
    FieldInternalIdentity,
    FieldRevision,
    FieldSuccesses,
    FieldFailures,
    FieldSamples,
    FieldWindow,
    FieldSearch,
    FieldFilter,
    FieldSort,
    FieldIncompleteReceipts,
    FieldMalformedReceipts,
    FieldLanguage,
    FieldSavedLanguage,
    FieldColor,
    FieldSavedColor,
    FieldRisk,
    FieldRiskScore,
    FieldConfidence,
    FieldClassification,
    FieldPreviousPeriod,
    FieldRecency,
    FieldImpact,
    ColumnName,
    ColumnEvent,
    ColumnRuntime,
    ColumnFailureRate,
    ColumnTrend,
    ColumnRisk,
    StateLoading,
    StateEmpty,
    StateEmptySearch,
    StateRefreshFailed,
    StateTimelineUnavailable,
    StateInsufficientHistory,
    StateInsufficientSamples,
    StateCoverageLimited,
    StateNoRecentFailures,
    StatePreferenceClean,
    StatePreferenceDirty,
    StatePreferenceSaved,
    StatePreferenceConflict,
    StatePreferenceSaveFailed,
    StatusHealthy,
    StatusDegraded,
    StatusCoverageLimited,
    StatusNoTerminalSamples,
    StatusUnavailable,
    StatusRegression,
    StatusImprovement,
    StatusStable,
    StatusInsufficientEvidence,
    DiagnosticPass,
    DiagnosticWarning,
    DiagnosticFail,
    DiagnosticUnknown,
    DiagnosticUnsupported,
    DiagnosticHookStatBinary,
    DiagnosticCodexBinary,
    DiagnosticEffectiveRuntime,
    DiagnosticInstrumentation,
    DiagnosticTrust,
    DiagnosticReceiptSpool,
    DiagnosticLedger,
    DiagnosticReceiptIntegrity,
    DiagnosticEvidenceCoverage,
    DiagnosticPathIdentity,
    DiagnosticEvidenceFreshness,
    DiagnosticHookStatBinaryExplanation,
    DiagnosticCodexBinaryExplanation,
    DiagnosticEffectiveRuntimeExplanation,
    DiagnosticInstrumentationExplanation,
    DiagnosticTrustExplanation,
    DiagnosticReceiptSpoolExplanation,
    DiagnosticLedgerExplanation,
    DiagnosticReceiptIntegrityExplanation,
    DiagnosticEvidenceCoverageExplanation,
    DiagnosticPathIdentityExplanation,
    DiagnosticEvidenceFreshnessExplanation,
    DiagnosticHandlerCounts,
    DiagnosticEvidenceAgeMinutes,
    LanguageAuto,
    LanguageEnUs,
    LanguageZhCn,
    ColorAuto,
    ColorAlways,
    ColorNever,
    CoverageComplete,
    CoveragePartial,
    CoverageSyncOnly,
    CoverageBestEffort,
    CoverageUnknown,
    CoverageNotAdmitted,
    CoverageSyntheticFixture,
    WindowLast24Hours,
    WindowLast7Days,
    WindowLast30Days,
    WindowAll,
    RuntimeCodex,
    RuntimeDeepSeekHarness,
    RuntimeOpenCode,
    EventSessionStart,
    EventSessionEnd,
    EventUserPromptSubmit,
    EventPreToolUse,
    EventPostToolUse,
    EventPermissionRequest,
    EventPreCompact,
    EventPostCompact,
    EventStop,
    EventSubagentStart,
    EventSubagentStop,
    TerminalCompleted,
    TerminalFailed,
    TerminalBlocked,
    TerminalStopped,
    TerminalTimedOut,
    TerminalProtocolFailure,
    TerminalIncomplete,
    TerminalUnknown,
    SortFailureRate,
    SortName,
    SortRuns,
    SortRisk,
    FilterFailuresOnly,
    FilterAllHooks,
    IdentityHook,
    RateWithSample,
    SampleCount,
    FingerprintExitNonzero,
    FingerprintTimedOut,
    FingerprintProtocolFailure,
    FingerprintExecutionFailed,
    FooterNavigate,
    FooterPage,
    FooterOpen,
    FooterBack,
    FooterRefresh,
    FooterQuit,
    FooterFocusContent,
    FooterFocusNavigation,
    FooterSearch,
    FooterFilter,
    FooterSort,
    FooterChange,
    FooterApply,
    FooterRevert,
    MinimumTerminal,
}

#[cfg(test)]
pub const ALL_MESSAGE_KEYS: &[MessageKey] = &[
    MessageKey::AppTitle,
    MessageKey::NavOverview,
    MessageKey::NavHooks,
    MessageKey::NavDiagnostics,
    MessageKey::NavSettings,
    MessageKey::ViewOverview,
    MessageKey::ViewHooks,
    MessageKey::ViewHookDetail,
    MessageKey::ViewDiagnostics,
    MessageKey::ViewSettings,
    MessageKey::SectionNavigation,
    MessageKey::SectionRuntimeSummary,
    MessageKey::SectionRiskyHooks,
    MessageKey::SectionRecentFailures,
    MessageKey::SectionTerminalBreakdown,
    MessageKey::SectionTimeline,
    MessageKey::SectionIntelligence,
    MessageKey::SectionTrends,
    MessageKey::SectionRevisionComparison,
    MessageKey::SectionFailureFingerprints,
    MessageKey::SectionDiagnostics,
    MessageKey::SectionInterface,
    MessageKey::FieldRuntime,
    MessageKey::FieldCoverage,
    MessageKey::FieldTotalRuns,
    MessageKey::FieldRunCount,
    MessageKey::FieldFailureRate,
    MessageKey::FieldHealth,
    MessageKey::FieldEvent,
    MessageKey::FieldInternalIdentity,
    MessageKey::FieldRevision,
    MessageKey::FieldSuccesses,
    MessageKey::FieldFailures,
    MessageKey::FieldSamples,
    MessageKey::FieldWindow,
    MessageKey::FieldSearch,
    MessageKey::FieldFilter,
    MessageKey::FieldSort,
    MessageKey::FieldIncompleteReceipts,
    MessageKey::FieldMalformedReceipts,
    MessageKey::FieldLanguage,
    MessageKey::FieldSavedLanguage,
    MessageKey::FieldColor,
    MessageKey::FieldSavedColor,
    MessageKey::FieldRisk,
    MessageKey::FieldRiskScore,
    MessageKey::FieldConfidence,
    MessageKey::FieldClassification,
    MessageKey::FieldPreviousPeriod,
    MessageKey::FieldRecency,
    MessageKey::FieldImpact,
    MessageKey::ColumnName,
    MessageKey::ColumnEvent,
    MessageKey::ColumnRuntime,
    MessageKey::ColumnFailureRate,
    MessageKey::ColumnTrend,
    MessageKey::ColumnRisk,
    MessageKey::StateLoading,
    MessageKey::StateEmpty,
    MessageKey::StateEmptySearch,
    MessageKey::StateRefreshFailed,
    MessageKey::StateTimelineUnavailable,
    MessageKey::StateInsufficientHistory,
    MessageKey::StateInsufficientSamples,
    MessageKey::StateCoverageLimited,
    MessageKey::StateNoRecentFailures,
    MessageKey::StatePreferenceClean,
    MessageKey::StatePreferenceDirty,
    MessageKey::StatePreferenceSaved,
    MessageKey::StatePreferenceConflict,
    MessageKey::StatePreferenceSaveFailed,
    MessageKey::StatusHealthy,
    MessageKey::StatusDegraded,
    MessageKey::StatusCoverageLimited,
    MessageKey::StatusNoTerminalSamples,
    MessageKey::StatusUnavailable,
    MessageKey::StatusRegression,
    MessageKey::StatusImprovement,
    MessageKey::StatusStable,
    MessageKey::StatusInsufficientEvidence,
    MessageKey::DiagnosticPass,
    MessageKey::DiagnosticWarning,
    MessageKey::DiagnosticFail,
    MessageKey::DiagnosticUnknown,
    MessageKey::DiagnosticUnsupported,
    MessageKey::DiagnosticHookStatBinary,
    MessageKey::DiagnosticCodexBinary,
    MessageKey::DiagnosticEffectiveRuntime,
    MessageKey::DiagnosticInstrumentation,
    MessageKey::DiagnosticTrust,
    MessageKey::DiagnosticReceiptSpool,
    MessageKey::DiagnosticLedger,
    MessageKey::DiagnosticReceiptIntegrity,
    MessageKey::DiagnosticEvidenceCoverage,
    MessageKey::DiagnosticPathIdentity,
    MessageKey::DiagnosticEvidenceFreshness,
    MessageKey::DiagnosticHookStatBinaryExplanation,
    MessageKey::DiagnosticCodexBinaryExplanation,
    MessageKey::DiagnosticEffectiveRuntimeExplanation,
    MessageKey::DiagnosticInstrumentationExplanation,
    MessageKey::DiagnosticTrustExplanation,
    MessageKey::DiagnosticReceiptSpoolExplanation,
    MessageKey::DiagnosticLedgerExplanation,
    MessageKey::DiagnosticReceiptIntegrityExplanation,
    MessageKey::DiagnosticEvidenceCoverageExplanation,
    MessageKey::DiagnosticPathIdentityExplanation,
    MessageKey::DiagnosticEvidenceFreshnessExplanation,
    MessageKey::DiagnosticHandlerCounts,
    MessageKey::DiagnosticEvidenceAgeMinutes,
    MessageKey::LanguageAuto,
    MessageKey::LanguageEnUs,
    MessageKey::LanguageZhCn,
    MessageKey::ColorAuto,
    MessageKey::ColorAlways,
    MessageKey::ColorNever,
    MessageKey::CoverageComplete,
    MessageKey::CoveragePartial,
    MessageKey::CoverageSyncOnly,
    MessageKey::CoverageBestEffort,
    MessageKey::CoverageUnknown,
    MessageKey::CoverageNotAdmitted,
    MessageKey::CoverageSyntheticFixture,
    MessageKey::WindowLast24Hours,
    MessageKey::WindowLast7Days,
    MessageKey::WindowLast30Days,
    MessageKey::WindowAll,
    MessageKey::RuntimeCodex,
    MessageKey::RuntimeDeepSeekHarness,
    MessageKey::RuntimeOpenCode,
    MessageKey::EventSessionStart,
    MessageKey::EventSessionEnd,
    MessageKey::EventUserPromptSubmit,
    MessageKey::EventPreToolUse,
    MessageKey::EventPostToolUse,
    MessageKey::EventPermissionRequest,
    MessageKey::EventPreCompact,
    MessageKey::EventPostCompact,
    MessageKey::EventStop,
    MessageKey::EventSubagentStart,
    MessageKey::EventSubagentStop,
    MessageKey::TerminalCompleted,
    MessageKey::TerminalFailed,
    MessageKey::TerminalBlocked,
    MessageKey::TerminalStopped,
    MessageKey::TerminalTimedOut,
    MessageKey::TerminalProtocolFailure,
    MessageKey::TerminalIncomplete,
    MessageKey::TerminalUnknown,
    MessageKey::SortFailureRate,
    MessageKey::SortName,
    MessageKey::SortRuns,
    MessageKey::SortRisk,
    MessageKey::FilterFailuresOnly,
    MessageKey::FilterAllHooks,
    MessageKey::IdentityHook,
    MessageKey::RateWithSample,
    MessageKey::SampleCount,
    MessageKey::FingerprintExitNonzero,
    MessageKey::FingerprintTimedOut,
    MessageKey::FingerprintProtocolFailure,
    MessageKey::FingerprintExecutionFailed,
    MessageKey::FooterNavigate,
    MessageKey::FooterPage,
    MessageKey::FooterOpen,
    MessageKey::FooterBack,
    MessageKey::FooterRefresh,
    MessageKey::FooterQuit,
    MessageKey::FooterFocusContent,
    MessageKey::FooterFocusNavigation,
    MessageKey::FooterSearch,
    MessageKey::FooterFilter,
    MessageKey::FooterSort,
    MessageKey::FooterChange,
    MessageKey::FooterApply,
    MessageKey::FooterRevert,
    MessageKey::MinimumTerminal,
];

impl InterfaceLanguage {
    pub const fn as_storage(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::EnUs => "en-US",
            Self::ZhCn => "zh-CN",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        let normalized = value
            .split(['.', '@'])
            .next()
            .unwrap_or(value)
            .trim()
            .replace('_', "-")
            .to_ascii_lowercase();
        match normalized.as_str() {
            "auto" => Some(Self::Auto),
            "en-us" | "en" => Some(Self::EnUs),
            "zh-cn" | "zh" => Some(Self::ZhCn),
            _ => None,
        }
    }
}

impl LanguageState {
    /// Resolve the small G00 model. G03 will add CLI, preference, and OS reads.
    pub fn resolve(
        requested: InterfaceLanguage,
        environment_locale: Option<&str>,
        preference_locale: Option<InterfaceLanguage>,
        system_locale: Option<&str>,
    ) -> Self {
        match requested {
            InterfaceLanguage::EnUs => Self {
                requested,
                resolved: ResolvedLocale::EnUs,
                source: LocaleSource::Explicit,
            },
            InterfaceLanguage::ZhCn => Self {
                requested,
                resolved: ResolvedLocale::ZhCn,
                source: LocaleSource::Explicit,
            },
            InterfaceLanguage::Auto => resolve_auto(
                requested,
                environment_locale,
                preference_locale,
                system_locale,
            ),
        }
    }
}

fn resolve_auto(
    requested: InterfaceLanguage,
    environment_locale: Option<&str>,
    preference_locale: Option<InterfaceLanguage>,
    system_locale: Option<&str>,
) -> LanguageState {
    for (candidate, source) in [
        (environment_locale, LocaleSource::Environment),
        (
            preference_locale.map(InterfaceLanguage::as_storage),
            LocaleSource::Preference,
        ),
        (system_locale, LocaleSource::System),
    ] {
        if let Some(locale) = candidate.and_then(InterfaceLanguage::parse) {
            let resolved = match locale {
                InterfaceLanguage::ZhCn => ResolvedLocale::ZhCn,
                InterfaceLanguage::EnUs => ResolvedLocale::EnUs,
                InterfaceLanguage::Auto => continue,
            };
            return LanguageState {
                requested,
                resolved,
                source,
            };
        }
    }
    LanguageState {
        requested,
        resolved: ResolvedLocale::EnUs,
        source: LocaleSource::Fallback,
    }
}

pub const fn t(locale: ResolvedLocale, key: MessageKey) -> &'static str {
    match locale {
        ResolvedLocale::EnUs => en_us::text(key),
        ResolvedLocale::ZhCn => zh_cn::text(key),
    }
}

pub const fn runtime_name(locale: ResolvedLocale, runtime: Runtime) -> &'static str {
    let key = match runtime {
        Runtime::Codex => MessageKey::RuntimeCodex,
        Runtime::DeepSeekHarness => MessageKey::RuntimeDeepSeekHarness,
        Runtime::OpenCode => MessageKey::RuntimeOpenCode,
    };
    t(locale, key)
}

pub const fn event_name(locale: ResolvedLocale, event: HookEvent) -> &'static str {
    let key = match event {
        HookEvent::SessionStart => MessageKey::EventSessionStart,
        HookEvent::SessionEnd => MessageKey::EventSessionEnd,
        HookEvent::UserPromptSubmit => MessageKey::EventUserPromptSubmit,
        HookEvent::PreToolUse => MessageKey::EventPreToolUse,
        HookEvent::PostToolUse => MessageKey::EventPostToolUse,
        HookEvent::PermissionRequest => MessageKey::EventPermissionRequest,
        HookEvent::PreCompact => MessageKey::EventPreCompact,
        HookEvent::PostCompact => MessageKey::EventPostCompact,
        HookEvent::Stop => MessageKey::EventStop,
        HookEvent::SubagentStart => MessageKey::EventSubagentStart,
        HookEvent::SubagentStop => MessageKey::EventSubagentStop,
    };
    t(locale, key)
}

pub const fn coverage_name(locale: ResolvedLocale, coverage: EvidenceCoverage) -> &'static str {
    let key = match coverage {
        EvidenceCoverage::Complete => MessageKey::CoverageComplete,
        EvidenceCoverage::Partial => MessageKey::CoveragePartial,
        EvidenceCoverage::SyncOnly => MessageKey::CoverageSyncOnly,
        EvidenceCoverage::BestEffort => MessageKey::CoverageBestEffort,
        EvidenceCoverage::Unknown => MessageKey::CoverageUnknown,
        EvidenceCoverage::NotAdmitted => MessageKey::CoverageNotAdmitted,
        EvidenceCoverage::SyntheticFixture => MessageKey::CoverageSyntheticFixture,
    };
    t(locale, key)
}

pub const fn window_name(locale: ResolvedLocale, window: TimeWindow) -> &'static str {
    let key = match window {
        TimeWindow::Last24Hours => MessageKey::WindowLast24Hours,
        TimeWindow::Last7Days => MessageKey::WindowLast7Days,
        TimeWindow::Last30Days => MessageKey::WindowLast30Days,
        TimeWindow::All => MessageKey::WindowAll,
    };
    t(locale, key)
}

pub const fn health_name(locale: ResolvedLocale, health: Health) -> &'static str {
    let key = match health {
        Health::Healthy => MessageKey::StatusHealthy,
        Health::Degraded => MessageKey::StatusDegraded,
        Health::CoverageLimited => MessageKey::StatusCoverageLimited,
        Health::NoTerminalSamples => MessageKey::StatusNoTerminalSamples,
    };
    t(locale, key)
}

pub const fn interface_language_name(
    locale: ResolvedLocale,
    language: InterfaceLanguage,
) -> &'static str {
    let key = match language {
        InterfaceLanguage::Auto => MessageKey::LanguageAuto,
        InterfaceLanguage::EnUs => MessageKey::LanguageEnUs,
        InterfaceLanguage::ZhCn => MessageKey::LanguageZhCn,
    };
    t(locale, key)
}

pub const fn interface_color_name(
    locale: ResolvedLocale,
    color: crate::interface_preferences::InterfaceColor,
) -> &'static str {
    let key = match color {
        crate::interface_preferences::InterfaceColor::Auto => MessageKey::ColorAuto,
        crate::interface_preferences::InterfaceColor::Always => MessageKey::ColorAlways,
        crate::interface_preferences::InterfaceColor::Never => MessageKey::ColorNever,
    };
    t(locale, key)
}

pub const fn diagnostic_status_name(
    locale: ResolvedLocale,
    status: DiagnosticStatus,
) -> &'static str {
    let key = match status {
        DiagnosticStatus::Pass => MessageKey::DiagnosticPass,
        DiagnosticStatus::Warning => MessageKey::DiagnosticWarning,
        DiagnosticStatus::Fail => MessageKey::DiagnosticFail,
        DiagnosticStatus::Unknown => MessageKey::DiagnosticUnknown,
        DiagnosticStatus::Unsupported => MessageKey::DiagnosticUnsupported,
    };
    t(locale, key)
}

pub const fn diagnostic_title(locale: ResolvedLocale, id: DiagnosticCheckId) -> &'static str {
    let key = match id {
        DiagnosticCheckId::HookStatBinary => MessageKey::DiagnosticHookStatBinary,
        DiagnosticCheckId::CodexBinary => MessageKey::DiagnosticCodexBinary,
        DiagnosticCheckId::EffectiveRuntime => MessageKey::DiagnosticEffectiveRuntime,
        DiagnosticCheckId::Instrumentation => MessageKey::DiagnosticInstrumentation,
        DiagnosticCheckId::Trust => MessageKey::DiagnosticTrust,
        DiagnosticCheckId::ReceiptSpool => MessageKey::DiagnosticReceiptSpool,
        DiagnosticCheckId::Ledger => MessageKey::DiagnosticLedger,
        DiagnosticCheckId::ReceiptIntegrity => MessageKey::DiagnosticReceiptIntegrity,
        DiagnosticCheckId::EvidenceCoverage => MessageKey::DiagnosticEvidenceCoverage,
        DiagnosticCheckId::PathIdentity => MessageKey::DiagnosticPathIdentity,
        DiagnosticCheckId::EvidenceFreshness => MessageKey::DiagnosticEvidenceFreshness,
    };
    t(locale, key)
}

pub const fn diagnostic_explanation(locale: ResolvedLocale, id: DiagnosticCheckId) -> &'static str {
    let key = match id {
        DiagnosticCheckId::HookStatBinary => MessageKey::DiagnosticHookStatBinaryExplanation,
        DiagnosticCheckId::CodexBinary => MessageKey::DiagnosticCodexBinaryExplanation,
        DiagnosticCheckId::EffectiveRuntime => MessageKey::DiagnosticEffectiveRuntimeExplanation,
        DiagnosticCheckId::Instrumentation => MessageKey::DiagnosticInstrumentationExplanation,
        DiagnosticCheckId::Trust => MessageKey::DiagnosticTrustExplanation,
        DiagnosticCheckId::ReceiptSpool => MessageKey::DiagnosticReceiptSpoolExplanation,
        DiagnosticCheckId::Ledger => MessageKey::DiagnosticLedgerExplanation,
        DiagnosticCheckId::ReceiptIntegrity => MessageKey::DiagnosticReceiptIntegrityExplanation,
        DiagnosticCheckId::EvidenceCoverage => MessageKey::DiagnosticEvidenceCoverageExplanation,
        DiagnosticCheckId::PathIdentity => MessageKey::DiagnosticPathIdentityExplanation,
        DiagnosticCheckId::EvidenceFreshness => MessageKey::DiagnosticEvidenceFreshnessExplanation,
    };
    t(locale, key)
}

pub const fn terminal_status_name(locale: ResolvedLocale, status: TerminalStatus) -> &'static str {
    let key = match status {
        TerminalStatus::Completed => MessageKey::TerminalCompleted,
        TerminalStatus::Failed => MessageKey::TerminalFailed,
        TerminalStatus::Blocked => MessageKey::TerminalBlocked,
        TerminalStatus::Stopped => MessageKey::TerminalStopped,
        TerminalStatus::TimedOut => MessageKey::TerminalTimedOut,
        TerminalStatus::ProtocolFailure => MessageKey::TerminalProtocolFailure,
        TerminalStatus::Incomplete => MessageKey::TerminalIncomplete,
        TerminalStatus::Unknown => MessageKey::TerminalUnknown,
    };
    t(locale, key)
}

pub const fn sort_name(locale: ResolvedLocale, sort: HookSort) -> &'static str {
    let key = match sort {
        HookSort::Risk => MessageKey::SortRisk,
        HookSort::FailureRate => MessageKey::SortFailureRate,
        HookSort::Name => MessageKey::SortName,
        HookSort::Runs => MessageKey::SortRuns,
    };
    t(locale, key)
}

pub const fn intelligence_availability_name(
    locale: ResolvedLocale,
    availability: IntelligenceAvailability,
) -> &'static str {
    let key = match availability {
        IntelligenceAvailability::Available => MessageKey::StatusStable,
        IntelligenceAvailability::InsufficientHistory => MessageKey::StateInsufficientHistory,
        IntelligenceAvailability::InsufficientSamples => MessageKey::StateInsufficientSamples,
        IntelligenceAvailability::CoverageLimited => MessageKey::StateCoverageLimited,
    };
    t(locale, key)
}

pub const fn regression_name(
    locale: ResolvedLocale,
    classification: RegressionClassification,
) -> &'static str {
    let key = match classification {
        RegressionClassification::Regression => MessageKey::StatusRegression,
        RegressionClassification::Improvement => MessageKey::StatusImprovement,
        RegressionClassification::Stable => MessageKey::StatusStable,
        RegressionClassification::InsufficientEvidence => MessageKey::StatusInsufficientEvidence,
    };
    t(locale, key)
}

pub const fn fingerprint_name(
    locale: ResolvedLocale,
    fingerprint: FailureFingerprintKind,
) -> &'static str {
    let key = match fingerprint {
        FailureFingerprintKind::ExitNonzero => MessageKey::FingerprintExitNonzero,
        FailureFingerprintKind::TimedOut => MessageKey::FingerprintTimedOut,
        FailureFingerprintKind::ProtocolFailure => MessageKey::FingerprintProtocolFailure,
        FailureFingerprintKind::ExecutionFailed => MessageKey::FingerprintExecutionFailed,
    };
    t(locale, key)
}

pub fn sample_count(locale: ResolvedLocale, count: u64) -> String {
    t(locale, MessageKey::SampleCount).replace("{count}", &count.to_string())
}

pub fn failure_rate_with_sample(locale: ResolvedLocale, rate: f64, samples: u64) -> String {
    t(locale, MessageKey::RateWithSample)
        .replace("{rate}", &format!("{rate:.2}"))
        .replace("{samples}", &sample_count(locale, samples))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_parser_accepts_supported_aliases_only() {
        assert_eq!(
            InterfaceLanguage::parse("zh_CN.UTF-8"),
            Some(InterfaceLanguage::ZhCn)
        );
        assert_eq!(
            InterfaceLanguage::parse("en-US"),
            Some(InterfaceLanguage::EnUs)
        );
        assert_eq!(InterfaceLanguage::parse("fr-FR"), None);
    }

    #[test]
    fn auto_falls_back_to_english_when_sources_are_unsupported() {
        let language = LanguageState::resolve(InterfaceLanguage::Auto, Some("fr-FR"), None, None);
        assert_eq!(language.resolved, ResolvedLocale::EnUs);
        assert_eq!(language.source, LocaleSource::Fallback);
    }

    #[test]
    fn locale_resolution_matches_the_environment_preference_system_order() {
        let preference = LanguageState::resolve(
            InterfaceLanguage::Auto,
            None,
            Some(InterfaceLanguage::ZhCn),
            Some("en-US"),
        );
        assert_eq!(preference.resolved, ResolvedLocale::ZhCn);
        assert_eq!(preference.source, LocaleSource::Preference);
        let environment = LanguageState::resolve(
            InterfaceLanguage::Auto,
            Some("en-US"),
            Some(InterfaceLanguage::ZhCn),
            None,
        );
        assert_eq!(environment.resolved, ResolvedLocale::EnUs);
        assert_eq!(environment.source, LocaleSource::Environment);
    }

    #[test]
    fn concrete_catalogs_cover_every_message_key() {
        for key in ALL_MESSAGE_KEYS {
            assert!(!t(ResolvedLocale::EnUs, *key).is_empty());
            assert!(!t(ResolvedLocale::ZhCn, *key).is_empty());
        }
    }

    #[test]
    fn chinese_rate_keeps_the_terminal_sample_count_adjacent() {
        assert_eq!(
            failure_rate_with_sample(ResolvedLocale::ZhCn, 12.5, 8),
            "12.50%（样本=8）"
        );
    }
}
