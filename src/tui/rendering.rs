//! Pure Ratatui rendering over an accepted Reliability Center view model.

use crate::analytics::TimeWindow;
use crate::workbench::{ChangeKind, HistoricalStatus};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, Table, Wrap},
};

use super::app::{App, Screen, SettingsField, SettingsSaveState};
use super::layout::{ApplicationShell, ShellLayout};
use super::localization::{
    LanguageState, MessageKey, ResolvedLocale, coverage_name, diagnostic_explanation,
    diagnostic_status_name, diagnostic_title, event_name, failure_rate_with_sample,
    fingerprint_name, health_name, intelligence_availability_name, interface_color_name,
    interface_language_name, regression_name, runtime_name, sort_name, t, terminal_status_name,
    window_name,
};
use super::state::ResourceState;
use super::theme::{ColorRole, Theme, TypographyRole};
use super::view_model::{
    ChangeRef, ChangeRowViewModel, ChangesViewModel, DiagnosticCheckViewModel, DiagnosticFact,
    DiagnosticStatus, DiagnosticsViewModel, DisplayIdentity, Health, HookDetailViewModel,
    HookRowViewModel,
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
            super::widgets::render_title(
                frame,
                areas.title,
                language.resolved,
                overall_status(app, language.resolved),
                theme,
            );
            render_navigation(
                frame,
                areas.navigation,
                language.resolved,
                app.navigation(),
                theme,
            );
            render_content(frame, areas.content, app, language.resolved, theme);
            render_shortcut_footer(frame, areas.footer, language.resolved, app, theme);
            if app.help_open() {
                render_help_overlay(frame, areas.content, language.resolved, theme);
            } else if app.discard_confirmation_open() {
                render_discard_confirmation(frame, areas.content, language.resolved, theme);
            }
        }
    }
}

fn overall_status(app: &App, locale: ResolvedLocale) -> &'static str {
    app.view_model().map_or_else(
        || t(locale, MessageKey::StateLoading),
        |view| {
            view.overview.runtime_summaries.first().map_or_else(
                || t(locale, MessageKey::StatusUnavailable),
                |summary| health_name(locale, summary.health),
            )
        },
    )
}

fn render_help_overlay(frame: &mut Frame, area: Rect, locale: ResolvedLocale, theme: Theme) {
    let lines = [
        t(locale, MessageKey::HelpNavigation),
        t(locale, MessageKey::HelpPeriods),
        t(locale, MessageKey::HelpHooks),
        t(locale, MessageKey::HelpChanges),
        t(locale, MessageKey::HelpDetail),
        t(locale, MessageKey::HelpSettings),
        t(locale, MessageKey::HelpRefresh),
    ]
    .join("\n\n");
    frame.render_widget(
        Paragraph::new(lines)
            .block(themed_block(t(locale, MessageKey::HelpTitle), theme))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_discard_confirmation(
    frame: &mut Frame,
    area: Rect,
    locale: ResolvedLocale,
    theme: Theme,
) {
    let message = format!(
        "{}\n\nEsc {}  Enter {}",
        t(locale, MessageKey::StatePreferenceDirty),
        t(locale, MessageKey::FooterCancel),
        t(locale, MessageKey::FooterDiscard),
    );
    frame.render_widget(
        Paragraph::new(message)
            .block(themed_block(t(locale, MessageKey::FooterDiscard), theme))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_content(frame: &mut Frame, area: Rect, app: &App, locale: ResolvedLocale, theme: Theme) {
    match app.screen() {
        Screen::Changes => {
            render_changes(frame, area, app, locale, theme);
            return;
        }
        Screen::ChangeDetail => {
            render_change_detail(frame, area, app, locale, theme);
            return;
        }
        Screen::Overview
        | Screen::Hooks
        | Screen::Diagnostics
        | Screen::Settings
        | Screen::HookDetail => {}
    }
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
            &format!("{}\n{}", period_selector(app, locale), t(locale, message)),
            role,
            theme,
        );
        return;
    };

    match app.screen() {
        Screen::Overview => render_overview(frame, area, app, locale, theme),
        Screen::Hooks => render_hooks(frame, area, app, locale, theme),
        Screen::Changes | Screen::ChangeDetail => unreachable!("handled before reliability view"),
        Screen::Diagnostics => match app.diagnostics() {
            Some(diagnostics) => render_diagnostics(frame, area, diagnostics, locale, theme),
            None => {
                let (message, role) = match app.diagnostics_state() {
                    ResourceState::Error { .. } => {
                        (MessageKey::StateRefreshFailed, ColorRole::Danger)
                    }
                    ResourceState::Loading { .. }
                    | ResourceState::Empty
                    | ResourceState::Ready(_) => (MessageKey::StateLoading, ColorRole::Info),
                };
                render_state_panel(
                    frame,
                    area,
                    t(locale, MessageKey::ViewDiagnostics),
                    t(locale, message),
                    role,
                    theme,
                );
            }
        },
        Screen::Settings => render_settings(frame, area, app, locale, theme),
        Screen::HookDetail => {
            let detail = app
                .selected_handler()
                .and_then(|reference| view.detail(reference));
            match detail {
                Some(detail) => render_hook_detail(frame, area, app, detail, locale, theme),
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
        Line::from(Span::styled(
            period_selector(app, locale),
            theme.typography_style(TypographyRole::Metadata),
        )),
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
            content_focused: app.local_list_active(),
            locale,
            theme,
            title: t(locale, MessageKey::SectionRiskyHooks),
        },
    );
}

fn period_selector(app: &App, locale: ResolvedLocale) -> String {
    let periods = [
        (TimeWindow::Today, window_name(locale, TimeWindow::Today)),
        (TimeWindow::Last24Hours, "24h"),
        (TimeWindow::Last7Days, "7d"),
        (TimeWindow::Last30Days, "30d"),
        (TimeWindow::All, t(locale, MessageKey::PeriodAll)),
    ];
    let selected = app.requested_window();
    let mut text = periods
        .iter()
        .map(|(period, label)| {
            if *period == selected {
                format!("[{label}]")
            } else {
                (*label).to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" | ");
    if app.view_state().is_loading() {
        text.push_str(&format!(" ({})", t(locale, MessageKey::StateLoading)));
    }
    text
}

fn render_changes(frame: &mut Frame, area: Rect, app: &App, locale: ResolvedLocale, theme: Theme) {
    let Some(changes) = app.changes() else {
        let (message, role) = match app.changes_state() {
            ResourceState::Loading { .. } => (MessageKey::StateLoading, ColorRole::Info),
            ResourceState::Error { .. } => (MessageKey::StateRefreshFailed, ColorRole::Danger),
            ResourceState::Empty | ResourceState::Ready(_) => {
                (MessageKey::StateEmpty, ColorRole::Warning)
            }
        };
        render_state_panel(
            frame,
            area,
            t(locale, MessageKey::ViewChanges),
            &format!("{}\n{}", period_selector(app, locale), t(locale, message)),
            role,
            theme,
        );
        return;
    };
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);
    let heading = format!(
        "{} · {}: {}",
        period_selector(app, locale),
        t(locale, MessageKey::FieldCoverage),
        coverage_name(locale, changes.coverage),
    );
    frame.render_widget(
        Paragraph::new(truncate_to_width(&heading, sections[0].width as usize))
            .style(theme.typography_style(TypographyRole::Metadata))
            .block(themed_block(t(locale, MessageKey::SectionChanges), theme)),
        sections[0],
    );
    if changes.rows.is_empty() {
        render_state_panel(
            frame,
            sections[1],
            t(locale, MessageKey::ViewChanges),
            t(locale, MessageKey::StateEmpty),
            ColorRole::Warning,
            theme,
        );
        return;
    }
    render_change_rows(
        frame,
        sections[1],
        changes,
        ChangeRowsContext {
            selected: app.selected_change(),
            content_focused: app.local_list_active(),
            locale,
            theme,
        },
    );
}

struct ChangeRowsContext<'a> {
    selected: Option<&'a ChangeRef>,
    content_focused: bool,
    locale: ResolvedLocale,
    theme: Theme,
}

fn render_change_rows(
    frame: &mut Frame,
    area: Rect,
    changes: &ChangesViewModel,
    context: ChangeRowsContext<'_>,
) {
    let selected_index = context.selected.map_or(0, |selected| {
        changes
            .rows
            .iter()
            .position(|row| &row.reference == selected)
            .unwrap_or(0)
    });
    if area.width < 64 {
        let rows = visible_rows(
            &changes.rows,
            selected_index,
            area.height.saturating_sub(2) as usize / 3,
        );
        let content = rows
            .iter()
            .map(|row| {
                let marker = if context.selected == Some(&row.reference) {
                    ">"
                } else {
                    " "
                };
                let identity = display_identity(context.locale, &row.display_identity, None);
                format!(
                    "{marker} {}\n  {} · {}\n  {}",
                    truncate_to_width(&identity, area.width.saturating_sub(6) as usize),
                    change_kind_name(context.locale, row.reference.kind),
                    event_name(context.locale, row.event),
                    truncate_to_width(
                        &change_evidence_summary(context.locale, row),
                        area.width.saturating_sub(6) as usize,
                    ),
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        frame.render_widget(
            Paragraph::new(content)
                .style(context.theme.typography_style(TypographyRole::Value))
                .block(themed_block(
                    t(context.locale, MessageKey::ViewChanges),
                    context.theme,
                ))
                .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }
    let rows = visible_rows(
        &changes.rows,
        selected_index,
        area.height.saturating_sub(3) as usize,
    );
    let header = Row::new([
        t(context.locale, MessageKey::ColumnName),
        t(context.locale, MessageKey::FieldClassification),
        t(context.locale, MessageKey::ColumnEvent),
        t(context.locale, MessageKey::FieldFailureRate),
        t(context.locale, MessageKey::FieldPreviousPeriod),
    ])
    .style(context.theme.typography_style(TypographyRole::SectionTitle));
    let rows = rows.iter().map(|row| {
        let style = if context.selected == Some(&row.reference) && context.content_focused {
            context.theme.color_style(ColorRole::Selected)
        } else {
            context.theme.typography_style(TypographyRole::Value)
        };
        Row::new(vec![
            Cell::from(truncate_to_width(
                &display_identity(context.locale, &row.display_identity, None),
                24,
            )),
            Cell::from(change_kind_name(context.locale, row.reference.kind)),
            Cell::from(event_name(context.locale, row.event)),
            Cell::from(failure_rate_with_sample(
                context.locale,
                row.current.failure_rate_percent,
                row.current.failure_sample_count,
            )),
            Cell::from(change_comparison_summary(context.locale, row)),
        ])
        .style(style)
    });
    let table = Table::new(
        rows,
        [
            Constraint::Min(16),
            Constraint::Length(19),
            Constraint::Length(15),
            Constraint::Length(16),
            Constraint::Length(20),
        ],
    )
    .header(header)
    .block(themed_block(
        t(context.locale, MessageKey::ViewChanges),
        context.theme,
    ));
    frame.render_widget(table, area);
}

fn render_change_detail(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    locale: ResolvedLocale,
    theme: Theme,
) {
    let detail = app.changes().and_then(|changes| {
        app.selected_change()
            .and_then(|reference| changes.detail(reference))
    });
    let Some(detail) = detail else {
        render_state_panel(
            frame,
            area,
            t(locale, MessageKey::ViewChangeDetail),
            t(locale, MessageKey::StateEmpty),
            ColorRole::Warning,
            theme,
        );
        return;
    };
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(13), Constraint::Min(7)])
        .split(area);
    let identity = display_identity(locale, &detail.row.display_identity, None);
    let current_revision = detail
        .revision_timeline
        .last()
        .map(|epoch| epoch.revision.as_str())
        .unwrap_or_else(|| t(locale, MessageKey::StateTimelineUnavailable));
    let facts = vec![
        Line::from(Span::styled(
            period_selector(app, locale),
            theme.typography_style(TypographyRole::Metadata),
        )),
        key_value_line(
            locale,
            MessageKey::FieldClassification,
            change_kind_name(locale, detail.row.reference.kind),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldCoverage,
            coverage_name(locale, detail.coverage),
            theme,
        ),
        key_value_line(locale, MessageKey::FieldRevision, current_revision, theme),
        key_value_line(
            locale,
            MessageKey::FieldFirstSeen,
            &detail.first_seen_unix_ms.to_string(),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldLastSeen,
            &detail.last_seen_unix_ms.to_string(),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldLatestEvidence,
            &detail.latest_evidence_unix_ms.to_string(),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldFailureRate,
            &failure_rate_with_sample(
                locale,
                detail.row.current.failure_rate_percent,
                detail.row.current.failure_sample_count,
            ),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldPreviousPeriod,
            &change_comparison_summary(locale, &detail.row),
            theme,
        ),
    ];
    frame.render_widget(
        Paragraph::new(facts)
            .block(themed_block(&identity, theme))
            .wrap(Wrap { trim: true }),
        sections[0],
    );
    let timeline = detail
        .revision_timeline
        .iter()
        .map(|epoch| {
            format!(
                "{} · {}–{} · {}",
                epoch.revision,
                epoch.first_seen_unix_ms,
                epoch.last_seen_unix_ms,
                failure_rate_with_sample(
                    locale,
                    epoch.metrics.failure_rate_percent,
                    epoch.metrics.failure_sample_count,
                ),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let timeline = if timeline.is_empty() {
        t(locale, MessageKey::StateEmpty).to_owned()
    } else {
        timeline
    };
    frame.render_widget(
        Paragraph::new(timeline)
            .block(themed_block(t(locale, MessageKey::FieldRevision), theme))
            .scroll((app.changes_detail_scroll_lines(), 0))
            .wrap(Wrap { trim: true }),
        sections[1],
    );
}

fn change_kind_name(locale: ResolvedLocale, kind: ChangeKind) -> &'static str {
    let key = match kind {
        ChangeKind::Regression => MessageKey::ChangeRegression,
        ChangeKind::Recovery => MessageKey::ChangeRecovery,
        ChangeKind::RevisionChange => MessageKey::ChangeRevision,
        ChangeKind::NewAdmittedHook => MessageKey::ChangeNewHook,
        ChangeKind::HistoricalOnly => MessageKey::StateHistoricalOnly,
    };
    t(locale, key)
}

fn change_comparison_summary(locale: ResolvedLocale, row: &ChangeRowViewModel) -> String {
    row.previous.as_ref().map_or_else(
        || intelligence_availability_name(locale, row.availability).to_owned(),
        |previous| {
            failure_rate_with_sample(
                locale,
                previous.failure_rate_percent,
                previous.failure_sample_count,
            )
        },
    )
}

fn change_evidence_summary(locale: ResolvedLocale, row: &ChangeRowViewModel) -> String {
    let current = failure_rate_with_sample(
        locale,
        row.current.failure_rate_percent,
        row.current.failure_sample_count,
    );
    let qualifier = if row.historical_status == HistoricalStatus::HistoricalOutsideSelectedPeriod {
        t(locale, MessageKey::StateHistoricalOnly)
    } else if row.availability != crate::analytics::IntelligenceAvailability::Available {
        intelligence_availability_name(locale, row.availability)
    } else {
        change_kind_name(locale, row.reference.kind)
    };
    format!(
        "{qualifier} · {current} · {}",
        change_comparison_summary(locale, row)
    )
}

fn render_hooks(frame: &mut Frame, area: Rect, app: &App, locale: ResolvedLocale, theme: Theme) {
    let query = app.hooks_query();
    let filter = if query.failures_only {
        t(locale, MessageKey::FilterFailuresOnly)
    } else {
        t(locale, MessageKey::FilterAllHooks)
    };
    let query_line = format!(
        "{}\n{}: {} · {}: {} · {}: {}",
        period_selector(app, locale),
        t(locale, MessageKey::FieldSearch),
        query.search,
        t(locale, MessageKey::FieldFilter),
        filter,
        t(locale, MessageKey::FieldSort),
        sort_name(locale, query.sort),
    );
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(5)])
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
            content_focused: app.local_list_active(),
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
    let selected_index = context.selected.map_or(0, |selected| {
        rows.iter()
            .position(|row| &row.internal_ref == selected)
            .unwrap_or(0)
    });
    if area.width < 54 {
        let rows = visible_rows(
            rows,
            selected_index,
            area.height.saturating_sub(2) as usize / 2,
        );
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
                    "{marker} {}\n  {} · {} · {} · {} · {}",
                    truncate_to_width(&identity, area.width.saturating_sub(6) as usize),
                    event_name(context.locale, row.event),
                    runtime_name(context.locale, row.internal_ref.runtime),
                    failure_rate_with_sample(
                        context.locale,
                        row.failure_rate_percent,
                        row.sample_count,
                    ),
                    trend_summary(context.locale, row),
                    compact_risk(context.locale, row),
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
    let rows = visible_rows(rows, selected_index, area.height.saturating_sub(3) as usize);
    let header = Row::new([
        t(context.locale, MessageKey::ColumnName),
        t(context.locale, MessageKey::ColumnEvent),
        t(context.locale, MessageKey::ColumnRuntime),
        t(context.locale, MessageKey::ColumnFailureRate),
        t(context.locale, MessageKey::ColumnTrend),
        t(context.locale, MessageKey::ColumnRisk),
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
            Cell::from(trend_summary(context.locale, row)),
            Cell::from(compact_risk(context.locale, row)),
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
    app: &App,
    detail: &HookDetailViewModel,
    locale: ResolvedLocale,
    theme: Theme,
) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(12), Constraint::Min(9)])
        .split(area);
    let identity = display_identity(
        locale,
        &detail.display_identity,
        detail.display_disambiguator,
    );
    let facts = vec![
        Line::from(Span::styled(
            period_selector(app, locale),
            theme.typography_style(TypographyRole::Metadata),
        )),
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
        key_value_line(
            locale,
            MessageKey::FieldRisk,
            &risk_detail(locale, &detail.risk),
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
    let trends = detail
        .trends
        .iter()
        .map(|trend| trend_detail(locale, trend))
        .collect::<Vec<_>>()
        .join("\n");
    let revision = revision_detail(locale, &detail.revision_comparison);
    let fingerprints = if detail.failure_fingerprints.is_empty() {
        t(locale, MessageKey::StateNoRecentFailures).to_owned()
    } else {
        detail
            .failure_fingerprints
            .iter()
            .map(|cluster| {
                format!(
                    "{}: {}",
                    fingerprint_name(locale, cluster.kind),
                    cluster.occurrences,
                )
            })
            .collect::<Vec<_>>()
            .join(" · ")
    };
    let body = format!(
        "{}: {} · {}: {} · {}: {}\n{}: {} · {}: {} · {}: {}\n\n{}\n{}\n\n{}\n{}\n\n{}\n{}\n\n{}\n{}",
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
        t(locale, MessageKey::SectionTrends),
        trends,
        t(locale, MessageKey::SectionRevisionComparison),
        revision,
        t(locale, MessageKey::SectionFailureFingerprints),
        fingerprints,
    );
    if area.height < 18 {
        let compact = format!(
            "{identity}\n{}\n{}\n{}\n{body}",
            key_value_text(
                locale,
                MessageKey::FieldRuntime,
                runtime_name(locale, detail.internal_ref.runtime),
            ),
            key_value_text(
                locale,
                MessageKey::FieldCoverage,
                coverage_name(locale, detail.coverage),
            ),
            key_value_text(
                locale,
                MessageKey::FieldFailureRate,
                &failure_rate_with_sample(locale, detail.failure_rate_percent, detail.sample_count),
            ),
        );
        frame.render_widget(
            Paragraph::new(compact)
                .style(theme.typography_style(TypographyRole::Value))
                .block(themed_block(t(locale, MessageKey::ViewHookDetail), theme))
                .wrap(Wrap { trim: true })
                .scroll((app.detail_scroll_lines(), 0)),
            area,
        );
        return;
    }
    frame.render_widget(
        Paragraph::new(body)
            .style(theme.typography_style(TypographyRole::Value))
            .block(themed_block(
                t(locale, MessageKey::SectionIntelligence),
                theme,
            ))
            .wrap(Wrap { trim: true })
            .scroll((app.detail_scroll_lines(), 0)),
        sections[1],
    );
}

fn visible_rows<T>(rows: &[T], selected_index: usize, capacity: usize) -> &[T] {
    let capacity = capacity.max(1);
    let start = selected_index
        .saturating_sub(capacity.saturating_sub(1))
        .min(rows.len().saturating_sub(capacity));
    let end = start.saturating_add(capacity).min(rows.len());
    &rows[start..end]
}

fn key_value_text(locale: ResolvedLocale, key: MessageKey, value: &str) -> String {
    format!("{}: {value}", t(locale, key))
}

fn trend_summary(locale: ResolvedLocale, row: &HookRowViewModel) -> &'static str {
    match row.trend.classification {
        crate::analytics::RegressionClassification::InsufficientEvidence => {
            intelligence_availability_name(locale, row.trend.availability)
        }
        classification => regression_name(locale, classification),
    }
}

fn compact_risk(locale: ResolvedLocale, row: &HookRowViewModel) -> String {
    format!("{} {}", t(locale, MessageKey::FieldRisk), row.risk.score)
}

fn risk_detail(locale: ResolvedLocale, risk: &crate::analytics::RiskScore) -> String {
    format!(
        "{} {} · {} {}% · {} {}% · {} {:+} · {} {:+} · {} {:+}",
        t(locale, MessageKey::FieldRiskScore),
        risk.score,
        t(locale, MessageKey::FieldConfidence),
        risk.sample_confidence_percent,
        t(locale, MessageKey::FieldCoverage),
        risk.coverage_multiplier_percent,
        t(locale, MessageKey::FieldRecency),
        risk.recency_points,
        t(locale, MessageKey::ColumnTrend),
        risk.trend_points,
        t(locale, MessageKey::FieldImpact),
        risk.impact_points,
    )
}

fn trend_detail(locale: ResolvedLocale, trend: &crate::analytics::TrendProjection) -> String {
    let current = failure_rate_with_sample(
        locale,
        trend.current.failure_rate_percent,
        trend.current.failure_sample_count,
    );
    let comparison = match &trend.previous {
        Some(previous) => format!(
            "{} {}",
            t(locale, MessageKey::FieldPreviousPeriod),
            failure_rate_with_sample(
                locale,
                previous.failure_rate_percent,
                previous.failure_sample_count,
            ),
        ),
        None => intelligence_availability_name(locale, trend.availability).to_owned(),
    };
    format!(
        "{}: {current} · {comparison} · {}",
        window_name(locale, trend.window),
        trend_summary_from_projection(locale, trend),
    )
}

fn trend_summary_from_projection(
    locale: ResolvedLocale,
    trend: &crate::analytics::TrendProjection,
) -> &'static str {
    match trend.classification {
        crate::analytics::RegressionClassification::InsufficientEvidence => {
            intelligence_availability_name(locale, trend.availability)
        }
        classification => regression_name(locale, classification),
    }
}

fn revision_detail(
    locale: ResolvedLocale,
    comparison: &crate::analytics::RevisionComparison,
) -> String {
    let current = format!(
        "{} {} · {}",
        t(locale, MessageKey::FieldRevision),
        comparison.current.revision,
        failure_rate_with_sample(
            locale,
            comparison.current.failure_rate_percent,
            comparison.current.failure_sample_count,
        ),
    );
    let previous = comparison.previous.as_ref().map_or_else(
        || intelligence_availability_name(locale, comparison.availability).to_owned(),
        |previous| {
            format!(
                "{} {} · {}",
                t(locale, MessageKey::FieldPreviousPeriod),
                previous.revision,
                failure_rate_with_sample(
                    locale,
                    previous.failure_rate_percent,
                    previous.failure_sample_count,
                ),
            )
        },
    );
    format!(
        "{current}\n{previous}\n{}: {}",
        t(locale, MessageKey::FieldClassification),
        trend_summary_from_classification(
            locale,
            comparison.classification,
            comparison.availability
        ),
    )
}

fn trend_summary_from_classification(
    locale: ResolvedLocale,
    classification: crate::analytics::RegressionClassification,
    availability: crate::analytics::IntelligenceAvailability,
) -> &'static str {
    match classification {
        crate::analytics::RegressionClassification::InsufficientEvidence => {
            intelligence_availability_name(locale, availability)
        }
        classification => regression_name(locale, classification),
    }
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
    use crate::domain::{
        EvidenceCoverage, EvidenceKind, ExecutionMode, HandlerIdentity, HookEvent, HookInvocation,
        Runtime, TerminalStatus,
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

    fn changes_app() -> App {
        const DAY: i64 = 24 * 60 * 60 * 1_000;
        let now = 20 * DAY;
        let values = (0..12)
            .map(|id| HookInvocation {
                source_key: "fixture".into(),
                source_record_id: format!("change-{id}"),
                runtime: Runtime::Codex,
                evidence_kind: EvidenceKind::SyntheticFixture,
                coverage: EvidenceCoverage::Complete,
                handler: HandlerIdentity {
                    key: "hk_changes".into(),
                    revision: if id < 6 { "r1" } else { "r2" }.into(),
                    label: "Deployment Stop Hook".into(),
                    source_kind: "fixture".into(),
                    event: HookEvent::Stop,
                    matcher_identity: "fixture".into(),
                    structural_identity: "fixture".into(),
                    execution_mode: ExecutionMode::Sync,
                },
                occurred_at_unix_ms: if id < 6 {
                    now - 8 * DAY + id as i64
                } else {
                    now - DAY + id as i64
                },
                terminal_status: if id < 6 {
                    TerminalStatus::Completed
                } else {
                    TerminalStatus::Failed
                },
                duration_ms: None,
                error_fingerprint: None,
            })
            .collect::<Vec<_>>();
        let mut app = App::from_report(synthetic_fixture_report(now));
        app.handle(super::super::keymap::Command::Down);
        app.handle(super::super::keymap::Command::Down);
        app.apply_changes(super::super::app::ChangesSnapshot::from_values(
            values,
            now,
            TimeWindow::Last7Days,
            EvidenceCoverage::Complete,
        ));
        app
    }

    #[test]
    fn normal_narrow_and_minimum_buffers_keep_sample_counts_or_resize_guidance() {
        let app = App::from_report(synthetic_fixture_report(1_000));
        for (width, height) in [(100, 30), (44, 16)] {
            assert!(rendered(app.clone(), ResolvedLocale::EnUs, width, height).contains("n="));
        }
        assert!(rendered(app.clone(), ResolvedLocale::EnUs, 24, 10).contains("HookStat"));
        assert!(rendered(app, ResolvedLocale::EnUs, 23, 10).contains("Resize"));
    }

    #[test]
    fn shared_shell_uses_two_row_header_sections_marker_and_contextual_footer() {
        let app = App::from_report(synthetic_fixture_report(1_000));
        let english = rendered(app, ResolvedLocale::EnUs, 100, 30);
        assert!(english.contains("HookStat Reliability Center —"));
        assert!(english.contains("Sections"));
        assert!(english.contains("> Overview"));
        assert!(!english.contains("•"));
        assert!(english.contains("↑↓ navigate  Enter open  ? help  r refresh  q quit"));
    }

    #[test]
    fn help_overlay_replaces_content_and_uses_active_locale() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.handle(super::super::keymap::Command::Help);
        let english = rendered(app.clone(), ResolvedLocale::EnUs, 100, 30);
        assert!(english.contains("Help"));
        assert!(english.contains("Periods: t Today"));
        assert!(english.contains("Esc/?/q dismiss"));
        let chinese = rendered(app, ResolvedLocale::ZhCn, 100, 30).replace(' ', "");
        assert!(chinese.contains("帮助"));
        assert!(chinese.contains("周期：t今天"));
        assert!(chinese.contains("变更：Enter"));
    }

    #[test]
    fn loading_shell_draws_before_data_and_marks_pending_today_immediately() {
        let mut app = App::loading(TimeWindow::Last7Days);
        let before = rendered(app.clone(), ResolvedLocale::EnUs, 100, 30);
        assert!(before.contains("Loading"));
        assert!(before.contains("[7d]"));
        app.handle(super::super::keymap::Command::Window(TimeWindow::Today));
        let after = rendered(app, ResolvedLocale::ZhCn, 100, 30).replace(' ', "");
        assert!(after.contains("[今天]"));
        assert!(after.contains("正在加载"));
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
    fn intelligence_detail_is_bilingual_and_keeps_rates_with_samples() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.handle(super::super::keymap::Command::Enter);
        assert_eq!(app.screen(), Screen::HookDetail);

        let english = rendered(app.clone(), ResolvedLocale::EnUs, 100, 40);
        assert!(english.contains("Reliability intelligence"));
        assert!(english.contains("Trends"));
        assert!(english.contains("Risk score"));
        assert!(english.contains("n="));

        let chinese = rendered(app, ResolvedLocale::ZhCn, 100, 40).replace(' ', "");
        assert!(chinese.contains("可靠性智能"));
        assert!(chinese.contains("趋势"));
        assert!(chinese.contains("风险评分"));
        assert!(chinese.contains("样本="));
    }

    #[test]
    fn diagnostics_are_localized_and_explicitly_read_only() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.handle(super::super::keymap::Command::Down);
        app.handle(super::super::keymap::Command::Down);
        app.handle(super::super::keymap::Command::Down);

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
        degraded.handle(super::super::keymap::Command::Down);
        degraded.handle(super::super::keymap::Command::Down);
        degraded.handle(super::super::keymap::Command::Down);
        let rendered = rendered(degraded, ResolvedLocale::EnUs, 100, 30);
        assert!(rendered.contains("Fail"));
        assert!(rendered.contains("Receipt integrity"));
    }

    #[test]
    fn every_top_level_view_reflows_at_normal_narrow_and_minimum_sizes() {
        for steps in [0, 1, 2, 3, 4] {
            let mut app = App::from_report(synthetic_fixture_report(1_000));
            for _ in 0..steps {
                app.handle(super::super::keymap::Command::Down);
            }
            for locale in [ResolvedLocale::EnUs, ResolvedLocale::ZhCn] {
                for (width, height) in [(100, 30), (44, 16), (24, 10)] {
                    assert!(rendered(app.clone(), locale, width, height).contains("HookStat"));
                }
            }
        }
    }

    #[test]
    fn changes_render_populated_narrow_and_drill_down_in_both_locales() {
        let mut app = changes_app();
        let english = rendered(app.clone(), ResolvedLocale::EnUs, 100, 30);
        assert!(english.contains("Changes"));
        assert!(english.contains("Regression"));
        assert!(english.contains("n=6"));

        let narrow = rendered(app.clone(), ResolvedLocale::EnUs, 44, 16);
        assert!(narrow.contains("Deployment"));
        app.handle(super::super::keymap::Command::Enter);
        app.handle(super::super::keymap::Command::Enter);
        assert_eq!(app.screen(), Screen::ChangeDetail);
        let detail = rendered(app, ResolvedLocale::ZhCn, 100, 30).replace(' ', "");
        assert!(detail.contains("首次发现"));
        assert!(detail.contains("最后发现"));
        assert!(detail.contains("修订版本"));
    }

    #[test]
    fn selected_rows_remain_inside_a_short_viewport() {
        let rows = (0..8).collect::<Vec<_>>();
        assert_eq!(visible_rows(&rows, 0, 3), &[0, 1, 2]);
        assert_eq!(visible_rows(&rows, 5, 3), &[3, 4, 5]);
        assert_eq!(visible_rows(&rows, 7, 3), &[5, 6, 7]);
        assert!(visible_rows::<usize>(&[], 0, 3).is_empty());
    }
}
