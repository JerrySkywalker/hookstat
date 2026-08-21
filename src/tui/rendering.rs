//! Pure Ratatui rendering over accepted application state.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Cell, Paragraph, Row, Table, Wrap},
};

use super::app::{App, CompatibilityScreen, Focus};
use super::layout::{ApplicationShell, ShellLayout};
use super::localization::{LanguageState, MessageKey, t};
use super::state::ResourceState;
use super::theme::{ColorRole, Theme, TypographyRole};
use super::widgets::{
    render_minimum_size, render_navigation, render_shortcut_footer, render_state_panel,
    themed_block, truncate_to_width,
};

pub fn draw(frame: &mut Frame, app: &App, language: LanguageState, theme: Theme) {
    match ApplicationShell::new().layout(frame.area()) {
        ShellLayout::TooSmall { available } => {
            render_minimum_size(frame, available, language.resolved, theme);
        }
        ShellLayout::Ready(areas) => {
            super::widgets::render_title(frame, areas.title, language.resolved, theme);
            render_navigation(
                frame,
                areas.navigation,
                language.resolved,
                app.navigation(),
                app.focus(),
                theme,
            );
            render_content(frame, areas.content, app, language, theme);
            render_shortcut_footer(
                frame,
                areas.footer,
                language.resolved,
                app.focus(),
                app.compatibility_screen() == CompatibilityScreen::Detail,
                theme,
            );
        }
    }
}

fn render_content(frame: &mut Frame, area: Rect, app: &App, language: LanguageState, theme: Theme) {
    if app.route_is_placeholder() {
        render_state_panel(
            frame,
            area,
            "v0.2 foundation",
            t(language.resolved, MessageKey::StatePlaceholder),
            ColorRole::Info,
            theme,
        );
        return;
    }

    let state = app.report_state();
    let Some(report) = state.accepted() else {
        let (message, role) = match state {
            ResourceState::Loading { .. } => (
                t(language.resolved, MessageKey::StateLoading),
                ColorRole::Info,
            ),
            ResourceState::Error { .. } => (
                t(language.resolved, MessageKey::StateRefreshFailed),
                ColorRole::Danger,
            ),
            ResourceState::Empty | ResourceState::Ready(_) => (
                t(language.resolved, MessageKey::StateEmpty),
                ColorRole::Warning,
            ),
        };
        render_state_panel(frame, area, "HookStat", message, role, theme);
        return;
    };

    if report.handlers.is_empty() {
        render_state_panel(
            frame,
            area,
            "v0.1 compatibility",
            t(language.resolved, MessageKey::StateEmpty),
            ColorRole::Warning,
            theme,
        );
        return;
    }

    match app.compatibility_screen() {
        CompatibilityScreen::Home => render_legacy_home(frame, area, app, theme),
        CompatibilityScreen::Detail => render_legacy_detail(frame, area, app, theme),
    }
    if state.is_loading() {
        render_notice(
            frame,
            area,
            t(language.resolved, MessageKey::StateLoading),
            ColorRole::Info,
            theme,
        );
    } else if state.error_message().is_some() {
        render_notice(
            frame,
            area,
            t(language.resolved, MessageKey::StateRefreshFailed),
            ColorRole::Danger,
            theme,
        );
    }
}

fn render_legacy_home(frame: &mut Frame, area: Rect, app: &App, theme: Theme) {
    let report = app
        .report_state()
        .accepted()
        .expect("accepted report checked before render");
    if area.height < 8 || area.width < 32 {
        render_minimum_compatibility(frame, area, app, theme);
        return;
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(4)])
        .split(area);
    let coverage = format!(
        "Codex · instrumented receipts · {:?} coverage · incomplete={} malformed={} · {}",
        report.qualification.coverage,
        report.incomplete_receipts,
        report.malformed_receipts,
        report.window.label()
    );
    frame.render_widget(
        Paragraph::new(truncate_to_width(&coverage, chunks[0].width as usize))
            .style(theme.typography_style(TypographyRole::Metadata))
            .wrap(Wrap { trim: true }),
        chunks[0],
    );

    if chunks[1].width < 52 {
        let lines = report
            .handlers
            .iter()
            .enumerate()
            .map(|(index, item)| {
                format!(
                    "{} {} {:.2}% (n={})",
                    if index == app.selected_handler_index() {
                        ">"
                    } else {
                        " "
                    },
                    truncate_to_width(&item.handler.key, 12),
                    item.failure_rate_percent,
                    item.failure_sample_count
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        frame.render_widget(
            Paragraph::new(lines)
                .style(theme.typography_style(TypographyRole::Value))
                .block(themed_block("v0.1 report compatibility", theme)),
            chunks[1],
        );
        return;
    }

    let header = Row::new(["Handler", "Runs", "Failed", "Failure"])
        .style(theme.typography_style(TypographyRole::SectionTitle));
    let rows = report.handlers.iter().enumerate().map(|(index, item)| {
        let style = if index == app.selected_handler_index() && app.focus() == Focus::Content {
            theme.color_style(ColorRole::Selected)
        } else {
            theme.typography_style(TypographyRole::Value)
        };
        Row::new(vec![
            Cell::from(truncate_to_width(&item.handler.key, 30)),
            Cell::from(item.runs.to_string()),
            Cell::from(item.failed_runs.to_string()),
            Cell::from(format!(
                "{:.2}% (n={})",
                item.failure_rate_percent, item.failure_sample_count
            )),
        ])
        .style(style)
    });
    let table = Table::new(
        rows,
        [
            Constraint::Min(18),
            Constraint::Length(8),
            Constraint::Length(9),
            Constraint::Length(15),
        ],
    )
    .header(header)
    .block(themed_block("v0.1 report compatibility", theme));
    frame.render_widget(table, chunks[1]);
}

fn render_minimum_compatibility(frame: &mut Frame, area: Rect, app: &App, theme: Theme) {
    let report = app
        .report_state()
        .accepted()
        .expect("accepted report checked before render");
    let Some(item) = report.handlers.get(app.selected_handler_index()) else {
        return;
    };
    let text = format!(
        "> {}\n  {}/{} {:.1}% (n={})",
        truncate_to_width(&item.handler.key, area.width.saturating_sub(8) as usize),
        item.failed_runs,
        item.failure_sample_count,
        item.failure_rate_percent,
        item.failure_sample_count,
    );
    frame.render_widget(
        Paragraph::new(text)
            .style(theme.typography_style(TypographyRole::Value))
            .block(themed_block("v0.1", theme)),
        area,
    );
}

fn render_legacy_detail(frame: &mut Frame, area: Rect, app: &App, theme: Theme) {
    let report = app
        .report_state()
        .accepted()
        .expect("accepted report checked before render");
    let Some(item) = report.handlers.get(app.selected_handler_index()) else {
        render_state_panel(
            frame,
            area,
            "v0.1 compatibility",
            "No handler selected. Esc to return.",
            ColorRole::Warning,
            theme,
        );
        return;
    };
    let mut text = format!(
        "{} · {} · Codex\n\n{}\nFailed {} / samples {} · {:.2}% (n={})\n\nCompleted {}  Failed {}  Blocked {}  Stopped {}\nTimedOut {}  ProtocolFailure {}",
        item.handler.key,
        item.handler.event.label(),
        report.window.label(),
        item.failed_runs,
        item.failure_sample_count,
        item.failure_rate_percent,
        item.failure_sample_count,
        item.terminal.completed,
        item.terminal.failed,
        item.terminal.blocked,
        item.terminal.stopped,
        item.terminal.timed_out,
        item.terminal.protocol_failure,
    );
    if item.terminal.incomplete > 0 || item.terminal.unknown > 0 {
        text.push_str(&format!(
            "\n\nCoverage warning: incomplete={} unknown={}; never treated as healthy.",
            item.terminal.incomplete, item.terminal.unknown
        ));
    }
    if let Some(p50) = item.p50_duration_ms {
        text.push_str(&format!(
            "\n\np50 {p50} ms · p95 {} ms · p99 {} ms",
            item.p95_duration_ms.unwrap_or(p50),
            item.p99_duration_ms.unwrap_or(p50)
        ));
    }
    frame.render_widget(
        Paragraph::new(text)
            .style(theme.typography_style(TypographyRole::Value))
            .wrap(Wrap { trim: true })
            .block(themed_block("v0.1 detail compatibility", theme)),
        area,
    );
}

fn render_notice(frame: &mut Frame, area: Rect, message: &str, role: ColorRole, theme: Theme) {
    let notice = Rect {
        x: area.x,
        y: area.y.saturating_add(area.height.saturating_sub(1)),
        width: area.width,
        height: 1,
    };
    frame.render_widget(
        Paragraph::new(truncate_to_width(message, notice.width as usize))
            .style(theme.color_style(role)),
        notice,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::synthetic_fixture_report;
    use ratatui::{Terminal, backend::TestBackend};

    fn rendered(app: App, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                draw(
                    frame,
                    &app,
                    LanguageState::resolve(
                        super::super::localization::InterfaceLanguage::EnUs,
                        None,
                        None,
                    ),
                    Theme::default_color(),
                );
            })
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    }

    #[test]
    fn normal_narrow_and_minimum_buffers_are_safe() {
        let app = App::from_report(synthetic_fixture_report(1_000));
        for (width, height) in [(100, 30), (44, 16), (24, 10)] {
            assert!(rendered(app.clone(), width, height).contains("n="));
        }
        assert!(rendered(app, 23, 10).contains("Resize"));
    }
}
