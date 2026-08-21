//! HookStat's internal implementation of the Jerry Terminal UI System.
//!
//! The module intentionally remains internal until a second conforming
//! application proves a stable shared-crate boundary (ADR 0004).

mod app;
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

pub use app::{RefreshReason, RefreshSnapshot};

use crate::domain::HookInvocation;
use crate::interface_preferences::{
    InterfacePreferenceSnapshot, InterfacePreferencesStore, PreferenceSaveOutcome,
};
use app::{App, AppEffect};
use crossterm::event::{self, Event};
use ratatui::{Terminal, backend::CrosstermBackend};
use refresh::{RefreshController, RefreshPoll, RefreshRequest};
use std::io;
use std::time::Duration;

type Refresh =
    Box<dyn FnMut(RefreshRequest<RefreshReason>) -> Result<RefreshSnapshot, String> + Send>;

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
        explicit_language,
        store,
        snapshot,
    );
    drop(terminal);
    result.and(guard.restore())
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
    mut refresh: RefreshController<RefreshReason, RefreshSnapshot>,
    explicit_language: Option<localization::InterfaceLanguage>,
    store: Option<InterfacePreferencesStore>,
    mut preference_snapshot: Option<InterfacePreferenceSnapshot>,
) -> io::Result<()> {
    let environment_locale = std::env::var("HOOKSTAT_LANG").ok();
    let system_locale = std::env::var("LANG").ok();
    loop {
        match refresh.poll() {
            RefreshPoll::Ready(snapshot) => app.apply_refresh(snapshot),
            RefreshPoll::Failed => app.reject_refresh(),
            RefreshPoll::WorkerUnavailable => app.worker_unavailable(),
            RefreshPoll::Pending | RefreshPoll::Stale => {}
        }
        let language = resolved_language(
            &app,
            explicit_language,
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
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        match event::read()? {
            Event::Resize(_, _) => {
                terminal.autoresize()?;
            }
            Event::Key(key) => {
                let Some(command) = keymap::command_for(key, app.is_search_editing()) else {
                    continue;
                };
                match app.handle(command) {
                    AppEffect::None => {}
                    AppEffect::Quit => return Ok(()),
                    AppEffect::RequestRefresh(reason) => {
                        refresh.request(reason);
                    }
                    AppEffect::ApplyInterface { language, color } => {
                        let Some(store) = &store else {
                            app.language_save_failed();
                            continue;
                        };
                        let Some(snapshot) = &preference_snapshot else {
                            app.language_save_failed();
                            continue;
                        };
                        match store.save_if_unchanged(snapshot, language, color) {
                            Ok(PreferenceSaveOutcome::Saved) => {
                                preference_snapshot = store.snapshot_read_only().ok();
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
        app.handle(keymap::Command::ToggleFocus);
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
