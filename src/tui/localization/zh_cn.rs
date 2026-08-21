use super::MessageKey;

pub const fn text(key: MessageKey) -> &'static str {
    match key {
        MessageKey::AppTitle => "HookStat 可靠性中心",
        MessageKey::NavOverview => "概览",
        MessageKey::NavHooks => "钩子",
        MessageKey::NavTrends => "趋势",
        MessageKey::NavDiagnostics => "诊断",
        MessageKey::NavSettings => "设置",
        MessageKey::StateLoading => "正在加载已接受的可靠性数据…",
        MessageKey::StateEmpty => "尚无已接纳的回执记录；这并不表示 0.00% 健康。",
        MessageKey::StateRefreshFailed => "刷新失败；已接纳的历史记录仍被保留。",
        MessageKey::StatePlaceholder => "此可靠性中心视图将在后续 v0.2 目标中实现。",
        MessageKey::FooterNavigate => "导航",
        MessageKey::FooterOpen => "打开",
        MessageKey::FooterBack => "返回",
        MessageKey::FooterRefresh => "刷新",
        MessageKey::FooterQuit => "退出",
        MessageKey::FooterFocusContent => "内容",
        MessageKey::FooterFocusNavigation => "导航栏",
        MessageKey::MinimumTerminal => "请调整到至少 24x10",
    }
}
