//! Pure Ratatui rendering over an accepted Reliability Center view model.

use crate::analytics::TimeWindow;
use crate::runtime_presentation::{
    ReliabilityJoinState, RuntimeCatalogIssue, RuntimeCatalogIssueSeverity,
    RuntimeEventPresentation, RuntimeHandlerKind, RuntimeHandlerMode, RuntimeHandlerPresentation,
    RuntimeTrust,
};
use crate::workbench::{ChangeKind, HistoricalStatus};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, Table, Wrap},
};

use super::app::{AliasSaveState, App, Screen, SettingsField, SettingsSaveState};
use super::human_time::format_human_time;
use super::layout::{ApplicationShell, ShellLayout};
use super::localization::{
    LanguageState, MessageKey, ResolvedLocale, coverage_name, diagnostic_explanation,
    diagnostic_status_name, diagnostic_title, event_name, failure_rate_with_sample,
    fingerprint_name, health_name, intelligence_availability_name, interface_color_name,
    interface_language_name, regression_name, runtime_name, t, terminal_status_name, window_name,
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
        Screen::FailureClusters => {
            render_failure_clusters(frame, area, app, locale, theme);
            return;
        }
        Screen::FailureClusterDetail => {
            render_failure_cluster_detail(frame, area, app, locale, theme);
            return;
        }
        Screen::Hooks => {
            render_hooks(frame, area, app, locale, theme);
            return;
        }
        Screen::HookDetail if app.runtime_hook_detail_active() => {
            render_runtime_hook_detail(frame, area, app, locale, theme);
            return;
        }
        Screen::Overview | Screen::Diagnostics | Screen::Settings | Screen::HookDetail => {}
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
        Screen::Changes
        | Screen::ChangeDetail
        | Screen::FailureClusters
        | Screen::FailureClusterDetail => unreachable!("handled before reliability view"),
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

fn alias_save_message(locale: ResolvedLocale, state: AliasSaveState) -> &'static str {
    let key = match state {
        AliasSaveState::Clean => MessageKey::StateAliasClean,
        AliasSaveState::Dirty => MessageKey::StateAliasDirty,
        AliasSaveState::Saved => MessageKey::StateAliasSaved,
        AliasSaveState::Conflict => MessageKey::StateAliasConflict,
        AliasSaveState::Failed => MessageKey::StateAliasSaveFailed,
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
                coverage_summary(locale, *coverage),
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
        .constraints([
            Constraint::Length(if area.width < 72 { 14 } else { 11 }),
            Constraint::Min(5),
        ])
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
            period_selector_for_window(locale, view.overview.window, app.view_state().is_loading()),
            theme.typography_style(TypographyRole::Metadata),
        )),
        label_value_line(
            t(locale, MessageKey::FieldMetricScope),
            &selected_scope(locale, view.overview.window),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldRuntime,
            runtime_name(locale, summary.runtime),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldCoverage,
            &coverage_summary(locale, summary.coverage),
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
            compact_scroll_lines: 0,
        },
    );
}

fn period_selector(app: &App, locale: ResolvedLocale) -> String {
    period_selector_for_window(
        locale,
        app.requested_window(),
        app.view_state().is_loading(),
    )
}

fn accepted_window(app: &App) -> TimeWindow {
    app.view_model()
        .map(|view| view.overview.window)
        .unwrap_or_else(|| app.requested_window())
}

fn period_selector_for_window(
    locale: ResolvedLocale,
    selected: TimeWindow,
    is_loading: bool,
) -> String {
    let periods = [
        (TimeWindow::Today, window_name(locale, TimeWindow::Today)),
        (TimeWindow::Last24Hours, "24h"),
        (TimeWindow::Last7Days, "7d"),
        (TimeWindow::Last30Days, "30d"),
        (TimeWindow::All, t(locale, MessageKey::PeriodAll)),
    ];
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
    if is_loading {
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
            &format!(
                "{}\n{}: {}\n{}",
                period_selector_for_window(
                    locale,
                    app.requested_window(),
                    app.changes_state().is_loading(),
                ),
                t(locale, MessageKey::FieldMetricScope),
                selected_scope(locale, app.requested_window()),
                t(locale, message),
            ),
            role,
            theme,
        );
        return;
    };
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(if area.width < 72 { 9 } else { 6 }),
            Constraint::Min(5),
        ])
        .split(area);
    let heading = format!(
        "{}\n{}: {}\n{}: {}",
        period_selector_for_window(locale, changes.window, app.changes_state().is_loading()),
        t(locale, MessageKey::FieldMetricScope),
        selected_scope(locale, changes.window),
        t(locale, MessageKey::FieldCoverage),
        coverage_summary(locale, changes.coverage),
    );
    frame.render_widget(
        Paragraph::new(heading)
            .style(theme.typography_style(TypographyRole::Metadata))
            .block(themed_block(t(locale, MessageKey::SectionChanges), theme))
            .wrap(Wrap { trim: true }),
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
    let identity = display_identity(locale, &detail.row.display_identity, None);
    let current_revision = detail
        .revision_timeline
        .last()
        .map(|epoch| epoch.revision.as_str())
        .unwrap_or_else(|| t(locale, MessageKey::StateTimelineUnavailable));
    let mut facts = vec![
        Line::from(Span::styled(
            period_selector_for_window(locale, detail.window, app.changes_state().is_loading()),
            theme.typography_style(TypographyRole::Metadata),
        )),
        label_value_line(
            t(locale, MessageKey::FieldMetricScope),
            &selected_scope(locale, detail.window),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldClassification,
            change_kind_name(locale, detail.row.reference.kind),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldCoverage,
            &coverage_summary(locale, detail.coverage),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldRevision,
            &short_revision(current_revision),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldChangeOccurred,
            &format_human_time(
                locale,
                detail.row.reference.occurred_at_unix_ms,
                changes_now(app),
            ),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldFirstSeen,
            &format_human_time(locale, detail.first_seen_unix_ms, changes_now(app)),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldLastSeen,
            &format_human_time(locale, detail.last_seen_unix_ms, changes_now(app)),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldLatestEvidence,
            &format_human_time(locale, detail.latest_evidence_unix_ms, changes_now(app)),
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
    let timeline_rows = detail
        .revision_timeline
        .iter()
        .map(|epoch| {
            format!(
                "{} · {} – {} · {}",
                short_revision(&epoch.revision),
                format_human_time(locale, epoch.first_seen_unix_ms, changes_now(app)),
                format_human_time(locale, epoch.last_seen_unix_ms, changes_now(app)),
                failure_rate_with_sample(
                    locale,
                    epoch.metrics.failure_rate_percent,
                    epoch.metrics.failure_sample_count,
                ),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let timeline_rows = if timeline_rows.is_empty() {
        t(locale, MessageKey::StateEmpty).to_owned()
    } else {
        timeline_rows
    };
    let timeline = format!(
        "{}\n{}: {}\n{}: {}\n{}: {}\n\n{timeline_rows}",
        t(locale, MessageKey::SectionTechnicalMetadata),
        t(locale, MessageKey::FieldInternalIdentity),
        detail.internal_ref.handler_key,
        t(locale, MessageKey::FieldFullRevision),
        current_revision,
        t(locale, MessageKey::FieldMetricScope),
        revision_timeline_scope(locale),
    );
    if area.height < 26 || area.width < 72 {
        facts.push(Line::from(""));
        facts.push(Line::from(t(locale, MessageKey::SectionTimeline)));
        facts.extend(timeline.lines().map(|line| Line::from(line.to_owned())));
        frame.render_widget(
            Paragraph::new(facts)
                .block(themed_block(&identity, theme))
                .wrap(Wrap { trim: true })
                .scroll((app.changes_detail_scroll_lines(), 0)),
            area,
        );
        return;
    }
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(20), Constraint::Min(6)])
        .split(area);
    frame.render_widget(
        Paragraph::new(facts)
            .block(themed_block(&identity, theme))
            .wrap(Wrap { trim: true }),
        sections[0],
    );
    frame.render_widget(
        Paragraph::new(timeline)
            .block(themed_block(t(locale, MessageKey::SectionTimeline), theme))
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
    let Some(catalog) = app.runtime_catalog() else {
        let message = if app.runtime_catalog_loading() {
            MessageKey::StateRuntimeCatalogLoading
        } else if app.runtime_catalog_error() {
            MessageKey::StateRuntimeCatalogUnavailable
        } else {
            MessageKey::StateRuntimeCatalogLoading
        };
        render_state_panel(
            frame,
            area,
            t(locale, MessageKey::ViewHooks),
            t(locale, message),
            if app.runtime_catalog_error() {
                ColorRole::Danger
            } else {
                ColorRole::Info
            },
            theme,
        );
        return;
    };

    if app.hooks_handlers_active()
        && let Some(event) = app.selected_runtime_event()
    {
        render_runtime_handlers(frame, area, app, event, locale, theme);
        return;
    }
    render_runtime_events(
        frame,
        area,
        app,
        &catalog.events,
        &catalog.issues,
        locale,
        theme,
    );
}

fn render_runtime_events(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    events: &[RuntimeEventPresentation],
    issues: &[RuntimeCatalogIssue],
    locale: ResolvedLocale,
    theme: Theme,
) {
    let mut notices = runtime_resource_notices(app, locale);
    let issue_text = issues
        .iter()
        .map(|issue| {
            let severity = match issue.severity {
                RuntimeCatalogIssueSeverity::Warning => t(locale, MessageKey::RuntimeIssueWarning),
                RuntimeCatalogIssueSeverity::Error => t(locale, MessageKey::RuntimeIssueError),
            };
            format!("{severity}: {}", issue.human_message)
        })
        .collect::<Vec<_>>()
        .join(" · ");
    if !issue_text.is_empty() {
        notices.push(issue_text);
    }
    let notice = notices.join(" · ");
    if area.width < 88 {
        let rows_per_viewport = usize::from(area.height.saturating_sub(4) / 5).max(1);
        let (start, end) = visible_list_window(
            app.runtime_event_selection_index(),
            events.len(),
            rows_per_viewport,
        );
        let body = events[start..end]
            .iter()
            .map(|event| {
                let selected = app.selected_runtime_event().is_some_and(|current| {
                    current.runtime_context == event.runtime_context
                        && current.runtime_event_name == event.runtime_event_name
                });
                let marker = if selected && app.hooks_events_active() {
                    ">"
                } else {
                    " "
                };
                format!(
                    "{marker} {}\n  {}: {} · {}: {} · {}: {}\n  {}: {}\n  {}",
                    runtime_event_name(locale, event),
                    t(locale, MessageKey::FieldInstalled),
                    event.installed_count(),
                    t(locale, MessageKey::FieldActive),
                    event.active_count(),
                    t(locale, MessageKey::FieldReview),
                    event.needs_review_count(),
                    t(locale, MessageKey::FieldHealth),
                    runtime_event_health(locale, app, event),
                    event.description.as_deref().unwrap_or("—"),
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        frame.render_widget(
            Paragraph::new(format!(
                "{}\n{}{}",
                t(locale, MessageKey::ViewHooks),
                if app.hooks_events_active() {
                    t(locale, MessageKey::HintOpenHandlers)
                } else {
                    t(locale, MessageKey::HintFocusEvents)
                },
                if notice.is_empty() {
                    String::new()
                } else {
                    format!("\n{}", notice)
                },
            ))
            .block(themed_block(t(locale, MessageKey::ViewHooks), theme))
            .wrap(Wrap { trim: true }),
            Rect { height: 4, ..area },
        );
        frame.render_widget(
            Paragraph::new(body)
                .style(theme.typography_style(TypographyRole::Value))
                .block(themed_block(t(locale, MessageKey::ViewHooks), theme))
                .wrap(Wrap { trim: true }),
            Rect {
                y: area.y.saturating_add(4),
                height: area.height.saturating_sub(4),
                ..area
            },
        );
        return;
    }
    let header = Row::new([
        t(locale, MessageKey::ColumnEvent),
        t(locale, MessageKey::ColumnInstalled),
        t(locale, MessageKey::ColumnActive),
        t(locale, MessageKey::ColumnReview),
        t(locale, MessageKey::ColumnHealth),
        t(locale, MessageKey::ColumnDescription),
    ])
    .style(theme.typography_style(TypographyRole::SectionTitle));
    let rows_per_viewport = usize::from(area.height.saturating_sub(3)).max(1);
    let (start, end) = visible_list_window(
        app.runtime_event_selection_index(),
        events.len(),
        rows_per_viewport,
    );
    let rows = events[start..end].iter().map(|event| {
        let selected = app.selected_runtime_event().is_some_and(|current| {
            current.runtime_context == event.runtime_context
                && current.runtime_event_name == event.runtime_event_name
        });
        let style = if selected && app.hooks_events_active() {
            theme.color_style(ColorRole::Selected)
        } else {
            theme.typography_style(TypographyRole::Value)
        };
        Row::new([
            Cell::from(runtime_event_name(locale, event)),
            Cell::from(event.installed_count().to_string()),
            Cell::from(event.active_count().to_string()),
            Cell::from(event.needs_review_count().to_string()),
            Cell::from(runtime_event_health(locale, app, event)),
            Cell::from(event.description.as_deref().unwrap_or("—")),
        ])
        .style(style)
    });
    let table = Table::new(
        rows,
        [
            Constraint::Length(22),
            Constraint::Length(11),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(24),
            Constraint::Min(24),
        ],
    )
    .header(header)
    .block(themed_block(t(locale, MessageKey::ViewHooks), theme));
    frame.render_widget(table, area);
    if !notice.is_empty() {
        render_notice(
            frame,
            area,
            &format!("{}: {notice}", t(locale, MessageKey::SectionRuntimeIssues)),
            ColorRole::Warning,
            theme,
        );
    }
}

fn render_runtime_handlers(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    event: &RuntimeEventPresentation,
    locale: ResolvedLocale,
    theme: Theme,
) {
    let title = format!(
        "{} — {}",
        t(locale, MessageKey::ViewHooks),
        runtime_event_name(locale, event)
    );
    if event.handlers.is_empty() {
        render_state_panel(
            frame,
            area,
            &title,
            &format!("{}: 0", t(locale, MessageKey::FieldInstalled)),
            ColorRole::Info,
            theme,
        );
        return;
    }
    let rows_per_viewport = usize::from(area.height / 4).max(1);
    let (start, end) = visible_list_window(
        app.runtime_handler_selection_index(),
        event.handlers.len(),
        rows_per_viewport,
    );
    let body = event
        .handlers
        .iter()
        .enumerate()
        .skip(start)
        .take(end - start)
        .map(|(index, handler)| {
            let marker = if app
                .selected_runtime_handler()
                .is_some_and(|current| current.runtime_catalog_id == handler.runtime_catalog_id)
            {
                ">"
            } else {
                " "
            };
            let state = if handler.managed {
                "M"
            } else if handler.needs_review {
                "!"
            } else if handler.enabled {
                "x"
            } else {
                " "
            };
            format!(
                "{marker}[{state}] {}\n  {}: {} · {} · {} · {}\n  {}: {} · {}: {}",
                runtime_handler_label(locale, index, handler),
                t(locale, MessageKey::FieldEnabled),
                if handler.enabled {
                    t(locale, MessageKey::FieldEnabled)
                } else {
                    t(locale, MessageKey::FieldDisabled)
                },
                handler.source.as_deref().unwrap_or("—"),
                runtime_handler_kind(locale, &handler.handler_kind),
                runtime_handler_mode(locale, handler.mode),
                t(locale, MessageKey::FieldTrust),
                runtime_trust(locale, handler.trust),
                t(locale, MessageKey::FieldHealth),
                runtime_handler_health(locale, app, event, handler),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let body = prepend_runtime_resource_notices(body, app, locale);
    frame.render_widget(
        Paragraph::new(body)
            .style(theme.typography_style(TypographyRole::Value))
            .block(themed_block(&title, theme))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_runtime_hook_detail(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    locale: ResolvedLocale,
    theme: Theme,
) {
    let Some(event) = app.selected_runtime_event() else {
        return;
    };
    let Some(handler) = app.selected_runtime_handler() else {
        return;
    };
    let handler_type = runtime_handler_kind(locale, &handler.handler_kind);
    let mut configuration = vec![
        key_value_text(
            locale,
            MessageKey::FieldEvent,
            &runtime_event_name(locale, event),
        ),
        key_value_text(
            locale,
            MessageKey::FieldEnabled,
            if handler.enabled { "true" } else { "false" },
        ),
        key_value_text(
            locale,
            MessageKey::FieldManaged,
            if handler.managed { "true" } else { "false" },
        ),
        key_value_text(
            locale,
            MessageKey::FieldNeedsReview,
            if handler.needs_review {
                "true"
            } else {
                "false"
            },
        ),
        key_value_text(
            locale,
            MessageKey::FieldTrust,
            runtime_trust(locale, handler.trust),
        ),
        key_value_text(locale, MessageKey::FieldHandlerType, &handler_type),
    ];
    if let Some(value) = handler.matcher.as_deref() {
        configuration.push(key_value_text(locale, MessageKey::FieldMatcher, value));
    }
    if handler.source.is_some() || handler.source_path.is_some() {
        let source = match (handler.source.as_deref(), handler.source_path.as_deref()) {
            (Some(source), Some(path)) if source != path => format!("{source} · {path}"),
            (Some(source), _) => source.to_owned(),
            (None, Some(path)) => path.to_owned(),
            (None, None) => String::new(),
        };
        configuration.push(key_value_text(locale, MessageKey::FieldSource, &source));
    }
    match &handler.handler_kind {
        RuntimeHandlerKind::Command { command } => {
            configuration.push(key_value_text(locale, MessageKey::FieldCommand, command));
        }
        RuntimeHandlerKind::McpTool { server, tool } => {
            configuration.push(key_value_text(locale, MessageKey::FieldMcpServer, server));
            configuration.push(key_value_text(locale, MessageKey::FieldMcpTool, tool));
        }
        RuntimeHandlerKind::Prompt => configuration.push(key_value_text(
            locale,
            MessageKey::FieldPrompt,
            &handler_type,
        )),
        RuntimeHandlerKind::Agent => configuration.push(key_value_text(
            locale,
            MessageKey::FieldAgent,
            &handler_type,
        )),
        RuntimeHandlerKind::Unknown { .. } => {}
    }
    if handler.mode.is_some() {
        configuration.push(key_value_text(
            locale,
            MessageKey::FieldMode,
            runtime_handler_mode(locale, handler.mode),
        ));
    }
    if let Some(value) = handler.timeout_seconds {
        configuration.push(key_value_text(
            locale,
            MessageKey::FieldTimeout,
            &format!("{value}s"),
        ));
    }
    if let Some(value) = handler.additional_context_limit {
        configuration.push(key_value_text(
            locale,
            MessageKey::FieldAdditionalContext,
            &value.to_string(),
        ));
    }

    let reliability = if let Some(detail) = app.matched_reliability_detail() {
        format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
            key_value_text(
                locale,
                MessageKey::FieldCoverage,
                &coverage_summary(locale, detail.coverage)
            ),
            key_value_text(
                locale,
                MessageKey::FieldWindow,
                window_name(locale, detail.window)
            ),
            key_value_text(
                locale,
                MessageKey::FieldMetricScope,
                &selected_scope(locale, detail.window),
            ),
            key_value_text(locale, MessageKey::FieldRunCount, &detail.runs.to_string()),
            key_value_text(
                locale,
                MessageKey::FieldSamples,
                &terminal_denominator(locale, detail.sample_count)
            ),
            key_value_text(
                locale,
                MessageKey::FieldFailures,
                &detail.failed_runs.to_string()
            ),
            key_value_text(
                locale,
                MessageKey::FieldFailureRate,
                &failure_rate_with_sample(locale, detail.failure_rate_percent, detail.sample_count)
            ),
            key_value_text(
                locale,
                MessageKey::FieldHealth,
                health_name(
                    locale,
                    presentation_health(detail.coverage, detail.failed_runs, detail.sample_count),
                ),
            ),
            key_value_text(
                locale,
                MessageKey::FieldHealthExplanation,
                risk_reason(
                    locale,
                    detail.failed_runs,
                    detail.sample_count,
                    detail.coverage,
                ),
            ),
            key_value_text(
                locale,
                MessageKey::FieldRisk,
                &risk_score(locale, detail.risk.score),
            ),
            key_value_text(
                locale,
                MessageKey::FieldReason,
                risk_reason(
                    locale,
                    detail.failed_runs,
                    detail.sample_count,
                    detail.coverage,
                ),
            ),
        )
    } else {
        runtime_handler_health(locale, app, event, handler)
    };
    let observation = app.matched_reliability_detail().map_or_else(
        || t(locale, MessageKey::StateReliabilityUnavailable).to_owned(),
        |detail| {
            let mut facts = vec![key_value_text(
                locale,
                MessageKey::FieldCurrentRevision,
                &short_revision(&detail.revision),
            )];
            if let Some(history) = app.matched_runtime_catalog_history() {
                facts.extend([
                    key_value_text(
                        locale,
                        MessageKey::FieldFirstSeen,
                        &format_human_time(
                            locale,
                            history.first_seen_unix_ms,
                            presentation_now(app),
                        ),
                    ),
                    key_value_text(
                        locale,
                        MessageKey::FieldLastSeen,
                        &format_human_time(
                            locale,
                            history.last_seen_unix_ms,
                            presentation_now(app),
                        ),
                    ),
                    key_value_text(
                        locale,
                        MessageKey::FieldLatestEvidence,
                        &format_human_time(
                            locale,
                            history.latest_evidence_unix_ms,
                            presentation_now(app),
                        ),
                    ),
                    key_value_text(
                        locale,
                        MessageKey::FieldRevisionCount,
                        &history.revision_count.to_string(),
                    ),
                    key_value_text(
                        locale,
                        MessageKey::FieldObservationStatus,
                        catalog_observation_status(locale, history.historical_status),
                    ),
                ]);
            } else {
                facts.push(key_value_text(
                    locale,
                    MessageKey::FieldObservationStatus,
                    t(locale, MessageKey::StateReliabilityUnavailable),
                ));
            }
            facts.join("\n")
        },
    );
    let advanced = app.matched_reliability_detail().map_or_else(
        || t(locale, MessageKey::StateReliabilityUnavailable).to_owned(),
        |detail| {
            let recent = if detail.recent_failures.is_empty() {
                t(locale, MessageKey::StateNoRecentFailures).to_owned()
            } else {
                detail
                    .recent_failures
                    .iter()
                    .map(|failure| {
                        format!(
                            "{} · {} · {}",
                            format_human_time(
                                locale,
                                failure.occurred_at_unix_ms,
                                presentation_now(app),
                            ),
                            terminal_status_name(locale, failure.status),
                            failure.bounded_fingerprint.as_deref().unwrap_or_default(),
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            let trends = detail
                .trends
                .iter()
                .map(|trend| trend_detail(locale, trend))
                .collect::<Vec<_>>()
                .join("\n");
            let fingerprints = if detail.failure_fingerprints.is_empty() {
                t(locale, MessageKey::StateNoRecentFailures).to_owned()
            } else {
                detail
                    .failure_fingerprints
                    .iter()
                    .map(|cluster| {
                        format!(
                            "{}: {} · {} {} · {} {}",
                            fingerprint_name(locale, cluster.kind),
                            cluster.occurrences,
                            t(locale, MessageKey::FieldFirstSeen),
                            format_human_time(
                                locale,
                                cluster.first_occurred_at_unix_ms,
                                presentation_now(app),
                            ),
                            t(locale, MessageKey::FieldLatestEvidence),
                            format_human_time(
                                locale,
                                cluster.latest_occurred_at_unix_ms,
                                presentation_now(app),
                            ),
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            format!(
                "{}\n{}\n\n{}\n{}\n\n{}\n{}\n\n{}\n{}\n\n{}\n{}: {}\n{}: {}",
                t(locale, MessageKey::SectionRecentFailures),
                recent,
                t(locale, MessageKey::SectionTrends),
                trends,
                t(locale, MessageKey::SectionRevisionComparison),
                revision_detail(locale, &detail.revision_comparison),
                t(locale, MessageKey::SectionFailureFingerprints),
                fingerprints,
                t(locale, MessageKey::SectionTechnicalMetadata),
                t(locale, MessageKey::FieldFullRevision),
                detail.revision,
                t(locale, MessageKey::FieldInternalIdentity),
                detail.internal_ref.handler_key,
            )
        },
    );
    let body = format!(
        "{}\n{}\n\n{}\n{}\n\n{}\n{}\n\n{}\n{}",
        t(locale, MessageKey::SectionRuntimeConfiguration),
        configuration.join("\n"),
        t(locale, MessageKey::SectionReliabilitySummary),
        reliability,
        t(locale, MessageKey::SectionObservationHistory),
        observation,
        t(locale, MessageKey::SectionIntelligence),
        advanced,
    );
    let body = prepend_runtime_resource_notices(body, app, locale);
    frame.render_widget(
        Paragraph::new(body)
            .style(theme.typography_style(TypographyRole::Value))
            .block(themed_block(t(locale, MessageKey::ViewHookDetail), theme))
            .wrap(Wrap { trim: true })
            .scroll((app.detail_scroll_lines(), 0)),
        area,
    );
}

fn runtime_event_name(locale: ResolvedLocale, event: &RuntimeEventPresentation) -> String {
    event
        .canonical_event
        .map(|event| event_name(locale, event).to_owned())
        .unwrap_or_else(|| event.runtime_event_name.clone())
}

fn runtime_handler_label(
    locale: ResolvedLocale,
    index: usize,
    handler: &RuntimeHandlerPresentation,
) -> String {
    match &handler.handler_kind {
        RuntimeHandlerKind::Command { command } if !command.is_empty() => {
            truncate_to_width(command, 42)
        }
        RuntimeHandlerKind::McpTool { tool, .. } if !tool.is_empty() => tool.clone(),
        _ => format!("{} {}", t(locale, MessageKey::IdentityHook), index + 1),
    }
}

fn runtime_handler_kind(locale: ResolvedLocale, kind: &RuntimeHandlerKind) -> String {
    match kind {
        RuntimeHandlerKind::Command { .. } => t(locale, MessageKey::FieldCommand).to_owned(),
        RuntimeHandlerKind::McpTool { .. } => t(locale, MessageKey::FieldMcpTool).to_owned(),
        RuntimeHandlerKind::Prompt => t(locale, MessageKey::FieldPrompt).to_owned(),
        RuntimeHandlerKind::Agent => t(locale, MessageKey::FieldAgent).to_owned(),
        RuntimeHandlerKind::Unknown { label } => label.clone(),
    }
}

fn runtime_handler_mode(locale: ResolvedLocale, mode: Option<RuntimeHandlerMode>) -> &'static str {
    match mode {
        Some(RuntimeHandlerMode::Sync) => t(locale, MessageKey::ValueSync),
        Some(RuntimeHandlerMode::Async) => t(locale, MessageKey::ValueAsync),
        None => t(locale, MessageKey::StatusUnavailable),
    }
}

fn runtime_trust(locale: ResolvedLocale, trust: RuntimeTrust) -> &'static str {
    match trust {
        RuntimeTrust::Managed => t(locale, MessageKey::FieldManaged),
        RuntimeTrust::Trusted => t(locale, MessageKey::ValueTrusted),
        RuntimeTrust::Untrusted => t(locale, MessageKey::ValueUntrusted),
        RuntimeTrust::Modified => t(locale, MessageKey::ValueModified),
        RuntimeTrust::Unknown => t(locale, MessageKey::StatusUnavailable),
    }
}

fn runtime_handler_health(
    locale: ResolvedLocale,
    app: &App,
    event: &RuntimeEventPresentation,
    handler: &RuntimeHandlerPresentation,
) -> String {
    if let Some(row) = app.runtime_handler_reliability_row(event, handler) {
        return health_name(
            locale,
            presentation_health(row.coverage, row.failed_runs, row.sample_count),
        )
        .to_owned();
    }
    app.runtime_event_reliability(event)
        .into_iter()
        .find(|(catalog_id, _)| catalog_id == &handler.runtime_catalog_id)
        .map(|(_, join)| runtime_join_health(locale, join))
        .unwrap_or_else(|| t(locale, MessageKey::StateReliabilityUnavailable).to_owned())
}

fn runtime_event_health(
    locale: ResolvedLocale,
    app: &App,
    event: &RuntimeEventPresentation,
) -> String {
    let joins = app.runtime_event_reliability(event);
    if joins.is_empty() {
        return t(locale, MessageKey::StateReliabilityUnavailable).to_owned();
    }
    if joins
        .iter()
        .any(|(_, join)| matches!(join, ReliabilityJoinState::Unavailable))
    {
        return t(locale, MessageKey::StateReliabilityUnavailable).to_owned();
    }
    if joins
        .iter()
        .any(|(_, join)| matches!(join, ReliabilityJoinState::Unsupported))
    {
        return coverage_summary(locale, crate::domain::EvidenceCoverage::NotAdmitted);
    }
    if joins
        .iter()
        .any(|(_, join)| matches!(join, ReliabilityJoinState::Ambiguous))
    {
        return t(locale, MessageKey::StateJoinAmbiguous).to_owned();
    }
    if joins
        .iter()
        .any(|(_, join)| matches!(join, ReliabilityJoinState::NoHistory))
    {
        return t(locale, MessageKey::StateNotObserved).to_owned();
    }
    let matched_health = event
        .handlers
        .iter()
        .filter_map(|handler| app.runtime_handler_reliability_row(event, handler))
        .map(|row| presentation_health(row.coverage, row.failed_runs, row.sample_count))
        .fold(None, |worst, health| {
            Some(conservative_health(worst, health))
        });
    if let Some(health) = matched_health {
        return health_name(locale, health).to_owned();
    }
    t(locale, MessageKey::StateReliabilityUnavailable).to_owned()
}

fn conservative_health(current: Option<Health>, next: Health) -> Health {
    let rank = |health| match health {
        Health::Healthy => 0,
        Health::CoverageLimited => 1,
        Health::NoTerminalSamples => 2,
        Health::Degraded => 3,
    };
    current
        .filter(|health| rank(*health) >= rank(next))
        .unwrap_or(next)
}

fn runtime_resource_notices(app: &App, locale: ResolvedLocale) -> Vec<String> {
    let mut notices = Vec::new();
    if app.runtime_catalog_error() {
        notices.push(t(locale, MessageKey::StateRuntimeCatalogStale).to_owned());
    } else if app.runtime_catalog_loading() {
        notices.push(t(locale, MessageKey::StateRuntimeCatalogLoading).to_owned());
    }
    match app.view_state() {
        ResourceState::Error { .. } => {
            notices.push(t(locale, MessageKey::StateRefreshFailed).to_owned())
        }
        ResourceState::Loading { .. } if app.view_model().is_none() => {
            notices.push(t(locale, MessageKey::StateLoading).to_owned());
        }
        ResourceState::Empty => {
            notices.push(t(locale, MessageKey::StateReliabilityUnavailable).to_owned())
        }
        ResourceState::Loading { .. } | ResourceState::Ready(_) => {}
    }
    notices
}

fn prepend_runtime_resource_notices(body: String, app: &App, locale: ResolvedLocale) -> String {
    let notices = runtime_resource_notices(app, locale);
    if notices.is_empty() {
        body
    } else {
        format!("{}\n\n{body}", notices.join(" · "))
    }
}

fn visible_list_window(selected: Option<usize>, total: usize, capacity: usize) -> (usize, usize) {
    if total == 0 {
        return (0, 0);
    }
    let capacity = capacity.max(1).min(total);
    let selected = selected.unwrap_or(0).min(total - 1);
    let start = selected
        .saturating_sub(capacity / 2)
        .min(total.saturating_sub(capacity));
    (start, start + capacity)
}

fn runtime_join_health(locale: ResolvedLocale, join: ReliabilityJoinState) -> String {
    match join {
        ReliabilityJoinState::Matched { .. } => {
            t(locale, MessageKey::StateObservedInSelectedPeriod).to_owned()
        }
        ReliabilityJoinState::NoHistory => t(locale, MessageKey::StateNotObserved).to_owned(),
        ReliabilityJoinState::Ambiguous => t(locale, MessageKey::StateJoinAmbiguous).to_owned(),
        ReliabilityJoinState::Unsupported => {
            coverage_summary(locale, crate::domain::EvidenceCoverage::NotAdmitted)
        }
        ReliabilityJoinState::Unavailable => {
            t(locale, MessageKey::StateReliabilityUnavailable).to_owned()
        }
    }
}

struct HookRowsContext<'a> {
    selected: Option<&'a super::view_model::HandlerRef>,
    content_focused: bool,
    locale: ResolvedLocale,
    theme: Theme,
    title: &'a str,
    compact_scroll_lines: u16,
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
    if area.width < 228 {
        let rows = visible_rows(
            rows,
            selected_index,
            if area.width < 54 {
                1
            } else {
                area.height.saturating_sub(2) as usize / 6
            },
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
                    "{marker} {}\n  {} · {} · {} · {}\n  {}: {}\n  {}: {}\n  {}: {}",
                    truncate_to_width(&identity, area.width.saturating_sub(6) as usize),
                    event_name(context.locale, row.event),
                    runtime_name(context.locale, row.internal_ref.runtime),
                    failure_rate_with_sample(
                        context.locale,
                        row.failure_rate_percent,
                        row.sample_count,
                    ),
                    trend_summary(context.locale, row),
                    t(context.locale, MessageKey::FieldRisk),
                    compact_risk(context.locale, row),
                    t(context.locale, MessageKey::FieldCoverage),
                    coverage_summary(context.locale, row.coverage),
                    t(context.locale, MessageKey::FieldReason),
                    risk_reason(
                        context.locale,
                        row.failed_runs,
                        row.sample_count,
                        row.coverage,
                    ),
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        frame.render_widget(
            Paragraph::new(content)
                .style(context.theme.typography_style(TypographyRole::Value))
                .block(themed_block(context.title, context.theme))
                .wrap(Wrap { trim: true })
                .scroll((context.compact_scroll_lines, 0)),
            area,
        );
        return;
    }
    let rows = visible_rows(
        rows,
        selected_index,
        area.height.saturating_sub(3) as usize / 2,
    );
    let header = Row::new([
        t(context.locale, MessageKey::ColumnName),
        t(context.locale, MessageKey::ColumnEvent),
        t(context.locale, MessageKey::ColumnRuntime),
        t(context.locale, MessageKey::FieldCoverage),
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
            Cell::from(coverage_summary(context.locale, row.coverage)),
            Cell::from(failure_rate_with_sample(
                context.locale,
                row.failure_rate_percent,
                row.sample_count,
            )),
            Cell::from(trend_summary(context.locale, row)),
            Cell::from(format!(
                "{}\n{}",
                compact_risk(context.locale, row),
                risk_reason(
                    context.locale,
                    row.failed_runs,
                    row.sample_count,
                    row.coverage,
                )
            )),
        ])
        .height(2)
        .style(style)
    });
    let table = Table::new(
        rows,
        [
            Constraint::Min(16),
            Constraint::Length(14),
            Constraint::Length(10),
            Constraint::Length(78),
            Constraint::Length(18),
            Constraint::Length(18),
            Constraint::Length(56),
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
        .constraints([
            Constraint::Length(if app.alias_editing() { 24 } else { 21 }),
            Constraint::Min(9),
        ])
        .split(area);
    let identity = display_identity(
        locale,
        &detail.display_identity,
        detail.display_disambiguator,
    );
    let mut facts = vec![
        Line::from(Span::styled(
            period_selector_for_window(locale, detail.window, app.view_state().is_loading()),
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
            &coverage_summary(locale, detail.coverage),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldRevision,
            &short_revision(&detail.revision),
            theme,
        ),
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
        label_value_line(
            t(locale, MessageKey::FieldMetricScope),
            &selected_scope(locale, detail.window),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldSamples,
            &terminal_denominator(locale, detail.sample_count),
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
            &risk_score(locale, detail.risk.score),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldReason,
            risk_reason(
                locale,
                detail.failed_runs,
                detail.sample_count,
                detail.coverage,
            ),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldHealth,
            health_name(
                locale,
                presentation_health(detail.coverage, detail.failed_runs, detail.sample_count),
            ),
            theme,
        ),
    ];
    if let Some(alias) = app.alias_draft() {
        let marker = if app.alias_text_editing() { ">" } else { " " };
        facts.push(key_value_line(
            locale,
            MessageKey::SectionAlias,
            &format!("{marker} {alias}"),
            theme,
        ));
        facts.push(Line::from(Span::styled(
            alias_save_message(locale, app.alias_save_state()),
            theme.typography_style(TypographyRole::Metadata),
        )));
    }
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
                    format_human_time(locale, failure.occurred_at_unix_ms, presentation_now(app)),
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
    let catalog_history = app.selected_catalog_history().map_or_else(
        || {
            format!(
                "{}: {}",
                t(locale, MessageKey::FieldDataFreshness),
                t(locale, MessageKey::StateLoading)
            )
        },
        |history| {
            format!(
                "{}: {} · {}: {} · {}: {}\n{}: {} · {}: {} · {}: {}",
                t(locale, MessageKey::FieldFirstSeen),
                format_human_time(locale, history.first_seen_unix_ms, presentation_now(app)),
                t(locale, MessageKey::FieldLastSeen),
                format_human_time(locale, history.last_seen_unix_ms, presentation_now(app)),
                t(locale, MessageKey::FieldLatestEvidence),
                format_human_time(
                    locale,
                    history.latest_evidence_unix_ms,
                    presentation_now(app)
                ),
                t(locale, MessageKey::FieldRevisionCount),
                history.revision_count,
                t(locale, MessageKey::FieldObservationStatus),
                catalog_observation_status(locale, history.historical_status),
                t(locale, MessageKey::FieldDataFreshness),
                format_human_time(
                    locale,
                    history.latest_evidence_unix_ms,
                    presentation_now(app)
                ),
            )
        },
    );
    let fingerprints = if detail.failure_fingerprints.is_empty() {
        t(locale, MessageKey::StateNoRecentFailures).to_owned()
    } else {
        detail
            .failure_fingerprints
            .iter()
            .map(|cluster| {
                format!(
                    "{}: {} · {} {} · {} {}",
                    fingerprint_name(locale, cluster.kind),
                    cluster.occurrences,
                    t(locale, MessageKey::FieldFirstSeen),
                    format_human_time(
                        locale,
                        cluster.first_occurred_at_unix_ms,
                        presentation_now(app),
                    ),
                    t(locale, MessageKey::FieldLatestEvidence),
                    format_human_time(
                        locale,
                        cluster.latest_occurred_at_unix_ms,
                        presentation_now(app),
                    ),
                )
            })
            .collect::<Vec<_>>()
            .join(" · ")
    };
    let technical_metadata = format!(
        "{}\n{}: {}\n{}: {}",
        t(locale, MessageKey::SectionTechnicalMetadata),
        t(locale, MessageKey::FieldInternalIdentity),
        detail.internal_ref.handler_key,
        t(locale, MessageKey::FieldFullRevision),
        detail.revision,
    );
    let body = format!(
        "{catalog_history}\n\n{}: {} · {}: {} · {}: {}\n{}: {} · {}: {} · {}: {}\n\n{}\n{}\n\n{}\n{}\n\n{}\n{}\n\n{}\n{}\n\n{technical_metadata}",
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
    if area.height < 18 || area.width < 72 {
        let compact = format!(
            "{identity}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{body}",
            key_value_text(
                locale,
                MessageKey::FieldRuntime,
                runtime_name(locale, detail.internal_ref.runtime),
            ),
            key_value_text(
                locale,
                MessageKey::FieldEvent,
                event_name(locale, detail.event),
            ),
            key_value_text(
                locale,
                MessageKey::FieldCoverage,
                &coverage_summary(locale, detail.coverage),
            ),
            label_value_text(
                t(locale, MessageKey::FieldMetricScope),
                &selected_scope(locale, detail.window),
            ),
            key_value_text(
                locale,
                MessageKey::FieldSamples,
                &terminal_denominator(locale, detail.sample_count),
            ),
            key_value_text(
                locale,
                MessageKey::FieldFailureRate,
                &failure_rate_with_sample(locale, detail.failure_rate_percent, detail.sample_count),
            ),
            key_value_text(
                locale,
                MessageKey::FieldRisk,
                &risk_score(locale, detail.risk.score),
            ),
            key_value_text(
                locale,
                MessageKey::FieldReason,
                risk_reason(
                    locale,
                    detail.failed_runs,
                    detail.sample_count,
                    detail.coverage,
                ),
            ),
            key_value_text(
                locale,
                MessageKey::FieldHealth,
                health_name(
                    locale,
                    presentation_health(detail.coverage, detail.failed_runs, detail.sample_count),
                ),
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

fn render_failure_clusters(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    locale: ResolvedLocale,
    theme: Theme,
) {
    let clusters = app.failure_clusters();
    let window = accepted_window(app);
    if clusters.is_empty() {
        let content = format!(
            "{}\n{}: {}\n\n{}",
            period_selector_for_window(locale, window, app.view_state().is_loading()),
            t(locale, MessageKey::FieldMetricScope),
            selected_scope(locale, window),
            t(locale, MessageKey::StateNoRecentFailures),
        );
        frame.render_widget(
            Paragraph::new(content)
                .style(theme.typography_style(TypographyRole::Value))
                .block(themed_block(
                    t(locale, MessageKey::ViewFailureClusters),
                    theme,
                ))
                .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(if area.width < 72 { 6 } else { 4 }),
            Constraint::Min(5),
        ])
        .split(area);
    let heading = format!(
        "{}\n{}: {}",
        period_selector_for_window(locale, window, app.view_state().is_loading()),
        t(locale, MessageKey::FieldMetricScope),
        selected_scope(locale, window),
    );
    frame.render_widget(
        Paragraph::new(heading)
            .style(theme.typography_style(TypographyRole::Metadata))
            .block(themed_block(
                t(locale, MessageKey::ViewFailureClusters),
                theme,
            ))
            .wrap(Wrap { trim: true }),
        sections[0],
    );
    let list_area = sections[1];
    let selected_index = app.selected_failure_cluster().map_or(0, |selected| {
        clusters
            .iter()
            .position(|cluster| cluster.reference == selected)
            .unwrap_or(0)
    });
    if list_area.width < 58 {
        let rows = visible_rows(
            clusters,
            selected_index,
            list_area.height.saturating_sub(2) as usize / 3,
        );
        let content = rows
            .iter()
            .map(|cluster| {
                let marker = if app.selected_failure_cluster() == Some(cluster.reference) {
                    ">"
                } else {
                    " "
                };
                format!(
                    "{marker} {}\n  {}: {} · {}: {}\n  {}: {}",
                    fingerprint_name(locale, cluster.reference.kind),
                    t(locale, MessageKey::FieldOccurrences),
                    cluster.occurrences,
                    t(locale, MessageKey::FieldAffectedHooks),
                    cluster.affected_hooks.len(),
                    t(locale, MessageKey::FieldLatestEvidence),
                    format_human_time(
                        locale,
                        cluster.latest_occurred_at_unix_ms,
                        presentation_now(app),
                    ),
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        frame.render_widget(
            Paragraph::new(content)
                .style(theme.typography_style(TypographyRole::Value))
                .block(themed_block(
                    t(locale, MessageKey::SectionFailureFingerprints),
                    theme,
                ))
                .wrap(Wrap { trim: true }),
            list_area,
        );
        return;
    }
    let rows = visible_rows(
        clusters,
        selected_index,
        list_area.height.saturating_sub(3) as usize,
    );
    let header = Row::new([
        t(locale, MessageKey::SectionFailureFingerprints),
        t(locale, MessageKey::FieldOccurrences),
        t(locale, MessageKey::FieldAffectedHooks),
        t(locale, MessageKey::FieldFirstSeen),
        t(locale, MessageKey::FieldLatestEvidence),
    ])
    .style(theme.typography_style(TypographyRole::SectionTitle));
    let rows = rows.iter().map(|cluster| {
        let style = if app.selected_failure_cluster() == Some(cluster.reference)
            && app.local_list_active()
        {
            theme.color_style(ColorRole::Selected)
        } else {
            theme.typography_style(TypographyRole::Value)
        };
        Row::new(vec![
            Cell::from(fingerprint_name(locale, cluster.reference.kind)),
            Cell::from(cluster.occurrences.to_string()),
            Cell::from(cluster.affected_hooks.len().to_string()),
            Cell::from(format_human_time(
                locale,
                cluster.first_occurred_at_unix_ms,
                presentation_now(app),
            )),
            Cell::from(format_human_time(
                locale,
                cluster.latest_occurred_at_unix_ms,
                presentation_now(app),
            )),
        ])
        .style(style)
    });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Min(18),
                Constraint::Length(14),
                Constraint::Length(16),
                Constraint::Length(18),
                Constraint::Length(18),
            ],
        )
        .header(header)
        .block(themed_block(
            t(locale, MessageKey::SectionFailureFingerprints),
            theme,
        )),
        list_area,
    );
}

fn render_failure_cluster_detail(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    locale: ResolvedLocale,
    theme: Theme,
) {
    let cluster = app.selected_failure_cluster().and_then(|reference| {
        app.view_model()
            .and_then(|view| view.failure_cluster(reference))
    });
    let Some(cluster) = cluster else {
        render_state_panel(
            frame,
            area,
            t(locale, MessageKey::ViewFailureClusterDetail),
            t(locale, MessageKey::StateEmpty),
            ColorRole::Warning,
            theme,
        );
        return;
    };
    let affected = cluster
        .affected_hooks
        .iter()
        .map(|hook| {
            format!(
                "{} · {}",
                display_identity(locale, &hook.display_identity, hook.display_disambiguator),
                event_name(locale, hook.event),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut facts = vec![
        Line::from(Span::styled(
            period_selector_for_window(locale, cluster.window, app.view_state().is_loading()),
            theme.typography_style(TypographyRole::Metadata),
        )),
        label_value_line(
            t(locale, MessageKey::FieldMetricScope),
            &selected_scope(locale, cluster.window),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::SectionFailureFingerprints,
            fingerprint_name(locale, cluster.reference.kind),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldOccurrences,
            &cluster.occurrences.to_string(),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldWindow,
            window_name(locale, cluster.window),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldCoverage,
            &coverage_summary(locale, cluster.coverage),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldFirstSeen,
            &format_human_time(
                locale,
                cluster.first_occurred_at_unix_ms,
                presentation_now(app),
            ),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldLatestEvidence,
            &format_human_time(
                locale,
                cluster.latest_occurred_at_unix_ms,
                presentation_now(app),
            ),
            theme,
        ),
        key_value_line(
            locale,
            MessageKey::FieldDataFreshness,
            &format_human_time(
                locale,
                cluster.latest_occurred_at_unix_ms,
                presentation_now(app),
            ),
            theme,
        ),
    ];
    if area.height < 24 || area.width < 88 {
        facts.push(Line::from(""));
        facts.push(Line::from(t(locale, MessageKey::FieldAffectedHooks)));
        facts.extend(affected.lines().map(|line| Line::from(line.to_owned())));
        frame.render_widget(
            Paragraph::new(facts)
                .style(theme.typography_style(TypographyRole::Value))
                .block(themed_block(
                    t(locale, MessageKey::ViewFailureClusterDetail),
                    theme,
                ))
                .wrap(Wrap { trim: true })
                .scroll((app.detail_scroll_lines(), 0)),
            area,
        );
        return;
    }
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(14), Constraint::Min(6)])
        .split(area);
    frame.render_widget(
        Paragraph::new(facts)
            .block(themed_block(
                t(locale, MessageKey::ViewFailureClusterDetail),
                theme,
            ))
            .wrap(Wrap { trim: true }),
        sections[0],
    );
    frame.render_widget(
        Paragraph::new(affected)
            .style(theme.typography_style(TypographyRole::Value))
            .block(themed_block(
                t(locale, MessageKey::FieldAffectedHooks),
                theme,
            ))
            .wrap(Wrap { trim: true })
            .scroll((app.detail_scroll_lines(), 0)),
        sections[1],
    );
}

fn catalog_observation_status(
    locale: ResolvedLocale,
    status: crate::workbench::HistoricalStatus,
) -> &'static str {
    match status {
        crate::workbench::HistoricalStatus::ObservedInSelectedPeriod => {
            t(locale, MessageKey::StateObservedInSelectedPeriod)
        }
        crate::workbench::HistoricalStatus::HistoricalOutsideSelectedPeriod => {
            t(locale, MessageKey::StateHistoricalOnly)
        }
    }
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
    risk_score(locale, row.risk.score)
}

fn risk_score(locale: ResolvedLocale, score: u8) -> String {
    format!("{} ({score}/100)", risk_category(locale, score))
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
        trend_scope(locale, trend.window),
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
        "{}: {} · {}\n  {}: {}",
        t(locale, MessageKey::FieldCurrentRevision),
        short_revision(&comparison.current.revision),
        failure_rate_with_sample(
            locale,
            comparison.current.failure_rate_percent,
            comparison.current.failure_sample_count,
        ),
        t(locale, MessageKey::FieldMetricScope),
        current_revision_scope(locale),
    );
    let previous = comparison.previous.as_ref().map_or_else(
        || {
            format!(
                "{}: {}\n  {}: {}",
                t(locale, MessageKey::FieldPreviousRevision),
                intelligence_availability_name(locale, comparison.availability),
                t(locale, MessageKey::FieldMetricScope),
                previous_revision_scope(locale),
            )
        },
        |previous| {
            format!(
                "{}: {} · {}\n  {}: {}",
                t(locale, MessageKey::FieldPreviousRevision),
                short_revision(&previous.revision),
                failure_rate_with_sample(
                    locale,
                    previous.failure_rate_percent,
                    previous.failure_sample_count,
                ),
                t(locale, MessageKey::FieldMetricScope),
                previous_revision_scope(locale),
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

fn label_value_line(label: &str, value: &str, theme: Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label}: "),
            theme.typography_style(TypographyRole::FieldLabel),
        ),
        Span::styled(
            value.to_owned(),
            theme.typography_style(TypographyRole::Value),
        ),
    ])
}

fn label_value_text(label: &str, value: &str) -> String {
    format!("{label}: {value}")
}

fn presentation_now(app: &App) -> i64 {
    app.view_model()
        .map(|view| view.overview.generated_at_unix_ms)
        .unwrap_or(0)
}

fn changes_now(app: &App) -> i64 {
    app.changes()
        .map(|changes| changes.generated_at_unix_ms)
        .unwrap_or_else(|| presentation_now(app))
}

fn selected_scope(locale: ResolvedLocale, window: TimeWindow) -> String {
    if window == TimeWindow::All {
        return t(locale, MessageKey::ScopeAllObservedAllRevisions).to_owned();
    }
    t(locale, MessageKey::ScopeSelectedAllRevisions)
        .replace("{period}", window_name(locale, window))
}

fn trend_scope(locale: ResolvedLocale, window: TimeWindow) -> String {
    if window == TimeWindow::All {
        return t(locale, MessageKey::ScopeAllObservedAllRevisions).to_owned();
    }
    t(locale, MessageKey::ScopePeriodAllRevisions).replace("{period}", window_name(locale, window))
}

fn current_revision_scope(locale: ResolvedLocale) -> &'static str {
    t(locale, MessageKey::ScopeAllObservedCurrentRevision)
}

fn previous_revision_scope(locale: ResolvedLocale) -> &'static str {
    t(locale, MessageKey::ScopeAllObservedPreviousRevision)
}

fn revision_timeline_scope(locale: ResolvedLocale) -> &'static str {
    t(locale, MessageKey::ScopeAllObservedRevisionTimeline)
}

fn terminal_denominator(locale: ResolvedLocale, samples: u64) -> String {
    t(locale, MessageKey::ScopeTerminalSamples).replace("{samples}", &samples.to_string())
}

fn coverage_summary(locale: ResolvedLocale, coverage: crate::domain::EvidenceCoverage) -> String {
    format!(
        "{} — {}",
        coverage_name(locale, coverage),
        coverage_explanation(locale, coverage)
    )
}

fn coverage_explanation(
    locale: ResolvedLocale,
    coverage: crate::domain::EvidenceCoverage,
) -> &'static str {
    use crate::domain::EvidenceCoverage;
    let key = match coverage {
        EvidenceCoverage::Complete => MessageKey::CoverageExplanationComplete,
        EvidenceCoverage::Partial => MessageKey::CoverageExplanationPartial,
        EvidenceCoverage::SyncOnly => MessageKey::CoverageExplanationSyncOnly,
        EvidenceCoverage::BestEffort => MessageKey::CoverageExplanationBestEffort,
        EvidenceCoverage::Unknown => MessageKey::CoverageExplanationUnknown,
        EvidenceCoverage::NotAdmitted => MessageKey::CoverageExplanationNotAdmitted,
        EvidenceCoverage::SyntheticFixture => MessageKey::CoverageExplanationSyntheticFixture,
    };
    t(locale, key)
}

fn presentation_health(
    coverage: crate::domain::EvidenceCoverage,
    failed_runs: u64,
    samples: u64,
) -> Health {
    if failed_runs > 0 {
        Health::Degraded
    } else if samples == 0 {
        Health::NoTerminalSamples
    } else if coverage == crate::domain::EvidenceCoverage::Complete {
        Health::Healthy
    } else {
        Health::CoverageLimited
    }
}

fn risk_category(locale: ResolvedLocale, score: u8) -> &'static str {
    let key = match score {
        0..=24 => MessageKey::RiskLow,
        25..=49 => MessageKey::RiskGuarded,
        50..=74 => MessageKey::RiskElevated,
        _ => MessageKey::RiskHigh,
    };
    t(locale, key)
}

fn risk_reason(
    locale: ResolvedLocale,
    failed_runs: u64,
    terminal_samples: u64,
    coverage: crate::domain::EvidenceCoverage,
) -> &'static str {
    let key = if failed_runs > 0 {
        MessageKey::RiskReasonFailures
    } else if terminal_samples == 0 {
        MessageKey::RiskReasonNoTerminalSamples
    } else if coverage == crate::domain::EvidenceCoverage::Complete {
        MessageKey::RiskReasonComplete
    } else {
        MessageKey::RiskReasonIncomplete
    };
    t(locale, key)
}

fn short_revision(revision: &str) -> String {
    const PRIMARY_REVISION_CHARS: usize = 12;
    let mut chars = revision.chars();
    let visible = chars
        .by_ref()
        .take(PRIMARY_REVISION_CHARS)
        .collect::<String>();
    if chars.next().is_some() {
        format!("{visible}…")
    } else {
        visible
    }
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
    use serde_json::json;

    fn control_center_catalog() -> crate::runtime_presentation::RuntimePresentationSnapshot {
        crate::runtime_presentation::RuntimePresentationSnapshot::from_codex_hooks_list(
            &json!({"result":{"data":[{
                "cwd":"C:/synthetic/workspace",
                "warnings":["synthetic catalog warning"],
                "errors":["synthetic catalog error"],
                "hooks":[
                    {"key":"fixture:0:0","eventName":"PreToolUse","handlerType":"command","command":"synthetic command --very-long-safe-argument=1234567890 --another-safe-argument=abcdefghijklmnopqrstuvwxyz","matcher":"^SyntheticToolWithAnIntentionallyLongSafeName$","source":"C:/synthetic/very/long/source/hooks.json","sourcePath":"C:/synthetic/very/long/source/hooks.json","enabled":true,"isManaged":false,"trustStatus":"trusted","async":false,"timeoutSec":9,"additionalContextLimit":64},
                    {"key":"fixture:0:1","eventName":"PostToolUse","handlerType":"mcp_tool","mcpServer":"synthetic-server","mcpTool":"synthetic-tool","source":"project","enabled":false,"isManaged":false,"trustStatus":"untrusted"},
                    {"key":"fixture:0:2","eventName":"UserPromptSubmit","handlerType":"prompt","source":"user","enabled":true,"isManaged":false,"trustStatus":"modified"},
                    {"key":"fixture:0:3","eventName":"SubagentStart","handlerType":"agent","source":"managed","enabled":true,"isManaged":true,"trustStatus":"trusted"},
                    {"key":"fixture:0:4","eventName":"Interrupt","handlerType":"command","command":"synthetic interrupt","enabled":true,"isManaged":false,"trustStatus":"trusted"},
                    {"key":"fixture:0:5","eventName":"FutureRuntimeEvent","handlerType":"future_handler","enabled":true,"isManaged":false,"trustStatus":"trusted"}
                ]
            }]}}),
            1_000,
        )
        .unwrap()
    }

    fn control_center_app() -> App {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.apply_runtime_catalog(control_center_catalog());
        app.handle(super::super::keymap::Command::Down);
        app.handle(super::super::keymap::Command::Enter);
        app
    }

    fn runtime_health_app(coverage: EvidenceCoverage, status: TerminalStatus) -> App {
        let catalog = control_center_catalog();
        let event = catalog
            .events
            .iter()
            .find(|event| event.runtime_event_name == "PreToolUse")
            .unwrap();
        let handler_key = event.handlers[0].reliability_handler_key.clone().unwrap();
        let value = HookInvocation {
            source_key: "runtime-health".into(),
            source_record_id: "runtime-health-0".into(),
            runtime: Runtime::Codex,
            evidence_kind: EvidenceKind::SyntheticFixture,
            evidence_generation: crate::domain::EvidenceGeneration::SyntheticFixture,
            coverage,
            handler: HandlerIdentity {
                key: handler_key,
                revision: "runtime-health-r1".into(),
                label: "Runtime health fixture".into(),
                source_kind: "fixture".into(),
                event: HookEvent::PreToolUse,
                matcher_identity: "fixture".into(),
                structural_identity: "fixture".into(),
                execution_mode: ExecutionMode::Sync,
            },
            occurred_at_unix_ms: 999,
            terminal_status: status,
            duration_ms: None,
            error_fingerprint: None,
        };
        let mut report =
            instrumented_report(std::slice::from_ref(&value), 1_000, TimeWindow::All, 0, 0);
        report.qualification.coverage = coverage;
        let mut app = App::from_snapshot(super::super::app::RefreshSnapshot::from_report(report));
        assert_eq!(
            app.view_model().unwrap().hooks.rows[0]
                .internal_ref
                .handler_key,
            event.handlers[0]
                .reliability_handler_key
                .as_deref()
                .unwrap()
        );
        app.apply_runtime_catalog(catalog);
        app.apply_changes(super::super::app::ChangesSnapshot::from_values(
            vec![value],
            1_000,
            TimeWindow::All,
            coverage,
        ));
        app.handle(super::super::keymap::Command::Down);
        app.handle(super::super::keymap::Command::Enter);
        for _ in 0..6 {
            app.handle(super::super::keymap::Command::Down);
        }
        assert_eq!(
            app.selected_runtime_event()
                .map(|event| event.runtime_event_name.as_str()),
            Some("PreToolUse")
        );
        app
    }

    fn many_events_catalog() -> crate::runtime_presentation::RuntimePresentationSnapshot {
        let hooks = (0..20)
            .map(|index| {
                json!({
                    "key": format!("fixture:0:{index}"),
                    "eventName": format!("FutureEvent{index:02}"),
                    "handlerType": "command",
                    "command": format!("event-handler-{index:02}"),
                    "enabled": true,
                    "isManaged": false,
                    "trustStatus": "trusted"
                })
            })
            .collect::<Vec<_>>();
        crate::runtime_presentation::RuntimePresentationSnapshot::from_codex_hooks_list(
            &json!({"result":{"data":[{"cwd":"C:/synthetic/many-events","hooks":hooks}]}}),
            1_000,
        )
        .unwrap()
    }

    fn many_handlers_catalog() -> crate::runtime_presentation::RuntimePresentationSnapshot {
        let hooks = (0..20)
            .map(|index| {
                json!({
                    "key": format!("fixture:0:{index}"),
                    "eventName": "PreToolUse",
                    "handlerType": "command",
                    "command": format!("handler-{index:02}"),
                    "enabled": true,
                    "isManaged": false,
                    "trustStatus": "trusted"
                })
            })
            .collect::<Vec<_>>();
        crate::runtime_presentation::RuntimePresentationSnapshot::from_codex_hooks_list(
            &json!({"result":{"data":[{"cwd":"C:/synthetic/many-handlers","hooks":hooks}]}}),
            1_000,
        )
        .unwrap()
    }

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
                evidence_generation: crate::domain::EvidenceGeneration::SyntheticFixture,
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
    fn hooks_control_center_renders_runtime_truth_before_reliability_in_both_locales() {
        let mut app = control_center_app();
        let events = rendered(app.clone(), ResolvedLocale::EnUs, 140, 42);
        assert!(events.contains("Event"));
        assert!(events.contains("Installed"));
        assert!(events.contains("Active"));
        assert!(events.contains("Review"));
        assert!(events.contains("Before a tool executes"));
        assert!(events.contains("Interrupt"));
        assert!(events.contains("FutureRuntimeEvent"));
        assert!(events.contains("Session end"));
        assert!(events.contains("synthetic catalog warning"));
        assert!(events.contains("synthetic catalog error"));

        let chinese = rendered(app.clone(), ResolvedLocale::ZhCn, 140, 42);
        let chinese = chinese.replace(' ', "");
        assert!(chinese.contains("已安装"));
        assert!(chinese.contains("运行时问题"));

        for _ in 0..6 {
            app.handle(super::super::keymap::Command::Down);
        }
        app.handle(super::super::keymap::Command::Enter);
        let handlers = rendered(app.clone(), ResolvedLocale::EnUs, 100, 40);
        assert!(handlers.contains("synthetic command"));
        assert!(handlers.contains("Trusted"));
        assert!(handlers.contains("Not observed"));

        app.handle(super::super::keymap::Command::Enter);
        let detail = rendered(app.clone(), ResolvedLocale::EnUs, 140, 58);
        assert!(detail.contains("Runtime configuration"));
        assert!(detail.contains("Reliability summary"));
        assert!(detail.contains("Observation history"));
        assert!(detail.contains("SyntheticToolWithAnIntentionallyLongSafeName"));
        assert!(detail.contains("very-long-safe-argument=1234567890"));
        assert!(detail.contains("C:/synthetic/very/long/source/hooks.json"));
        assert!(detail.contains("Timeout: 9s"));
        assert!(detail.contains("Additional context: 64"));

        let narrow = rendered(app, ResolvedLocale::ZhCn, 44, 44).replace(' ', "");
        assert!(narrow.contains("运行时配置"));
        assert!(narrow.contains("可靠性摘要"));
    }

    #[test]
    fn hooks_control_center_renders_joined_health_errors_unknown_types_and_selected_rows() {
        let cases = [
            (
                EvidenceCoverage::Complete,
                TerminalStatus::Completed,
                Health::Healthy,
            ),
            (
                EvidenceCoverage::Complete,
                TerminalStatus::Failed,
                Health::Degraded,
            ),
            (
                EvidenceCoverage::Partial,
                TerminalStatus::Completed,
                Health::CoverageLimited,
            ),
            (
                EvidenceCoverage::Complete,
                TerminalStatus::Incomplete,
                Health::NoTerminalSamples,
            ),
        ];
        for (coverage, status, expected) in cases {
            let app = runtime_health_app(coverage, status);
            let event = app.selected_runtime_event().unwrap().clone();
            let handler = event.handlers[0].clone();
            assert_eq!(
                runtime_handler_health(ResolvedLocale::EnUs, &app, &event, &handler),
                health_name(ResolvedLocale::EnUs, expected)
            );
            assert_eq!(
                runtime_event_health(ResolvedLocale::EnUs, &app, &event),
                health_name(ResolvedLocale::EnUs, expected)
            );
        }

        let mut mixed = runtime_health_app(EvidenceCoverage::Complete, TerminalStatus::Completed);
        let mut mixed_catalog = control_center_catalog();
        let event = mixed_catalog
            .events
            .iter_mut()
            .find(|event| event.runtime_event_name == "PreToolUse")
            .unwrap();
        let mut unobserved = event.handlers[0].clone();
        unobserved.runtime_catalog_id = "fixture:0:unobserved".into();
        unobserved.reliability_handler_key = Some("hk_unobserved".into());
        event.handlers.push(unobserved);
        mixed.apply_runtime_catalog(mixed_catalog);
        let event = mixed.selected_runtime_event().unwrap().clone();
        assert_eq!(
            runtime_event_health(ResolvedLocale::EnUs, &mixed, &event),
            t(ResolvedLocale::EnUs, MessageKey::StateNotObserved)
        );

        let mut app = runtime_health_app(EvidenceCoverage::Complete, TerminalStatus::Failed);
        app.reject_refresh();
        let events = rendered(app.clone(), ResolvedLocale::EnUs, 120, 36);
        assert!(events.contains("Refresh failed; accepted history retained."));
        app.handle(super::super::keymap::Command::Enter);
        app.handle(super::super::keymap::Command::Enter);
        let detail = rendered(app, ResolvedLocale::EnUs, 120, 100);
        assert!(detail.contains("Health: ! Degraded"));
        assert!(detail.contains("Health explanation"));

        let mut intelligence =
            runtime_health_app(EvidenceCoverage::Complete, TerminalStatus::Failed);
        intelligence.handle(super::super::keymap::Command::Enter);
        intelligence.handle(super::super::keymap::Command::Enter);
        let detail = rendered(intelligence, ResolvedLocale::EnUs, 140, 150);
        assert!(detail.contains("Metric scope: All observed time, all revisions"));
        let reliability_summary = &detail[detail.find("Reliability summary").unwrap()
            ..detail.find("Observation history").unwrap()];
        assert!(reliability_summary.contains("Risk:"));
        assert!(reliability_summary.contains("Reason:"));
        assert!(detail.contains("Current revision"));
        assert!(detail.contains("Current revision: runtime-heal…"));
        assert!(!detail.contains("Current revision: runtime-health-r1"));
        assert!(detail.contains("Observation status"));
        assert!(detail.contains("Reliability intelligence"));
        assert!(detail.contains("Recent failures"));
        assert!(detail.contains("Trends"));
        assert!(detail.contains("Revision comparison"));
        assert!(detail.contains("Failure fingerprints"));
        assert!(detail.contains("Advanced technical metadata"));
        assert!(detail.contains("Full revision: runtime-health-r1"));
        let technical_metadata = &detail[detail.find("Advanced technical metadata").unwrap()..];
        assert!(!technical_metadata.contains("Risk:"));
        assert!(!technical_metadata.contains("Reason:"));

        let mut unknown = control_center_app();
        unknown.handle(super::super::keymap::Command::Enter);
        let handlers = rendered(unknown.clone(), ResolvedLocale::EnUs, 100, 36);
        assert!(handlers.contains("future_handler"));
        unknown.handle(super::super::keymap::Command::Enter);
        let detail = rendered(unknown, ResolvedLocale::EnUs, 100, 48);
        assert!(detail.contains("Handler type: future_handler"));

        let mut disabled = control_center_app();
        for _ in 0..4 {
            disabled.handle(super::super::keymap::Command::Down);
        }
        disabled.handle(super::super::keymap::Command::Enter);
        let handlers = rendered(disabled.clone(), ResolvedLocale::EnUs, 100, 36);
        assert!(handlers.contains("Enabled: Disabled"));
        assert!(handlers.contains("Untrusted"));
        disabled.handle(super::super::keymap::Command::Enter);
        let detail = rendered(disabled, ResolvedLocale::EnUs, 100, 48);
        assert!(detail.contains("Enabled: false"));
        assert!(!detail.contains("Disabled: false"));
    }

    #[test]
    fn hooks_control_center_keeps_large_event_and_handler_selections_visible() {
        let mut events = App::from_report(synthetic_fixture_report(1_000));
        events.apply_runtime_catalog(many_events_catalog());
        events.handle(super::super::keymap::Command::Down);
        events.handle(super::super::keymap::Command::Enter);
        events.handle(super::super::keymap::Command::PageDown);
        assert_eq!(events.runtime_event_selection_index(), Some(5));
        assert!(rendered(events, ResolvedLocale::EnUs, 44, 24).contains("FutureEvent05"));

        let mut handlers = App::from_report(synthetic_fixture_report(1_000));
        handlers.apply_runtime_catalog(many_handlers_catalog());
        handlers.handle(super::super::keymap::Command::Down);
        handlers.handle(super::super::keymap::Command::Enter);
        for _ in 0..5 {
            handlers.handle(super::super::keymap::Command::Down);
        }
        handlers.handle(super::super::keymap::Command::Enter);
        handlers.handle(super::super::keymap::Command::PageDown);
        assert_eq!(handlers.runtime_handler_selection_index(), Some(5));
        assert!(rendered(handlers, ResolvedLocale::EnUs, 44, 24).contains("handler-05"));

        let mut fallback_catalog = many_handlers_catalog();
        let event = fallback_catalog
            .events
            .iter_mut()
            .find(|event| event.runtime_event_name == "PreToolUse")
            .unwrap();
        for handler in &mut event.handlers {
            handler.handler_kind = crate::runtime_presentation::RuntimeHandlerKind::Prompt;
        }
        let mut fallback = App::from_report(synthetic_fixture_report(1_000));
        fallback.apply_runtime_catalog(fallback_catalog);
        fallback.handle(super::super::keymap::Command::Down);
        fallback.handle(super::super::keymap::Command::Enter);
        for _ in 0..5 {
            fallback.handle(super::super::keymap::Command::Down);
        }
        fallback.handle(super::super::keymap::Command::Enter);
        fallback.handle(super::super::keymap::Command::PageDown);
        let fallback_rendered = rendered(fallback, ResolvedLocale::EnUs, 44, 24);
        assert!(fallback_rendered.contains("hook 6"));
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
    fn historical_detail_rendering_ignores_stale_runtime_selection() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.apply_runtime_catalog(control_center_catalog());
        app.handle(super::super::keymap::Command::Down);
        app.handle(super::super::keymap::Command::Enter);
        app.handle(super::super::keymap::Command::Enter);
        app.handle(super::super::keymap::Command::Enter);
        assert_eq!(app.screen(), Screen::HookDetail);
        assert!(app.selected_runtime_handler().is_some());

        app.handle(super::super::keymap::Command::Back);
        app.handle(super::super::keymap::Command::Back);
        app.handle(super::super::keymap::Command::Back);
        app.handle(super::super::keymap::Command::Up);
        assert_eq!(app.screen(), Screen::Overview);
        app.handle(super::super::keymap::Command::Enter);
        assert_eq!(app.screen(), Screen::HookDetail);

        let historical = rendered(app, ResolvedLocale::EnUs, 120, 70);
        assert!(historical.contains("Reliability intelligence"));
        assert!(!historical.contains("Runtime configuration"));
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
        assert!(english.contains("risk ("));
        assert!(english.contains("Reason:"));
        assert!(english.contains("n="));

        let detail = app
            .selected_handler()
            .and_then(|reference| app.view_model().and_then(|view| view.detail(reference)))
            .expect("synthetic fixture selects a hook detail");
        let english_revision = revision_detail(ResolvedLocale::EnUs, &detail.revision_comparison);
        assert!(english_revision.contains("Current revision:"));
        assert!(english_revision.contains("All observed time, current revision"));
        assert!(english_revision.contains("Previous revision:"));
        assert!(english_revision.contains("All observed time, previous revision"));

        let chinese = rendered(app.clone(), ResolvedLocale::ZhCn, 100, 40).replace(' ', "");
        assert!(chinese.contains("可靠性智能"));
        assert!(chinese.contains("趋势"));
        assert!(chinese.contains("风险"));
        assert!(chinese.contains("原因:"));
        assert!(chinese.contains("样本="));

        let chinese_revision =
            revision_detail(ResolvedLocale::ZhCn, &detail.revision_comparison).replace(' ', "");
        assert!(chinese_revision.contains("当前修订版本:"));
        assert!(chinese_revision.contains("全部观测时间，当前修订版本"));
        assert!(chinese_revision.contains("上一修订版本:"));
        assert!(chinese_revision.contains("全部观测时间，上一修订版本"));
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
        assert!(
            empty_buffer
                .replace(' ', "")
                .contains("正在加载当前运行时目录")
        );

        let mut diagnostics = DiagnosticsReport::empty(1_000);
        diagnostics.overall_status = DiagnosticStatus::Fail;
        diagnostics.checks = vec![DiagnosticCheck {
            id: DiagnosticCheckId::ReceiptIntegrity,
            status: DiagnosticStatus::Fail,
            facts: vec![],
        }];
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
        assert!(detail.contains("指标范围"));
        assert!(detail.contains("变更发生时间"));
        assert!(detail.contains("首次发现"));
        assert!(detail.contains("最后发现"));
        assert!(detail.contains("修订版本"));
        assert!(!detail.contains("1728000000"));
        assert!(!detail.contains("1036800000"));
    }

    #[test]
    fn change_detail_scope_stays_bound_to_accepted_data_during_refresh() {
        let mut app = changes_app();
        app.handle(super::super::keymap::Command::Enter);
        app.handle(super::super::keymap::Command::Enter);
        assert_eq!(app.screen(), Screen::ChangeDetail);

        app.handle(super::super::keymap::Command::Window(TimeWindow::Today));
        let detail = rendered(app, ResolvedLocale::EnUs, 100, 40);
        assert!(detail.contains("Metric scope: Selected Last 7 days, all revisions"));
        assert!(!detail.contains("Metric scope: Selected Today, all revisions"));
    }

    #[test]
    fn narrow_tall_change_detail_scrolls_into_human_timeline() {
        let mut app = changes_app();
        app.handle(super::super::keymap::Command::Enter);
        app.handle(super::super::keymap::Command::Enter);
        assert_eq!(app.screen(), Screen::ChangeDetail);

        app.handle(super::super::keymap::Command::PageDown);
        app.handle(super::super::keymap::Command::PageDown);
        let detail = rendered(app, ResolvedLocale::EnUs, 44, 40);
        assert!(detail.contains("Timeline"));
        assert!(detail.contains("Advanced technical"));
        assert!(detail.contains("metadata"));
        assert!(detail.contains("Full revision"));
        assert!(detail.contains("r1 ·"));
        assert!(detail.contains("r2 ·"));
        assert!(!detail.replace(' ', "").contains("1728000000"));
    }

    #[test]
    fn scope_pending_state_risk_reasons_and_cluster_details_stay_visible() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        let overview = rendered(app.clone(), ResolvedLocale::EnUs, 120, 40);
        assert!(overview.contains("Metric scope: Selected Last 7 days, all revisions"));
        assert!(overview.contains("Reason:"));

        app.handle(super::super::keymap::Command::Down);
        app.handle(super::super::keymap::Command::Enter);
        assert_eq!(app.screen(), Screen::Hooks);
        let hooks = rendered(app.clone(), ResolvedLocale::EnUs, 120, 40);
        assert!(hooks.contains("Loading current runtime catalog"));
        let compact_hooks = rendered(app.clone(), ResolvedLocale::EnUs, 44, 40);
        assert!(compact_hooks.contains("Loading"));
        assert!(compact_hooks.contains("catalog"));
        let compact_short = rendered(app.clone(), ResolvedLocale::EnUs, 44, 16);
        assert!(compact_short.contains("Loading"));
        app.handle(super::super::keymap::Command::Window(TimeWindow::Today));
        let pending_hooks = rendered(app, ResolvedLocale::EnUs, 120, 40);
        assert!(pending_hooks.contains("Loading current runtime catalog"));

        let mut changes = changes_app();
        let changes_list = rendered(changes.clone(), ResolvedLocale::EnUs, 100, 40);
        assert!(changes_list.contains("Metric scope: Selected Last 7 days, all revisions"));
        let compact_changes = rendered(changes.clone(), ResolvedLocale::EnUs, 44, 40);
        assert!(compact_changes.contains("Metric scope"));
        assert!(compact_changes.contains("Selected"));
        assert!(compact_changes.contains("Last 7"));
        assert!(compact_changes.contains("days"));
        changes.handle(super::super::keymap::Command::Window(TimeWindow::Today));
        let pending_changes = rendered(changes, ResolvedLocale::EnUs, 100, 40);
        assert!(pending_changes.contains("Loading accepted reliability data"));
        assert!(pending_changes.contains("Selected Last 7 days, all revisions"));
        assert!(!pending_changes.contains("Metric scope: Selected Today, all revisions"));

        let mut clusters = App::from_report(synthetic_fixture_report(1_000));
        clusters.handle(super::super::keymap::Command::Enter);
        assert_eq!(clusters.screen(), Screen::HookDetail);
        clusters.handle(super::super::keymap::Command::Filter);
        assert_eq!(clusters.screen(), Screen::FailureClusters);
        let list = rendered(clusters.clone(), ResolvedLocale::EnUs, 120, 40);
        assert!(list.contains("Metric scope: Selected Last 7 days, all revisions"));
        let compact_clusters = rendered(clusters.clone(), ResolvedLocale::EnUs, 44, 40);
        assert!(compact_clusters.contains("Metric scope"));
        assert!(compact_clusters.contains("Selected"));
        assert!(compact_clusters.contains("Last 7"));
        assert!(compact_clusters.contains("days"));
        clusters.handle(super::super::keymap::Command::Window(TimeWindow::Today));
        let pending_clusters = rendered(clusters.clone(), ResolvedLocale::EnUs, 120, 40);
        assert!(pending_clusters.contains("Loading accepted reliability data"));
        assert!(pending_clusters.contains("Selected Last 7 days, all revisions"));

        clusters.handle(super::super::keymap::Command::Enter);
        assert_eq!(clusters.screen(), Screen::FailureClusterDetail);
        let detail = rendered(clusters, ResolvedLocale::EnUs, 44, 40);
        assert!(detail.contains("Metric scope"));
        assert!(detail.contains("Coverage"));
        assert!(detail.contains("Affected hooks"));
        assert!(!detail.contains("1728000000"));
    }

    #[test]
    fn human_reliability_helpers_make_scope_coverage_risk_and_revision_explicit() {
        assert_eq!(
            selected_scope(ResolvedLocale::EnUs, TimeWindow::Last7Days),
            "Selected Last 7 days, all revisions"
        );
        assert_eq!(
            current_revision_scope(ResolvedLocale::EnUs),
            "All observed time, current revision"
        );
        assert_eq!(
            previous_revision_scope(ResolvedLocale::EnUs),
            "All observed time, previous revision"
        );
        assert_eq!(
            revision_timeline_scope(ResolvedLocale::EnUs),
            "All observed time, revision timeline"
        );
        assert!(
            coverage_summary(ResolvedLocale::EnUs, EvidenceCoverage::Partial)
                .contains("terminal coverage is incomplete")
        );
        assert_eq!(
            risk_reason(ResolvedLocale::EnUs, 0, 0, EvidenceCoverage::Partial),
            "no terminal samples; this is not a healthy result."
        );
        assert_eq!(
            presentation_health(EvidenceCoverage::Partial, 0, 0),
            Health::NoTerminalSamples
        );
        assert_eq!(short_revision("0123456789abcdef"), "0123456789ab…");
    }

    #[test]
    fn catalog_alias_and_failure_cluster_surfaces_are_bilingual_and_narrow_safe() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.handle(super::super::keymap::Command::Enter);
        assert_eq!(app.screen(), Screen::HookDetail);
        app.handle(super::super::keymap::Command::EditAlias);
        app.handle(super::super::keymap::Command::SearchInput('名'));
        let chinese = rendered(app.clone(), ResolvedLocale::ZhCn, 100, 40).replace(' ', "");
        assert!(chinese.contains("人类别名"));
        assert!(chinese.contains("别名变更已暂存"));

        app.handle(super::super::keymap::Command::Back);
        app.handle(super::super::keymap::Command::Filter);
        assert_eq!(app.screen(), Screen::FailureClusters);
        let english = rendered(app.clone(), ResolvedLocale::EnUs, 100, 30);
        assert!(english.contains("Failure clusters"));
        assert!(english.contains("Occurrences"));
        assert!(rendered(app.clone(), ResolvedLocale::EnUs, 44, 16).contains("Failure"));
        app.handle(super::super::keymap::Command::Enter);
        assert_eq!(app.screen(), Screen::FailureClusterDetail);
        let detail = rendered(app, ResolvedLocale::EnUs, 100, 30);
        assert!(detail.contains("Affected hooks"));
        assert!(detail.contains("Coverage"));
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
