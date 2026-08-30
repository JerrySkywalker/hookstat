//! HookStat's internal implementation of the Jerry Terminal UI System.
//!
//! Product-specific rendering adapts the shared `terminal-ui-contract` semantic
//! core to HookStat's Ratatui/Crossterm versions and reliability content.

mod app;
#[cfg(test)]
mod conformance;
mod human_time;
mod keymap;
mod layout;
pub mod localization;
mod navigation;
mod refresh;
mod rendering;
mod state;
mod terminal;
mod theme;
mod view_model;
mod widgets;

pub use app::{
    AliasAnnotation, AliasApplyOutcome, AliasApplyRequest, ChangesRefreshReason, ChangesSnapshot,
    DiagnosticsRefreshReason, RefreshReason, RefreshSnapshot, RuntimeCatalogRefreshReason,
};

use crate::diagnostics::DiagnosticsReport;
use crate::domain::HookInvocation;
use crate::interface_preferences::{
    InterfacePreferenceSnapshot, InterfacePreferencesStore, PreferenceSaveOutcome,
};
use crate::observability::{StartupObservatory, StartupPhase};
use crate::runtime_presentation::RuntimePresentationSnapshot;
use app::{App, AppEffect};
use crossterm::event::{self, Event};
use ratatui::{Terminal, backend::CrosstermBackend};
use refresh::{RefreshController, RefreshPoll, RefreshRequest};
use std::io;
use std::time::Duration;

type Refresh =
    Box<dyn FnMut(RefreshRequest<RefreshReason>) -> Result<RefreshSnapshot, String> + Send>;
type DiagnosticsRefresh = Box<
    dyn FnMut(RefreshRequest<DiagnosticsRefreshReason>) -> Result<DiagnosticsReport, String> + Send,
>;
type ChangesRefresh =
    Box<dyn FnMut(RefreshRequest<ChangesRefreshReason>) -> Result<ChangesSnapshot, String> + Send>;
type RuntimeCatalogRefresh = Box<
    dyn FnMut(
            RefreshRequest<RuntimeCatalogRefreshReason>,
        ) -> Result<RuntimePresentationSnapshot, String>
        + Send,
>;
type AliasApply = Box<dyn FnMut(AliasApplyRequest) -> AliasApplyOutcome + Send>;

struct RunLoopOptions {
    explicit_language: Option<localization::InterfaceLanguage>,
    store: Option<InterfacePreferencesStore>,
    preference_snapshot: Option<InterfacePreferenceSnapshot>,
    observatory: Option<StartupObservatory>,
}

pub fn run(
    values: Vec<HookInvocation>,
    malformed: u64,
    incomplete: u64,
    now: i64,
) -> io::Result<()> {
    let worker_values = values.clone();
    let initial = RefreshSnapshot::from_values(
        values,
        malformed,
        incomplete,
        now,
        crate::analytics::TimeWindow::Last7Days,
    );
    run_with_refresh_snapshot(initial, move |request| {
        Ok(RefreshSnapshot::from_values(
            worker_values.clone(),
            malformed,
            incomplete,
            now_unix_ms(),
            request.reason.window(),
        ))
    })
}

/// Start the TUI with an admitted, read-only source callback. The callback is
/// executed on the refresh worker, never from rendering or key handling.
pub fn run_with_refresh(
    values: Vec<HookInvocation>,
    malformed: u64,
    incomplete: u64,
    now: i64,
    refresh: impl FnMut(RefreshRequest<RefreshReason>) -> Result<RefreshSnapshot, String>
    + Send
    + 'static,
) -> io::Result<()> {
    let initial = RefreshSnapshot::from_values(
        values,
        malformed,
        incomplete,
        now,
        crate::analytics::TimeWindow::Last7Days,
    );
    run_with_refresh_snapshot(initial, refresh)
}

/// Start the TUI from a completed presentation snapshot. The caller prepares
/// the initial view model before the terminal event loop is created.
pub fn run_with_refresh_snapshot(
    initial: RefreshSnapshot,
    refresh: impl FnMut(RefreshRequest<RefreshReason>) -> Result<RefreshSnapshot, String>
    + Send
    + 'static,
) -> io::Result<()> {
    run_with_refresh_snapshot_language(initial, None, refresh)
}

/// Starts the TUI with an optional explicit, non-persistent locale override.
pub fn run_with_refresh_snapshot_language(
    initial: RefreshSnapshot,
    explicit_language: Option<localization::InterfaceLanguage>,
    refresh: impl FnMut(RefreshRequest<RefreshReason>) -> Result<RefreshSnapshot, String>
    + Send
    + 'static,
) -> io::Result<()> {
    let mut guard = terminal::TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    let store = crate::codex::default_data_root()
        .ok()
        .map(|root| InterfacePreferencesStore::new(root.join("interface.toml")));
    let snapshot = store
        .as_ref()
        .and_then(|store| store.snapshot_read_only().ok());
    let mut app = App::from_snapshot(initial);
    if let Some(snapshot) = &snapshot {
        app.set_persisted_interface(snapshot.language(), snapshot.color());
    }
    let refresh: Refresh = Box::new(refresh);
    let result = run_loop(
        &mut terminal,
        app,
        RefreshController::spawn(refresh),
        None,
        None,
        None,
        None,
        RunLoopOptions {
            explicit_language,
            store,
            preference_snapshot: snapshot,
            observatory: None,
        },
    );
    drop(terminal);
    result.and(guard.restore())
}

/// Starts from an empty/loading application model. The first terminal frame is
/// independent of receipt reconciliation, SQLite queries, analytics, and
/// diagnostics discovery; both worker paths publish immutable snapshots later.
#[allow(
    clippy::too_many_arguments,
    reason = "the public TUI entry point accepts the independent, typed refresh paths"
)]
pub fn run_loading_with_refreshes_language(
    initial_window: crate::analytics::TimeWindow,
    explicit_language: Option<localization::InterfaceLanguage>,
    reliability_refresh: impl FnMut(RefreshRequest<RefreshReason>) -> Result<RefreshSnapshot, String>
    + Send
    + 'static,
    diagnostics_refresh: impl FnMut(
        RefreshRequest<DiagnosticsRefreshReason>,
    ) -> Result<DiagnosticsReport, String>
    + Send
    + 'static,
    changes_refresh: impl FnMut(RefreshRequest<ChangesRefreshReason>) -> Result<ChangesSnapshot, String>
    + Send
    + 'static,
    runtime_catalog_refresh: impl FnMut(
        RefreshRequest<RuntimeCatalogRefreshReason>,
    ) -> Result<RuntimePresentationSnapshot, String>
    + Send
    + 'static,
    alias_apply: impl FnMut(AliasApplyRequest) -> AliasApplyOutcome + Send + 'static,
    observatory: StartupObservatory,
) -> io::Result<()> {
    let mut guard = terminal::TerminalGuard::enter()?;
    observatory.mark(StartupPhase::TerminalGuardEntered);
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    let store = crate::codex::default_data_root()
        .ok()
        .map(|root| InterfacePreferencesStore::new(root.join("interface.toml")));
    let snapshot = store
        .as_ref()
        .and_then(|store| store.snapshot_read_only().ok());
    let mut app = App::loading(initial_window);
    if let Some(snapshot) = &snapshot {
        app.set_persisted_interface(snapshot.language(), snapshot.color());
    }
    let refresh: Refresh = Box::new(reliability_refresh);
    let diagnostics: DiagnosticsRefresh = Box::new(diagnostics_refresh);
    let changes: ChangesRefresh = Box::new(changes_refresh);
    let runtime_catalog: RuntimeCatalogRefresh = Box::new(runtime_catalog_refresh);
    let alias_apply: AliasApply = Box::new(alias_apply);
    let result = run_loop(
        &mut terminal,
        app,
        RefreshController::spawn(refresh),
        Some(RefreshController::spawn(runtime_catalog)),
        Some(RefreshController::spawn(diagnostics)),
        Some(RefreshController::spawn(changes)),
        Some(alias_apply),
        RunLoopOptions {
            explicit_language,
            store,
            preference_snapshot: snapshot,
            observatory: Some(observatory),
        },
    );
    drop(terminal);
    result.and(guard.restore())
}

#[allow(
    clippy::too_many_arguments,
    reason = "the loop owns independent resources whose failure states must remain isolated"
)]
fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
    mut refresh: RefreshController<RefreshReason, RefreshSnapshot>,
    mut runtime_catalog: Option<
        RefreshController<RuntimeCatalogRefreshReason, RuntimePresentationSnapshot>,
    >,
    mut diagnostics: Option<RefreshController<DiagnosticsRefreshReason, DiagnosticsReport>>,
    mut changes: Option<RefreshController<ChangesRefreshReason, ChangesSnapshot>>,
    mut alias_apply: Option<AliasApply>,
    mut options: RunLoopOptions,
) -> io::Result<()> {
    let environment_locale = std::env::var("HOOKSTAT_LANG").ok();
    let system_locale = operating_system_locale();
    let initial_reliability_request = app
        .view_model()
        .is_none()
        .then_some(RefreshReason::Window(app.requested_window()));
    let initial_diagnostics_request = diagnostics
        .as_ref()
        .map(|_| DiagnosticsRefreshReason::Initial);
    let initial_runtime_catalog_request = runtime_catalog.as_ref().and_then(|_| {
        app.runtime_catalog_initial_load_pending()
            .then_some(RuntimeCatalogRefreshReason::Initial)
    });
    let mut first_frame_drawn = false;
    loop {
        match refresh.poll() {
            RefreshPoll::Ready { generation, value } => {
                app.apply_refresh(value);
                if let Some(observatory) = options.observatory.as_ref() {
                    observatory.record_accepted_generation(generation);
                    observatory.mark(StartupPhase::ReliabilitySnapshotReady);
                }
            }
            RefreshPoll::Failed => app.reject_refresh(),
            RefreshPoll::WorkerUnavailable => app.worker_unavailable(),
            RefreshPoll::Pending | RefreshPoll::Stale => {}
        }
        if let Some(runtime_catalog) = &mut runtime_catalog {
            match runtime_catalog.poll() {
                RefreshPoll::Ready { value, .. } => app.apply_runtime_catalog(value),
                RefreshPoll::Failed | RefreshPoll::WorkerUnavailable => {
                    app.reject_runtime_catalog()
                }
                RefreshPoll::Pending | RefreshPoll::Stale => {}
            }
        }
        if let Some(diagnostics) = &mut diagnostics {
            match diagnostics.poll() {
                RefreshPoll::Ready { value, .. } => {
                    app.apply_diagnostics(value);
                    if let Some(observatory) = options.observatory.as_ref() {
                        observatory.mark(StartupPhase::DiagnosticsReady);
                    }
                }
                RefreshPoll::Failed | RefreshPoll::WorkerUnavailable => app.reject_diagnostics(),
                RefreshPoll::Pending | RefreshPoll::Stale => {}
            }
        }
        if let Some(changes) = &mut changes {
            match changes.poll() {
                RefreshPoll::Ready { value, .. } => app.apply_changes(value),
                RefreshPoll::Failed | RefreshPoll::WorkerUnavailable => app.reject_changes(),
                RefreshPoll::Pending | RefreshPoll::Stale => {}
            }
        }
        let language = resolved_language(
            &app,
            options.explicit_language,
            environment_locale.as_deref(),
            system_locale.as_deref(),
        );
        let color = if app.settings_dirty() {
            app.draft_color()
        } else {
            app.accepted_color()
        };
        let theme = theme::Theme::from_interface_color(color);
        terminal.draw(|frame| rendering::draw(frame, &app, language, theme))?;
        if !first_frame_drawn {
            first_frame_drawn = true;
            if let Some(observatory) = options.observatory.as_ref() {
                observatory.mark(StartupPhase::FirstFrameDrawn);
            }
            // Queue workers only after the loading shell has reached the
            // terminal. Sending a request is non-blocking; the worker owns all
            // subsequent I/O, probing, and aggregation.
            if let Some(reason) = initial_reliability_request {
                let generation = refresh.request(reason);
                if let Some(observatory) = options.observatory.as_ref() {
                    observatory.record_requested_generation(generation);
                }
            }
            if let Some(reason) = initial_runtime_catalog_request
                && let Some(runtime_catalog) = &mut runtime_catalog
            {
                runtime_catalog.request(reason);
            }
            if let Some(reason) = initial_diagnostics_request
                && let Some(diagnostics) = &mut diagnostics
            {
                diagnostics.request(reason);
            }
        }
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        match event::read()? {
            Event::Resize(_, _) => {
                terminal.autoresize()?;
            }
            Event::Key(key) => {
                let Some(command) = keymap::command_for(key, app.is_text_editing()) else {
                    continue;
                };
                match app.handle(command) {
                    AppEffect::None => {}
                    AppEffect::Quit => return Ok(()),
                    AppEffect::RequestRefresh(reason) => {
                        let generation = refresh.request(reason);
                        if let Some(observatory) = options.observatory.as_ref() {
                            observatory.record_requested_generation(generation);
                        }
                    }
                    AppEffect::RequestRefreshAndRuntimeCatalog(refresh_reason, catalog_reason) => {
                        let generation = refresh.request(refresh_reason);
                        if let Some(observatory) = options.observatory.as_ref() {
                            observatory.record_requested_generation(generation);
                        }
                        if let Some(runtime_catalog) = &mut runtime_catalog {
                            runtime_catalog.request(catalog_reason);
                        } else {
                            app.reject_runtime_catalog();
                        }
                    }
                    AppEffect::RequestDiagnostics(reason) => {
                        if let Some(diagnostics) = &mut diagnostics {
                            diagnostics.request(reason);
                        } else {
                            app.reject_diagnostics();
                        }
                    }
                    AppEffect::RequestChanges(reason) => {
                        if let Some(changes) = &mut changes {
                            changes.request(reason);
                        } else {
                            app.reject_changes();
                        }
                    }
                    AppEffect::RequestRefreshAndChanges(refresh_reason, changes_reason) => {
                        let generation = refresh.request(refresh_reason);
                        if let Some(observatory) = options.observatory.as_ref() {
                            observatory.record_requested_generation(generation);
                        }
                        if let Some(changes) = &mut changes {
                            changes.request(changes_reason);
                        } else {
                            app.reject_changes();
                        }
                    }
                    AppEffect::ApplyAlias(request) => {
                        let outcome = alias_apply
                            .as_mut()
                            .map_or(AliasApplyOutcome::Failed, |apply| apply(request));
                        app.alias_apply_result(outcome);
                        if outcome == AliasApplyOutcome::Saved {
                            let generation =
                                refresh.request(RefreshReason::Manual(app.requested_window()));
                            if let Some(observatory) = options.observatory.as_ref() {
                                observatory.record_requested_generation(generation);
                            }
                        }
                    }
                    AppEffect::ApplyInterface { language, color } => {
                        let Some(store) = &options.store else {
                            app.language_save_failed();
                            continue;
                        };
                        let Some(snapshot) = &options.preference_snapshot else {
                            app.language_save_failed();
                            continue;
                        };
                        match store.save_if_unchanged(snapshot, language, color) {
                            Ok(PreferenceSaveOutcome::Saved) => {
                                options.preference_snapshot = store.snapshot_read_only().ok();
                                app.language_saved();
                            }
                            Ok(PreferenceSaveOutcome::Conflict) => app.language_save_conflict(),
                            Err(_) => app.language_save_failed(),
                        }
                    }
                }
            }
            _ => {}
        };
    }
}

fn operating_system_locale() -> Option<String> {
    sys_locale::get_locale().or_else(|| {
        ["LC_ALL", "LC_MESSAGES", "LANG"]
            .into_iter()
            .find_map(|name| std::env::var(name).ok())
    })
}

fn resolved_language(
    app: &App,
    explicit_language: Option<localization::InterfaceLanguage>,
    environment_locale: Option<&str>,
    system_locale: Option<&str>,
) -> localization::LanguageState {
    let requested = if app.settings_dirty() {
        app.draft_language()
    } else {
        explicit_language.unwrap_or(localization::InterfaceLanguage::Auto)
    };
    let persisted = (!app.settings_dirty()).then_some(app.accepted_language());
    localization::LanguageState::resolve(requested, environment_locale, persisted, system_locale)
}

fn now_unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::synthetic_fixture_report;

    #[test]
    fn staged_language_is_used_immediately_without_waiting_for_persistence() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.set_persisted_interface(
            localization::InterfaceLanguage::EnUs,
            crate::interface_preferences::InterfaceColor::Auto,
        );
        app.handle(keymap::Command::Down);
        app.handle(keymap::Command::Down);
        app.handle(keymap::Command::Down);
        app.handle(keymap::Command::Down);
        app.handle(keymap::Command::Enter);
        app.handle(keymap::Command::NextSetting);
        let staged = resolved_language(&app, None, None, None);
        assert_eq!(staged.resolved, localization::ResolvedLocale::ZhCn);
        app.language_saved();
        let persisted = resolved_language(&app, None, None, None);
        assert_eq!(persisted.resolved, localization::ResolvedLocale::ZhCn);
    }
}
