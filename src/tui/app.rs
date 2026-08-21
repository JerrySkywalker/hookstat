//! Terminal-independent Reliability Center state.

use crate::analytics::TimeWindow;
use crate::domain::HookInvocation;
use crate::report::{MachineReport, instrumented_report};

use super::keymap::Command;
use super::navigation::{NavigationState, Route};
use super::state::ResourceState;
#[cfg(test)]
use super::view_model::HookSort;
use super::view_model::{HandlerRef, HookRowViewModel, HooksQuery, ReliabilityCenterViewModel};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Focus {
    Content,
    Navigation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Screen {
    Overview,
    Hooks,
    HookDetail,
    Diagnostics,
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
    view_model: ReliabilityCenterViewModel,
}

impl RefreshSnapshot {
    /// This is intentionally called by the refresh worker, not by rendering or
    /// key handling. It keeps SQLite/receipt reads and aggregation off the UI
    /// event-loop thread, then hands rendering a completed immutable snapshot.
    pub fn from_values(
        values: Vec<HookInvocation>,
        malformed: u64,
        incomplete: u64,
        now: i64,
        window: TimeWindow,
    ) -> Self {
        Self::from_report(instrumented_report(
            &values, now, window, malformed, incomplete,
        ))
    }

    pub fn from_report(report: MachineReport) -> Self {
        Self {
            view_model: ReliabilityCenterViewModel::from_report(report),
        }
    }

    fn into_view_model(self) -> ReliabilityCenterViewModel {
        self.view_model
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
    screen: Screen,
    requested_window: TimeWindow,
    selected_handler: Option<HandlerRef>,
    view: ResourceState<ReliabilityCenterViewModel>,
    hooks_query: HooksQuery,
    visible_hooks: Vec<HookRowViewModel>,
    search_editing: bool,
}

impl App {
    #[cfg(test)]
    pub fn from_report(report: MachineReport) -> Self {
        Self::from_view_model(ReliabilityCenterViewModel::from_report(report))
    }

    pub fn from_snapshot(snapshot: RefreshSnapshot) -> Self {
        Self::from_view_model(snapshot.into_view_model())
    }

    fn from_view_model(view_model: ReliabilityCenterViewModel) -> Self {
        let hooks_query = HooksQuery::default();
        let visible_hooks = view_model.filtered_hooks(&hooks_query);
        let selected_handler = view_model
            .overview
            .highest_risk_hooks
            .first()
            .or_else(|| visible_hooks.first())
            .map(|row| row.internal_ref.clone());
        Self {
            navigation: NavigationState::new(),
            focus: Focus::Content,
            screen: Screen::Overview,
            requested_window: view_model.overview.window,
            selected_handler,
            view: ResourceState::Ready(view_model),
            hooks_query,
            visible_hooks,
            search_editing: false,
        }
    }

    pub const fn navigation(&self) -> NavigationState {
        self.navigation
    }

    pub const fn focus(&self) -> Focus {
        self.focus
    }

    pub const fn screen(&self) -> Screen {
        self.screen
    }

    pub const fn view_state(&self) -> &ResourceState<ReliabilityCenterViewModel> {
        &self.view
    }

    pub fn view_model(&self) -> Option<&ReliabilityCenterViewModel> {
        self.view.accepted()
    }

    pub fn selected_handler(&self) -> Option<&HandlerRef> {
        self.selected_handler.as_ref()
    }

    pub const fn hooks_query(&self) -> &HooksQuery {
        &self.hooks_query
    }

    pub fn visible_hooks(&self) -> &[HookRowViewModel] {
        &self.visible_hooks
    }

    pub const fn is_search_editing(&self) -> bool {
        self.search_editing
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
                    self.move_content(-1);
                }
                AppEffect::None
            }
            Command::Down => {
                if self.focus == Focus::Navigation {
                    self.navigation.move_by(1);
                } else {
                    self.move_content(1);
                }
                AppEffect::None
            }
            Command::Enter => {
                if self.search_editing {
                    self.search_editing = false;
                } else if self.focus == Focus::Navigation {
                    self.navigation.activate_selected();
                    self.screen = match self.navigation.active() {
                        Route::Overview => Screen::Overview,
                        Route::Hooks => Screen::Hooks,
                        Route::Diagnostics => Screen::Diagnostics,
                    };
                    self.repair_handler_selection();
                } else if matches!(self.screen, Screen::Overview | Screen::Hooks)
                    && self.selected_handler.is_some()
                {
                    self.navigation.activate(Route::Hooks);
                    self.screen = Screen::HookDetail;
                }
                AppEffect::None
            }
            Command::Back => {
                if self.search_editing {
                    self.search_editing = false;
                } else if self.screen == Screen::HookDetail {
                    self.navigation.activate(Route::Hooks);
                    self.screen = Screen::Hooks;
                    self.repair_handler_selection();
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
            Command::Search => {
                if self.screen == Screen::Hooks {
                    self.search_editing = true;
                }
                AppEffect::None
            }
            Command::SearchInput(value) => {
                if self.search_editing {
                    self.hooks_query.search.push(value);
                    self.rebuild_visible_hooks();
                    self.repair_handler_selection();
                }
                AppEffect::None
            }
            Command::SearchBackspace => {
                if self.search_editing {
                    self.hooks_query.search.pop();
                    self.rebuild_visible_hooks();
                    self.repair_handler_selection();
                }
                AppEffect::None
            }
            Command::CloseSearch => {
                self.search_editing = false;
                AppEffect::None
            }
            Command::Filter => {
                if self.screen == Screen::Hooks {
                    self.hooks_query.failures_only = !self.hooks_query.failures_only;
                    self.rebuild_visible_hooks();
                    self.repair_handler_selection();
                }
                AppEffect::None
            }
            Command::Sort => {
                if self.screen == Screen::Hooks {
                    self.hooks_query.sort = self.hooks_query.sort.next();
                    self.rebuild_visible_hooks();
                    self.repair_handler_selection();
                }
                AppEffect::None
            }
        }
    }

    pub fn apply_refresh(&mut self, snapshot: RefreshSnapshot) {
        self.requested_window = snapshot.view_model.overview.window;
        self.view =
            std::mem::replace(&mut self.view, ResourceState::Empty).ready(snapshot.view_model);
        self.rebuild_visible_hooks();
        self.repair_handler_selection();
    }

    pub fn reject_refresh(&mut self) {
        self.view = std::mem::replace(&mut self.view, ResourceState::Empty).error("refresh_failed");
    }

    pub fn worker_unavailable(&mut self) {
        self.view =
            std::mem::replace(&mut self.view, ResourceState::Empty).error("worker_unavailable");
    }

    fn request_refresh(&mut self, reason: RefreshReason) -> AppEffect {
        self.view = std::mem::replace(&mut self.view, ResourceState::Empty).loading();
        AppEffect::RequestRefresh(reason)
    }

    fn move_content(&mut self, delta: isize) {
        if self.screen == Screen::Diagnostics {
            return;
        }
        let candidates = self
            .selectable_rows()
            .into_iter()
            .map(|row| row.internal_ref)
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            self.selected_handler = None;
            return;
        }
        let current = self
            .selected_handler
            .as_ref()
            .and_then(|selected| {
                candidates
                    .iter()
                    .position(|candidate| candidate == selected)
            })
            .unwrap_or(0);
        let next = (current as isize + delta).rem_euclid(candidates.len() as isize) as usize;
        self.selected_handler = Some(candidates[next].clone());
    }

    fn rebuild_visible_hooks(&mut self) {
        self.visible_hooks = self
            .view
            .accepted()
            .map(|view| view.filtered_hooks(&self.hooks_query))
            .unwrap_or_default();
    }

    fn repair_handler_selection(&mut self) {
        let candidates = self
            .selectable_rows()
            .into_iter()
            .map(|row| row.internal_ref)
            .collect::<Vec<_>>();
        let preserved = self
            .selected_handler
            .as_ref()
            .is_some_and(|selected| candidates.iter().any(|candidate| candidate == selected));
        if !preserved {
            self.selected_handler = candidates.into_iter().next();
        }
    }

    fn selectable_rows(&self) -> Vec<HookRowViewModel> {
        match self.screen {
            Screen::Overview => self
                .view
                .accepted()
                .map(|view| view.overview.highest_risk_hooks.clone())
                .unwrap_or_default(),
            Screen::Hooks | Screen::HookDetail => self.visible_hooks.clone(),
            Screen::Diagnostics => Vec::new(),
        }
    }

    #[cfg(test)]
    pub const fn hook_sort(&self) -> HookSort {
        self.hooks_query.sort
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::synthetic_fixture_report;

    #[test]
    fn hooks_selection_uses_stable_identity_across_refresh() {
        let report = synthetic_fixture_report(1_000);
        let mut app = App::from_report(report.clone());
        app.handle(Command::ToggleFocus);
        app.handle(Command::Down);
        app.handle(Command::Enter);
        app.handle(Command::ToggleFocus);
        app.handle(Command::Down);
        let selected = app.selected_handler().cloned();
        app.apply_refresh(RefreshSnapshot::from_report(report));
        assert_eq!(app.selected_handler(), selected.as_ref());
    }

    #[test]
    fn hooks_search_filter_and_sort_are_ui_state_not_snapshot_mutation() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.handle(Command::ToggleFocus);
        app.handle(Command::Down);
        app.handle(Command::Enter);
        app.handle(Command::ToggleFocus);
        app.handle(Command::Search);
        for value in ['a', 'l', 'p', 'h', 'a'] {
            app.handle(Command::SearchInput(value));
        }
        assert_eq!(app.visible_hooks().len(), 1);
        app.handle(Command::CloseSearch);
        app.handle(Command::Filter);
        assert_eq!(app.visible_hooks().len(), 1);
        let previous = app.hook_sort();
        app.handle(Command::Sort);
        assert_ne!(app.hook_sort(), previous);
    }

    #[test]
    fn refresh_error_preserves_accepted_history() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        assert!(matches!(
            app.handle(Command::Refresh),
            AppEffect::RequestRefresh(_)
        ));
        assert!(app.view_state().is_loading());
        app.reject_refresh();
        assert!(app.view_model().is_some());
    }

    #[test]
    fn navigation_opens_the_read_only_diagnostics_route() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.handle(Command::ToggleFocus);
        app.handle(Command::Down);
        app.handle(Command::Down);
        app.handle(Command::Enter);
        assert_eq!(app.navigation().active(), Route::Diagnostics);
        assert_eq!(app.screen(), Screen::Diagnostics);
        assert!(app.view_model().is_some());
    }
}
