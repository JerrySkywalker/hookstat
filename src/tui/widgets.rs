//! Reusable visual containers for the terminal UI system.

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use terminal_ui_contract::{
    footer::{FooterAction, FooterState, format_footer},
    text::truncate_to_width as shared_truncate_to_width,
};
#[cfg(test)]
use unicode_width::UnicodeWidthStr;

use super::app::{App, Screen};
use super::localization::{MessageKey, ResolvedLocale, t};
use super::navigation::{NavigationState, Route};
use super::theme::{ColorRole, Theme, TypographyRole};

pub fn themed_block(title: &str, theme: Theme) -> Block<'_> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(theme.chrome_style(terminal_ui_contract::chrome::ChromeToken::Border))
}

pub fn render_title(
    frame: &mut Frame,
    area: Rect,
    locale: ResolvedLocale,
    overall_status: &str,
    theme: Theme,
) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            t(locale, MessageKey::AppTitle),
            theme.typography_style(TypographyRole::ApplicationTitle),
        ),
        Span::raw(format!(" — {overall_status}")),
    ]))
    .style(theme.chrome_style(terminal_ui_contract::chrome::ChromeToken::Base))
    .alignment(Alignment::Left)
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(theme.color_style(ColorRole::Border)),
    );
    frame.render_widget(title, area);
}

pub fn render_navigation(
    frame: &mut Frame,
    area: Rect,
    locale: ResolvedLocale,
    navigation: NavigationState,
    theme: Theme,
) {
    let lines = Route::ALL
        .iter()
        .map(|route| {
            let current = *route == navigation.current();
            let marker = if current { ">" } else { " " };
            let style = if current {
                theme.chrome_style(terminal_ui_contract::chrome::ChromeToken::CurrentScreen)
            } else {
                theme.chrome_style(terminal_ui_contract::chrome::ChromeToken::Base)
            };
            Line::from(Span::styled(
                format!("{marker} {}", route_label(locale, *route)),
                style,
            ))
        })
        .collect::<Vec<_>>();
    let navigation = Paragraph::new(Text::from(lines)).block(themed_block(
        t(locale, MessageKey::SectionNavigation),
        theme,
    ));
    frame.render_widget(navigation, area);
}

pub fn render_shortcut_footer(
    frame: &mut Frame,
    area: Rect,
    locale: ResolvedLocale,
    app: &App,
    theme: Theme,
) {
    let footer = if app.screen() == Screen::Hooks && app.local_list_active() {
        // The shared contract's generic LocalList includes search/filter/sort
        // affordances. Current runtime catalog rows deliberately expose only
        // actions that are actually available at this local level.
        if area.width < 64 {
            format!(
                "↑↓ {}  Enter {}  Esc {}  q {}",
                t(locale, MessageKey::FooterNavigate),
                t(locale, MessageKey::FooterOpen),
                t(locale, MessageKey::FooterBack),
                t(locale, MessageKey::FooterQuit),
            )
        } else {
            format!(
                "↑↓ {}  Enter {}  Esc {}  ? {}  r {}  q {}",
                t(locale, MessageKey::FooterNavigate),
                t(locale, MessageKey::FooterOpen),
                t(locale, MessageKey::FooterBack),
                t(locale, MessageKey::FooterHelp),
                t(locale, MessageKey::FooterRefresh),
                t(locale, MessageKey::FooterQuit),
            )
        }
    } else {
        let state = footer_state(app);
        format_footer(state, |action| footer_action_text(locale, action))
    };
    frame.render_widget(
        Paragraph::new(truncate_to_width(&footer, area.width as usize))
            .style(theme.chrome_style(terminal_ui_contract::chrome::ChromeToken::Footer)),
        area,
    );
}

fn footer_state(app: &App) -> FooterState {
    if app.help_open() {
        FooterState::HelpOverlay
    } else if app.discard_confirmation_open() {
        FooterState::DiscardConfirmation
    } else if app.settings_save_state() == super::app::SettingsSaveState::Conflict
        || (app.alias_editing() && app.alias_save_state() == super::app::AliasSaveState::Conflict)
    {
        FooterState::ConflictWarning
    } else if app.alias_text_editing() {
        FooterState::SettingsEdit
    } else if app.alias_editing() && app.alias_dirty() {
        FooterState::DirtyDraft
    } else if app.alias_editing() {
        FooterState::SettingsNormal
    } else if app.screen() == Screen::Settings && app.settings_editing() {
        FooterState::SettingsEdit
    } else if app.screen() == Screen::Settings && app.settings_dirty() {
        FooterState::DirtyDraft
    } else if app.screen() == Screen::Settings {
        FooterState::SettingsNormal
    } else if matches!(
        app.screen(),
        Screen::HookDetail | Screen::ChangeDetail | Screen::FailureClusterDetail
    ) {
        FooterState::Detail
    } else if app.local_list_active() {
        FooterState::LocalList
    } else {
        FooterState::NormalNavigationWithRefresh
    }
}

fn footer_action_text(locale: ResolvedLocale, action: FooterAction) -> &'static str {
    let key = match action {
        FooterAction::Navigate => MessageKey::FooterNavigate,
        FooterAction::Open => MessageKey::FooterOpen,
        FooterAction::Help => MessageKey::FooterHelp,
        FooterAction::Refresh => MessageKey::FooterRefresh,
        FooterAction::Quit => MessageKey::FooterQuit,
        FooterAction::Select => MessageKey::FooterSelect,
        FooterAction::Back => MessageKey::FooterBack,
        FooterAction::Page => MessageKey::FooterPage,
        FooterAction::Search => MessageKey::FooterSearch,
        FooterAction::Filter => MessageKey::FooterFilter,
        FooterAction::Sort => MessageKey::FooterSort,
        FooterAction::Edit => MessageKey::FooterEdit,
        FooterAction::Change => MessageKey::FooterChange,
        FooterAction::Apply => MessageKey::FooterApply,
        FooterAction::Revert => MessageKey::FooterRevert,
        FooterAction::Cancel => MessageKey::FooterCancel,
        FooterAction::Discard => MessageKey::FooterDiscard,
        FooterAction::Dismiss => MessageKey::FooterDismiss,
    };
    t(locale, key)
}

pub fn render_state_panel(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    message: &str,
    role: ColorRole,
    theme: Theme,
) {
    let panel = Paragraph::new(message)
        .style(theme.color_style(role))
        .wrap(Wrap { trim: true })
        .block(themed_block(title, theme));
    frame.render_widget(panel, area);
}

pub fn render_minimum_size(frame: &mut Frame, area: Rect, locale: ResolvedLocale, theme: Theme) {
    render_state_panel(
        frame,
        area,
        t(locale, MessageKey::AppTitle),
        t(locale, MessageKey::MinimumTerminal),
        ColorRole::Warning,
        theme,
    );
}

#[cfg(test)]
pub fn display_width(value: &str) -> usize {
    UnicodeWidthStr::width(value)
}

/// Truncate by terminal display cells and never split a grapheme cluster.
pub fn truncate_to_width(value: &str, maximum_width: usize) -> String {
    shared_truncate_to_width(value, maximum_width)
}

fn route_label(locale: ResolvedLocale, route: Route) -> &'static str {
    let key = match route {
        Route::Overview => MessageKey::NavOverview,
        Route::Hooks => MessageKey::NavHooks,
        Route::Changes => MessageKey::NavChanges,
        Route::Diagnostics => MessageKey::NavDiagnostics,
        Route::Settings => MessageKey::NavSettings,
    };
    t(locale, key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_primitives_preserve_combining_and_cjk_graphemes() {
        assert_eq!(display_width("e\u{301}"), 1);
        assert_eq!(display_width("诊"), 2);
        assert_eq!(truncate_to_width("e\u{301}诊断", 3), "e\u{301}诊");
    }
}
