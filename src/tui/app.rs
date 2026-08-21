//! Terminal-independent application state and compatibility view model.

use crate::analytics::TimeWindow;
use crate::domain::HookInvocation;
use crate::report::{MachineReport, instrumented_report};

use super::keymap::Command;
use super::navigation::{NavigationState, Route};
use super::state::ResourceState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Focus {
    Content,
    Navigation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompatibilityScreen {
    Home,
    Detail,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RefreshReason {
    Manual(TimeWindow),
    Window(TimeWindow),
}

impl RefreshReason {
    pub const fn window(self) -> TimeWindow {
        match self {
            Self::Manual(window) | Self::Window(window) => window,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RefreshSnapshot {
    report: MachineReport,
}

impl RefreshSnapshot {
    /// This is intentionally called by the refresh worker, not by rendering or
    /// key handling. It keeps SQLite/receipt reads and aggregation off the UI
    /// event-loop thread.
    pub fn from_values(
        values: Vec<HookInvocation>,
        malformed: u64,
        incomplete: u64,
        now: i64,
        window: TimeWindow,
    ) -> Self {
        Self {
            report: instrumented_report(&values, now, window, malformed, incomplete),
        }
    }

    pub fn from_report(report: MachineReport) -> Self {
        Self { report }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEffect {
    None,
    Quit,
    RequestRefresh(RefreshReason),
}

#[derive(Clone, Debug)]
pub struct App {
    navigation: NavigationState,
    focus: Focus,
    compatibility_screen: CompatibilityScreen,
    requested_window: TimeWindow,
    selected_handler_key: Option<String>,
    report: ResourceState<MachineReport>,
}

impl App {
    pub fn new(values: Vec<HookInvocation>, malformed: u64, incomplete: u64, now: i64) -> Self {
        let report =
            instrumented_report(&values, now, TimeWindow::Last7Days, malformed, incomplete);
        Self::from_report(report)
    }

    pub fn from_report(report: MachineReport) -> Self {
        let selected_handler_key = report.handlers.first().map(|item| item.handler.key.clone());
        Self {
            navigation: NavigationState::new(),
            focus: Focus::Content,
            compatibility_screen: CompatibilityScreen::Home,
            requested_window: report.window,
            selected_handler_key,
            report: ResourceState::Ready(report),
        }
    }

    pub const fn navigation(&self) -> NavigationState {
        self.navigation
    }

    pub const fn focus(&self) -> Focus {
        self.focus
    }

    pub const fn compatibility_screen(&self) -> CompatibilityScreen {
        self.compatibility_screen
    }

    pub const fn report_state(&self) -> &ResourceState<MachineReport> {
        &self.report
    }

    pub fn selected_handler_index(&self) -> usize {
        let Some(report) = self.report.accepted() else {
            return 0;
        };
        self.selected_handler_key
            .as_deref()
            .and_then(|key| {
                report
                    .handlers
                    .iter()
                    .position(|item| item.handler.key == key)
            })
            .unwrap_or(0)
    }

    pub fn handle(&mut self, command: Command) -> AppEffect {
        match command {
            Command::Quit => AppEffect::Quit,
            Command::ToggleFocus => {
                self.focus = match self.focus {
                    Focus::Content => Focus::Navigation,
                    Focus::Navigation => Focus::Content,
                };
                AppEffect::None
            }
            Command::Up => {
                if self.focus == Focus::Navigation {
                    self.navigation.move_by(-1);
                } else {
                    self.move_handler(-1);
                }
                AppEffect::None
            }
            Command::Down => {
                if self.focus == Focus::Navigation {
                    self.navigation.move_by(1);
                } else {
                    self.move_handler(1);
                }
                AppEffect::None
            }
            Command::Enter => {
                if self.focus == Focus::Navigation {
                    self.navigation.activate_selected();
                    self.compatibility_screen = CompatibilityScreen::Home;
                } else if self.navigation.active() == Route::Overview
                    && self
                        .report
                        .accepted()
                        .is_some_and(|report| !report.handlers.is_empty())
                {
                    self.compatibility_screen = CompatibilityScreen::Detail;
                }
                AppEffect::None
            }
            Command::Back => {
                if self.compatibility_screen == CompatibilityScreen::Detail {
                    self.compatibility_screen = CompatibilityScreen::Home;
                } else {
                    self.navigation.back();
                }
                AppEffect::None
            }
            Command::Refresh => self.request_refresh(RefreshReason::Manual(self.requested_window)),
            Command::Window(window) => {
                self.requested_window = window;
                self.request_refresh(RefreshReason::Window(window))
            }
        }
    }

    pub fn apply_refresh(&mut self, snapshot: RefreshSnapshot) {
        self.requested_window = snapshot.report.window;
        self.report =
            std::mem::replace(&mut self.report, ResourceState::Empty).ready(snapshot.report);
        self.repair_handler_selection();
    }

    pub fn reject_refresh(&mut self) {
        self.report = std::mem::replace(&mut self.report, ResourceState::Empty)
            .error("refresh failed; accepted history retained");
    }

    pub fn worker_unavailable(&mut self) {
        self.report = std::mem::replace(&mut self.report, ResourceState::Empty)
            .error("refresh worker unavailable; accepted history retained");
    }

    fn request_refresh(&mut self, reason: RefreshReason) -> AppEffect {
        self.report = std::mem::replace(&mut self.report, ResourceState::Empty).loading();
        AppEffect::RequestRefresh(reason)
    }

    fn move_handler(&mut self, delta: isize) {
        let Some(report) = self.report.accepted() else {
            return;
        };
        if report.handlers.is_empty() {
            self.selected_handler_key = None;
            return;
        }
        let current = self.selected_handler_index();
        let selected =
            (current as isize + delta).rem_euclid(report.handlers.len() as isize) as usize;
        self.selected_handler_key = Some(report.handlers[selected].handler.key.clone());
    }

    fn repair_handler_selection(&mut self) {
        let Some(report) = self.report.accepted() else {
            self.selected_handler_key = None;
            return;
        };
        let selected_still_exists = self
            .selected_handler_key
            .as_deref()
            .is_some_and(|key| report.handlers.iter().any(|item| item.handler.key == key));
        if !selected_still_exists {
            self.selected_handler_key =
                report.handlers.first().map(|item| item.handler.key.clone());
        }
    }

    pub fn route_is_placeholder(&self) -> bool {
        self.navigation.active() != Route::Overview
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::synthetic_fixture_report;

    #[test]
    fn content_focus_preserves_v01_handler_navigation() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        let initial = app.selected_handler_index();
        app.handle(Command::Down);
        assert_ne!(app.selected_handler_index(), initial);
    }

    #[test]
    fn navigation_focus_uses_typed_future_routes() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.handle(Command::ToggleFocus);
        app.handle(Command::Down);
        app.handle(Command::Enter);
        assert_eq!(app.navigation().active(), Route::Hooks);
        assert!(app.route_is_placeholder());
    }

    #[test]
    fn refresh_changes_state_without_discarding_accepted_report() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        assert_eq!(
            app.handle(Command::Refresh),
            AppEffect::RequestRefresh(RefreshReason::Manual(TimeWindow::Last7Days))
        );
        assert!(app.report_state().is_loading());
        assert!(app.report_state().accepted().is_some());
        app.reject_refresh();
        assert_eq!(
            app.report_state().error_message(),
            Some("refresh failed; accepted history retained")
        );
        assert!(app.report_state().accepted().is_some());
    }
}
