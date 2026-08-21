//! Reusable visual containers for the terminal UI system.

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::app::Focus;
use super::localization::{MessageKey, ResolvedLocale, t};
use super::navigation::{NavigationState, Route};
use super::theme::{ColorRole, Theme, TypographyRole};

pub fn themed_block(title: &str, theme: Theme) -> Block<'_> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(theme.color_style(ColorRole::Border))
}

pub fn render_title(frame: &mut Frame, area: Rect, locale: ResolvedLocale, theme: Theme) {
    let title = Paragraph::new(t(locale, MessageKey::AppTitle))
        .style(theme.typography_style(TypographyRole::ApplicationTitle))
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
    focus: Focus,
    theme: Theme,
) {
    let lines = Route::ALL
        .iter()
        .map(|route| {
            let selected = *route == navigation.selected();
            let active = *route == navigation.active();
            let marker = if selected {
                ">"
            } else if active {
                "•"
            } else {
                " "
            };
            let style = if selected && focus == Focus::Navigation {
                theme.color_style(ColorRole::Selected)
            } else if active {
                theme.color_style(ColorRole::Info)
            } else {
                theme.typography_style(TypographyRole::Value)
            };
            Line::from(Span::styled(
                format!("{marker} {}", route_label(locale, *route)),
                style,
            ))
        })
        .collect::<Vec<_>>();
    let navigation = Paragraph::new(Text::from(lines)).block(themed_block("Navigation", theme));
    frame.render_widget(navigation, area);
}

pub fn render_shortcut_footer(
    frame: &mut Frame,
    area: Rect,
    locale: ResolvedLocale,
    focus: Focus,
    detail_open: bool,
    theme: Theme,
) {
    let focus_text = match focus {
        Focus::Content => t(locale, MessageKey::FooterFocusNavigation),
        Focus::Navigation => t(locale, MessageKey::FooterFocusContent),
    };
    let back_or_open = if detail_open {
        format!("Esc {}", t(locale, MessageKey::FooterBack))
    } else {
        format!("Enter {}", t(locale, MessageKey::FooterOpen))
    };
    let text = format!(
        "Tab {focus_text} · ↑/↓ {} · {back_or_open} · r {} · q {}",
        t(locale, MessageKey::FooterNavigate),
        t(locale, MessageKey::FooterRefresh),
        t(locale, MessageKey::FooterQuit),
    );
    frame.render_widget(
        Paragraph::new(truncate_to_width(&text, area.width as usize))
            .style(theme.typography_style(TypographyRole::Metadata)),
        area,
    );
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

pub fn display_width(value: &str) -> usize {
    UnicodeWidthStr::width(value)
}

/// Truncate by terminal display cells and never split a grapheme cluster.
pub fn truncate_to_width(value: &str, maximum_width: usize) -> String {
    let mut result = String::new();
    let mut used = 0;
    for grapheme in value.graphemes(true) {
        let width = display_width(grapheme);
        if used + width > maximum_width {
            break;
        }
        result.push_str(grapheme);
        used += width;
    }
    result
}

fn route_label(locale: ResolvedLocale, route: Route) -> &'static str {
    let key = match route {
        Route::Overview => MessageKey::NavOverview,
        Route::Hooks => MessageKey::NavHooks,
        Route::Trends => MessageKey::NavTrends,
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
