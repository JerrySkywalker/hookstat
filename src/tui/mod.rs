//! HookStat's internal implementation of the Jerry Terminal UI System.
//!
//! The module intentionally remains internal until a second conforming
//! application proves a stable shared-crate boundary (ADR 0004).

mod app;
mod keymap;
mod layout;
mod localization;
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
    let mut guard = terminal::TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    let app = App::from_snapshot(initial);
    let refresh: Refresh = Box::new(refresh);
    let result = run_loop(&mut terminal, app, RefreshController::spawn(refresh));
    drop(terminal);
    result.and(guard.restore())
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
    mut refresh: RefreshController<RefreshReason, RefreshSnapshot>,
) -> io::Result<()> {
    let environment_locale = std::env::var("HOOKSTAT_LANG").ok();
    let language = localization::LanguageState::resolve(
        localization::InterfaceLanguage::Auto,
        environment_locale.as_deref(),
        None,
    );
    let theme = theme::Theme::from_environment();
    loop {
        match refresh.poll() {
            RefreshPoll::Ready(snapshot) => app.apply_refresh(snapshot),
            RefreshPoll::Failed => app.reject_refresh(),
            RefreshPoll::WorkerUnavailable => app.worker_unavailable(),
            RefreshPoll::Pending | RefreshPoll::Stale => {}
        }
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
                }
            }
            _ => {}
        };
    }
}

fn now_unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis() as i64)
        .unwrap_or(0)
}
