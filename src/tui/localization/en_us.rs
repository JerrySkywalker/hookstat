use super::MessageKey;

pub const fn text(key: MessageKey) -> &'static str {
    match key {
        MessageKey::AppTitle => "HookStat Reliability Center",
        MessageKey::NavOverview => "Overview",
        MessageKey::NavHooks => "Hooks",
        MessageKey::NavTrends => "Trends",
        MessageKey::NavDiagnostics => "Diagnostics",
        MessageKey::NavSettings => "Settings",
        MessageKey::StateLoading => "Loading accepted reliability data…",
        MessageKey::StateEmpty => "No admitted receipt rows yet. This is not 0.00% healthy.",
        MessageKey::StateRefreshFailed => "Refresh failed; accepted history retained.",
        MessageKey::StatePlaceholder => {
            "This Reliability Center view is introduced in a later v0.2 goal."
        }
        MessageKey::FooterNavigate => "navigate",
        MessageKey::FooterOpen => "open",
        MessageKey::FooterBack => "back",
        MessageKey::FooterRefresh => "refresh",
        MessageKey::FooterQuit => "quit",
        MessageKey::FooterFocusContent => "content",
        MessageKey::FooterFocusNavigation => "navigation",
        MessageKey::MinimumTerminal => "Resize to at least 24x10",
    }
}
