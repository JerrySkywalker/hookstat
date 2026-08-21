//! Pure Ratatui rendering over an accepted Reliability Center view model.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, Table, Wrap},
};

use super::app::{App, Focus, Screen, SettingsField, SettingsSaveState};
use super::layout::{ApplicationShell, ShellLayout};
use super::localization::{
    LanguageState, MessageKey, ResolvedLocale, coverage_name, diagnostic_explanation,
    diagnostic_status_name, diagnostic_title, event_name, failure_rate_with_sample, health_name,
    interface_color_name, interface_language_name, runtime_name, sort_name, t,
    terminal_status_name, window_name,
};
use super::state::ResourceState;
use super::theme::{ColorRole, Theme, TypographyRole};
use super::view_model::{
    DiagnosticCheckViewModel, DiagnosticFact, DiagnosticStatus, DiagnosticsViewModel,
    DisplayIdentity, Health, HookDetailViewModel, HookRowViewModel,
};
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
            render_content(frame, areas.content, app, language.resolved, theme);
            render_shortcut_footer(frame, areas.footer, language.resolved, app, theme);
        }
    }
}

fn render_content(frame: &mut Frame, area: Rect, app: &App, locale: ResolvedLocale, theme: Theme) {
    let Some(view) = app.view_model() else {
        let (message, role) = match app.view_state() {
            ResourceState::Loading { .. } => (MessageKey::StateLoading, ColorRole::Info),
            ResourceState::Error { .. } => (MessageKey::StateRefreshFailed, ColorRole::Danger),
            ResourceState::Empty | ResourceState::Ready(_) => {
                (MessageKey::StateEmpty, ColorRole::Warning)
            }
        };
        render_state_panel(
            frame,
            area,
            t(locale, MessageKey::AppTitle),
            t(locale, message),
            role,
            theme,
        );
        return;
    };

    match app.screen() {
        Screen::Overview => render_overview(frame, area, app, locale, theme),
        Screen::Hooks => render_hooks(frame, area, app, locale, theme),
        Screen::Diagnostics => render_diagnostics(frame, area, &view.diagnostics, locale, theme),
        Screen::Settings => render_settings(frame, area, app, locale, theme),
        Screen::HookDetail => {
            let detail = app
                .selected_handler()
                .and_then(|reference| view.detail(reference));
            match detail {
                Some(detail) => render_hook_detail(frame, area, detail, locale, theme),
                None => render_state_panel(
                    frame,
                    area,
                    t(locale, MessageKey::ViewHookDetail),
                    t(locale, MessageKey::StateEmpty),
                    ColorRole::Warning,
                    theme,
                ),
            }
        }
    }
    if app.view_state().is_loading() {
        render_notice(
            frame,
            area,
            t(locale, MessageKey::StateLoading),
            ColorRole::Info,
            theme,
        );
    } else if app.view_state().error_message().is_some() {
        render_notice(
            frame,
            area,
            t(locale, MessageKey::StateRefreshFailed),
            ColorRole::Danger,
            theme,
        );
    }
}

fn render_settings(frame: &mut Frame, area: Rect, app: &App, locale: ResolvedLocale, theme: Theme) {
    let lines = vec![
        key_value_line(
            locale,
            MessageKey::FieldLanguage,
            &format!(
                "{} {}",
                settings_marker(app, SettingsField::Language),
                interface_language_name(locale, app.draft_language())
            ),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldColor,
            &format!(
                "{} {}",
                settings_marker(app, SettingsField::Color),
                interface_color_name(locale, app.draft_color())
            ),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldSavedLanguage,
            interface_language_name(locale, app.accepted_language()),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldSavedColor,
            interface_color_name(locale, app.accepted_color()),
            theme,
        ),
        Line::from(Span::styled(
            settings_message(locale, app.settings_save_state()),
            theme.typography_style(TypographyRole::Metadata),
        )),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(themed_block(t(locale, MessageKey::SectionInterface), theme))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn settings_marker(app: &App, field: SettingsField) -> &'static str {
    if app.settings_field() == field {
        ">"
    } else {
        " "
    }
}

fn settings_message(locale: ResolvedLocale, state: SettingsSaveState) -> &'static str {
    let key = match state {
        SettingsSaveState::Clean => MessageKey::StatePreferenceClean,
        SettingsSaveState::Dirty => MessageKey::StatePreferenceDirty,
        SettingsSaveState::Saved => MessageKey::StatePreferenceSaved,
        SettingsSaveState::Conflict => MessageKey::StatePreferenceConflict,
        SettingsSaveState::Failed => MessageKey::StatePreferenceSaveFailed,
    };
    t(locale, key)
}

fn render_diagnostics(
    frame: &mut Frame,
    area: Rect,
    diagnostics: &DiagnosticsViewModel,
    locale: ResolvedLocale,
    theme: Theme,
) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);
    let summary = format!(
        "{}: {}",
        t(locale, MessageKey::FieldHealth),
        diagnostic_status_name(locale, diagnostics.overall_status),
    );
    frame.render_widget(
        Paragraph::new(truncate_to_width(&summary, sections[0].width as usize))
            .style(theme.color_style(diagnostic_color(diagnostics.overall_status)))
            .block(themed_block(t(locale, MessageKey::ViewDiagnostics), theme)),
        sections[0],
    );

    if sections[1].width < 54 {
        let content = diagnostics
            .checks
            .iter()
            .map(|check| {
                format!(
                    "{} · {}\n{}\n{}",
                    diagnostic_title(locale, check.id),
                    diagnostic_status_name(locale, check.status),
                    diagnostic_facts(locale, check),
                    diagnostic_explanation(locale, check.id),
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        frame.render_widget(
            Paragraph::new(content)
                .style(theme.typography_style(TypographyRole::Value))
                .wrap(Wrap { trim: true })
                .block(themed_block(
                    t(locale, MessageKey::SectionDiagnostics),
                    theme,
                )),
            sections[1],
        );
        return;
    }

    let header = Row::new([
        t(locale, MessageKey::ColumnName),
        t(locale, MessageKey::FieldHealth),
        t(locale, MessageKey::FieldCoverage),
    ])
    .style(theme.typography_style(TypographyRole::SectionTitle));
    let rows = diagnostics.checks.iter().map(|check| {
        Row::new([
            truncate_to_width(diagnostic_title(locale, check.id), 24),
            diagnostic_status_name(locale, check.status).to_owned(),
            truncate_to_width(&diagnostic_facts(locale, check), 50),
        ])
        .style(theme.color_style(diagnostic_color(check.status)))
    });
    let table = Table::new(
        rows,
        [
            Constraint::Length(24),
            Constraint::Length(15),
            Constraint::Min(16),
        ],
    )
    .header(header)
    .block(themed_block(
        t(locale, MessageKey::SectionDiagnostics),
        theme,
    ));
    frame.render_widget(table, sections[1]);
}

fn diagnostic_facts(locale: ResolvedLocale, check: &DiagnosticCheckViewModel) -> String {
    let facts = check
        .facts
        .iter()
        .map(|fact| match fact {
            DiagnosticFact::Runtime { runtime } => format!(
                "{}: {}",
                t(locale, MessageKey::FieldRuntime),
                runtime_name(locale, *runtime),
            ),
            DiagnosticFact::Version { value } => format!("v{value}"),
            DiagnosticFact::HandlerCounts {
                discovered,
                instrumented,
                unsupported,
            } => t(locale, MessageKey::DiagnosticHandlerCounts)
                .replace("{discovered}", &discovered.to_string())
                .replace("{instrumented}", &instrumented.to_string())
                .replace("{unsupported}", &unsupported.to_string()),
            DiagnosticFact::LedgerInvocations { count } => {
                format!("{}: {count}", t(locale, MessageKey::FieldTotalRuns),)
            }
            DiagnosticFact::ReceiptRecords { count } => {
                format!("{}: {count}", t(locale, MessageKey::FieldRunCount),)
            }
            DiagnosticFact::ReceiptIntegrity {
                incomplete,
                malformed,
            } => format!(
                "{}: {incomplete} · {}: {malformed}",
                t(locale, MessageKey::FieldIncompleteReceipts),
                t(locale, MessageKey::FieldMalformedReceipts),
            ),
            DiagnosticFact::Coverage { coverage } => format!(
                "{}: {}",
                t(locale, MessageKey::FieldCoverage),
                coverage_name(locale, *coverage),
            ),
            DiagnosticFact::EvidenceAgeMinutes { age_minutes } => {
                t(locale, MessageKey::DiagnosticEvidenceAgeMinutes)
                    .replace("{minutes}", &age_minutes.to_string())
            }
        })
        .collect::<Vec<_>>();
    if facts.is_empty() {
        t(locale, MessageKey::StatusUnavailable).to_owned()
    } else {
        facts.join(" · ")
    }
}

fn render_overview(frame: &mut Frame, area: Rect, app: &App, locale: ResolvedLocale, theme: Theme) {
    let view = app
        .view_model()
        .expect("accepted view checked before render");
    let summary = view.overview.runtime_summaries.first();
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(5)])
        .split(area);
    let Some(summary) = summary else {
        render_state_panel(
            frame,
            area,
            t(locale, MessageKey::ViewOverview),
            t(locale, MessageKey::StateEmpty),
            ColorRole::Warning,
            theme,
        );
        return;
    };
    let overview_lines = vec![
        key_value_line(
            locale,
            MessageKey::FieldRuntime,
            runtime_name(locale, summary.runtime),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldCoverage,
            coverage_name(locale, summary.coverage),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldTotalRuns,
            &summary.total_runs.to_string(),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldFailureRate,
            &failure_rate_with_sample(
                locale,
                summary.failure_rate_percent,
                summary.terminal_sample_count,
            ),
            theme,
        ),
        status_line(locale, summary.health, theme),
    ];
    frame.render_widget(
        Paragraph::new(overview_lines)
            .block(themed_block(
                t(locale, MessageKey::SectionRuntimeSummary),
                theme,
            ))
            .wrap(Wrap { trim: true }),
        sections[0],
    );

    if view.overview.highest_risk_hooks.is_empty() {
        render_state_panel(
            frame,
            sections[1],
            t(locale, MessageKey::SectionRiskyHooks),
            t(locale, MessageKey::StateEmpty),
            ColorRole::Warning,
            theme,
        );
        return;
    }
    render_hook_rows(
        frame,
        sections[1],
        &view.overview.highest_risk_hooks,
        HookRowsContext {
            selected: app.selected_handler(),
            content_focused: app.focus() == Focus::Content,
            locale,
            theme,
            title: t(locale, MessageKey::SectionRiskyHooks),
        },
    );
}

fn render_hooks(frame: &mut Frame, area: Rect, app: &App, locale: ResolvedLocale, theme: Theme) {
    let query = app.hooks_query();
    let filter = if query.failures_only {
        t(locale, MessageKey::FilterFailuresOnly)
    } else {
        t(locale, MessageKey::FilterAllHooks)
    };
    let query_line = format!(
        "{}: {} · {}: {} · {}: {}",
        t(locale, MessageKey::FieldSearch),
        query.search,
        t(locale, MessageKey::FieldFilter),
        filter,
        t(locale, MessageKey::FieldSort),
        sort_name(locale, query.sort),
    );
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);
    frame.render_widget(
        Paragraph::new(truncate_to_width(&query_line, sections[0].width as usize))
            .style(theme.typography_style(TypographyRole::Metadata))
            .block(themed_block(t(locale, MessageKey::ViewHooks), theme)),
        sections[0],
    );
    if app.visible_hooks().is_empty() {
        let state = if query.search.is_empty() && !query.failures_only {
            MessageKey::StateEmpty
        } else {
            MessageKey::StateEmptySearch
        };
        render_state_panel(
            frame,
            sections[1],
            t(locale, MessageKey::ViewHooks),
            t(locale, state),
            ColorRole::Warning,
            theme,
        );
        return;
    }
    render_hook_rows(
        frame,
        sections[1],
        app.visible_hooks(),
        HookRowsContext {
            selected: app.selected_handler(),
            content_focused: app.focus() == Focus::Content,
            locale,
            theme,
            title: t(locale, MessageKey::ViewHooks),
        },
    );
}

struct HookRowsContext<'a> {
    selected: Option<&'a super::view_model::HandlerRef>,
    content_focused: bool,
    locale: ResolvedLocale,
    theme: Theme,
    title: &'a str,
}

fn render_hook_rows(
    frame: &mut Frame,
    area: Rect,
    rows: &[HookRowViewModel],
    context: HookRowsContext<'_>,
) {
    if area.width < 54 {
        let content = rows
            .iter()
            .map(|row| {
                let marker = if context.selected == Some(&row.internal_ref) {
                    ">"
                } else {
                    " "
                };
                let identity = display_identity(
                    context.locale,
                    &row.display_identity,
                    row.display_disambiguator,
                );
                format!(
                    "{marker} {}\n  {} · {} · {} · {}",
                    truncate_to_width(&identity, area.width.saturating_sub(6) as usize),
                    event_name(context.locale, row.event),
                    runtime_name(context.locale, row.internal_ref.runtime),
                    failure_rate_with_sample(
                        context.locale,
                        row.failure_rate_percent,
                        row.sample_count,
                    ),
                    t(context.locale, MessageKey::StatusUnavailable),
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        frame.render_widget(
            Paragraph::new(content)
                .style(context.theme.typography_style(TypographyRole::Value))
                .block(themed_block(context.title, context.theme))
                .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }
    let header = Row::new([
        t(context.locale, MessageKey::ColumnName),
        t(context.locale, MessageKey::ColumnEvent),
        t(context.locale, MessageKey::ColumnRuntime),
        t(context.locale, MessageKey::ColumnFailureRate),
        t(context.locale, MessageKey::ColumnTrend),
    ])
    .style(context.theme.typography_style(TypographyRole::SectionTitle));
    let rows = rows.iter().map(|row| {
        let style = if context.selected == Some(&row.internal_ref) && context.content_focused {
            context.theme.color_style(ColorRole::Selected)
        } else {
            context.theme.typography_style(TypographyRole::Value)
        };
        Row::new(vec![
            Cell::from(truncate_to_width(
                &display_identity(
                    context.locale,
                    &row.display_identity,
                    row.display_disambiguator,
                ),
                28,
            )),
            Cell::from(event_name(context.locale, row.event)),
            Cell::from(runtime_name(context.locale, row.internal_ref.runtime)),
            Cell::from(failure_rate_with_sample(
                context.locale,
                row.failure_rate_percent,
                row.sample_count,
            )),
            Cell::from(t(context.locale, MessageKey::StatusUnavailable)),
        ])
        .style(style)
    });
    let table = Table::new(
        rows,
        [
            Constraint::Min(16),
            Constraint::Length(14),
            Constraint::Length(10),
            Constraint::Length(18),
            Constraint::Length(12),
        ],
    )
    .header(header)
    .block(themed_block(context.title, context.theme));
    frame.render_widget(table, area);
}

fn render_hook_detail(
    frame: &mut Frame,
    area: Rect,
    detail: &HookDetailViewModel,
    locale: ResolvedLocale,
    theme: Theme,
) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10),
            Constraint::Min(7),
            Constraint::Length(3),
        ])
        .split(area);
    let identity = display_identity(
        locale,
        &detail.display_identity,
        detail.display_disambiguator,
    );
    let facts = vec![
        key_value_line(
            locale,
            MessageKey::FieldRuntime,
            runtime_name(locale, detail.internal_ref.runtime),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldEvent,
            event_name(locale, detail.event),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldCoverage,
            coverage_name(locale, detail.coverage),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldInternalIdentity,
            &detail.internal_ref.handler_key,
            theme,
        ),
        key_value_line(locale, MessageKey::FieldRevision, &detail.revision, theme),
        key_value_line(
            locale,
            MessageKey::FieldRunCount,
            &detail.runs.to_string(),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldWindow,
            window_name(locale, detail.window),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldFailureRate,
            &failure_rate_with_sample(locale, detail.failure_rate_percent, detail.sample_count),
            theme,
        ),
    ];
    frame.render_widget(
        Paragraph::new(facts)
            .block(themed_block(&identity, theme))
            .wrap(Wrap { trim: true }),
        sections[0],
    );

    let recent = if detail.recent_failures.is_empty() {
        t(locale, MessageKey::StateNoRecentFailures).to_owned()
    } else {
        detail
            .recent_failures
            .iter()
            .map(|failure| {
                let fingerprint = failure.bounded_fingerprint.as_deref().unwrap_or_default();
                format!(
                    "{} · {} · {}",
                    failure.occurred_at_unix_ms,
                    terminal_status_name(locale, failure.status),
                    fingerprint,
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let terminal = &detail.terminal_breakdown;
    let body = format!(
        "{}: {} · {}: {} · {}: {}\n{}: {} · {}: {} · {}: {}\n\n{}\n{}",
        t(locale, MessageKey::FieldSuccesses),
        terminal.completed,
        t(locale, MessageKey::FieldFailures),
        detail.failed_runs,
        t(locale, MessageKey::FieldSamples),
        detail.sample_count,
        terminal_status_name(locale, crate::domain::TerminalStatus::Blocked),
        terminal.blocked,
        terminal_status_name(locale, crate::domain::TerminalStatus::Stopped),
        terminal.stopped,
        terminal_status_name(locale, crate::domain::TerminalStatus::Incomplete),
        terminal.incomplete + terminal.unknown,
        t(locale, MessageKey::SectionRecentFailures),
        recent,
    );
    frame.render_widget(
        Paragraph::new(body)
            .style(theme.typography_style(TypographyRole::Value))
            .block(themed_block(
                t(locale, MessageKey::SectionTerminalBreakdown),
                theme,
            ))
            .wrap(Wrap { trim: true }),
        sections[1],
    );
    render_state_panel(
        frame,
        sections[2],
        t(locale, MessageKey::SectionTimeline),
        t(locale, MessageKey::StateTimelineUnavailable),
        ColorRole::Info,
        theme,
    );
}

fn key_value_line(
    locale: ResolvedLocale,
    key: MessageKey,
    value: &str,
    theme: Theme,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{}: ", t(locale, key)),
            theme.typography_style(TypographyRole::FieldLabel),
        ),
        Span::styled(
            value.to_owned(),
            theme.typography_style(TypographyRole::Value),
        ),
    ])
}

fn status_line(locale: ResolvedLocale, health: Health, theme: Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{}: ", t(locale, MessageKey::FieldHealth)),
            theme.typography_style(TypographyRole::FieldLabel),
        ),
        Span::styled(
            health_name(locale, health),
            theme.color_style(health_color(health)),
        ),
    ])
}

fn display_identity(
    locale: ResolvedLocale,
    identity: &DisplayIdentity,
    disambiguator: Option<usize>,
) -> String {
    let name = match identity {
        DisplayIdentity::ExistingMetadata(value) => value.clone(),
        DisplayIdentity::EventFallback(event) => {
            format!(
                "{} {}",
                event_name(locale, *event),
                t(locale, MessageKey::IdentityHook)
            )
        }
    };
    match disambiguator {
        Some(index) => format!("{name} #{index}"),
        None => name,
    }
}

fn health_color(health: Health) -> ColorRole {
    match health {
        Health::Healthy => ColorRole::Success,
        Health::Degraded => ColorRole::Danger,
        Health::CoverageLimited | Health::NoTerminalSamples => ColorRole::Warning,
    }
}

fn diagnostic_color(status: DiagnosticStatus) -> ColorRole {
    match status {
        DiagnosticStatus::Pass => ColorRole::Success,
        DiagnosticStatus::Warning => ColorRole::Warning,
        DiagnosticStatus::Fail => ColorRole::Danger,
        DiagnosticStatus::Unknown | DiagnosticStatus::Unsupported => ColorRole::Info,
    }
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
    use crate::analytics::TimeWindow;
    use crate::diagnostics::{
        DiagnosticCheck, DiagnosticCheckId, DiagnosticStatus, DiagnosticsReport,
    };
    use crate::report::instrumented_report;
    use crate::report::synthetic_fixture_report;
    use ratatui::{Terminal, backend::TestBackend};

    fn rendered(app: App, locale: ResolvedLocale, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                draw(
                    frame,
                    &app,
                    LanguageState::resolve(
                        match locale {
                            ResolvedLocale::EnUs => {
                                super::super::localization::InterfaceLanguage::EnUs
                            }
                            ResolvedLocale::ZhCn => {
                                super::super::localization::InterfaceLanguage::ZhCn
                            }
                        },
                        None,
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
    fn normal_narrow_and_minimum_buffers_keep_sample_counts_or_resize_guidance() {
        let app = App::from_report(synthetic_fixture_report(1_000));
        for (width, height) in [(100, 30), (44, 16), (24, 10)] {
            assert!(rendered(app.clone(), ResolvedLocale::EnUs, width, height).contains("n="));
        }
        assert!(rendered(app, ResolvedLocale::EnUs, 23, 10).contains("Resize"));
    }

    #[test]
    fn chinese_overview_uses_the_catalog_and_preserves_semantic_state() {
        let rendered = rendered(
            App::from_report(synthetic_fixture_report(1_000)),
            ResolvedLocale::ZhCn,
            100,
            30,
        );
        let normalized = rendered.replace(' ', "");
        assert!(normalized.contains("样本="));
        assert!(normalized.contains("已降级"));
    }

    #[test]
    fn diagnostics_are_localized_and_explicitly_read_only() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.handle(super::super::keymap::Command::ToggleFocus);
        app.handle(super::super::keymap::Command::Down);
        app.handle(super::super::keymap::Command::Down);
        app.handle(super::super::keymap::Command::Enter);

        let english = rendered(app.clone(), ResolvedLocale::EnUs, 100, 30);
        assert!(english.contains("Read-only diagnostics"));
        assert!(english.contains("Unknown"));
        assert!(english.contains("Receipt integrity"));

        let chinese = rendered(app, ResolvedLocale::ZhCn, 100, 30);
        let normalized = chinese.replace(' ', "");
        assert!(normalized.contains("只读诊断"));
        assert!(normalized.contains("未知"));
        assert!(normalized.contains("回执完整性"));
    }

    #[test]
    fn representative_viewports_cover_empty_populated_and_degraded_states() {
        let populated = App::from_report(synthetic_fixture_report(1_000));
        assert!(rendered(populated.clone(), ResolvedLocale::EnUs, 100, 30).contains("Codex"));
        assert!(rendered(populated.clone(), ResolvedLocale::EnUs, 44, 16).contains("n="));
        assert!(rendered(populated, ResolvedLocale::EnUs, 100, 10).contains("n="));

        let mut empty =
            App::from_report(instrumented_report(&[], 1_000, TimeWindow::Last7Days, 0, 0));
        empty.handle(super::super::keymap::Command::ToggleFocus);
        empty.handle(super::super::keymap::Command::Down);
        empty.handle(super::super::keymap::Command::Enter);
        assert_eq!(empty.screen(), Screen::Hooks);
        let empty_buffer = rendered(empty, ResolvedLocale::ZhCn, 100, 30);
        assert!(empty_buffer.replace(' ', "").contains("尚无已接纳"));

        let diagnostics = DiagnosticsReport {
            schema_version: 1,
            read_only: true,
            generated_at_unix_ms: 1_000,
            overall_status: DiagnosticStatus::Fail,
            checks: vec![DiagnosticCheck {
                id: DiagnosticCheckId::ReceiptIntegrity,
                status: DiagnosticStatus::Fail,
                facts: vec![],
            }],
        };
        let mut degraded = App::from_snapshot(
            super::super::app::RefreshSnapshot::from_report_with_diagnostics(
                synthetic_fixture_report(1_000),
                diagnostics,
            ),
        );
        degraded.handle(super::super::keymap::Command::ToggleFocus);
        degraded.handle(super::super::keymap::Command::Down);
        degraded.handle(super::super::keymap::Command::Down);
        degraded.handle(super::super::keymap::Command::Enter);
        let rendered = rendered(degraded, ResolvedLocale::EnUs, 100, 30);
        assert!(rendered.contains("Fail"));
        assert!(rendered.contains("Receipt integrity"));
    }

    #[test]
    fn every_top_level_view_reflows_at_normal_narrow_and_minimum_sizes() {
        for steps in [0, 1, 2, 3] {
            let mut app = App::from_report(synthetic_fixture_report(1_000));
            app.handle(super::super::keymap::Command::ToggleFocus);
            for _ in 0..steps {
                app.handle(super::super::keymap::Command::Down);
            }
            app.handle(super::super::keymap::Command::Enter);
            for locale in [ResolvedLocale::EnUs, ResolvedLocale::ZhCn] {
                for (width, height) in [(100, 30), (44, 16), (24, 10)] {
                    assert!(rendered(app.clone(), locale, width, height).contains("HookStat"));
                }
            }
        }
    }
}
