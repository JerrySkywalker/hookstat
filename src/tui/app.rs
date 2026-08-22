//! Terminal-independent Reliability Center state.

use crate::analytics::TimeWindow;
use crate::diagnostics::DiagnosticsReport;
use crate::domain::HookInvocation;
use crate::interface_preferences::InterfaceColor;
use crate::report::{MachineReport, instrumented_report};
use jerry_terminal_ui::interaction::{
    DiscardDecision, OverlayDismissKey, OverlayState, QuitDisposition, SettingsEditor,
};

use super::keymap::Command;
use super::localization::InterfaceLanguage;
use super::navigation::{NavigationState, Route};
use super::state::ResourceState;
#[cfg(test)]
use super::view_model::HookSort;
use super::view_model::{HandlerRef, HookRowViewModel, HooksQuery, ReliabilityCenterViewModel};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Screen {
    Overview,
    Hooks,
    HookDetail,
    Diagnostics,
    Settings,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LocalMode {
    None,
    HooksList,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticsRefreshReason {
    Initial,
    Explicit,
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

    pub fn from_report_with_diagnostics(
        report: MachineReport,
        diagnostics: DiagnosticsReport,
    ) -> Self {
        let mut view_model = ReliabilityCenterViewModel::from_report(report);
        view_model.diagnostics = diagnostics;
        Self { view_model }
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
    RequestDiagnostics(DiagnosticsRefreshReason),
    ApplyInterface {
        language: InterfaceLanguage,
        color: InterfaceColor,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsSaveState {
    Clean,
    Dirty,
    Saved,
    Conflict,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsField {
    Language,
    Color,
}

impl SettingsField {
    const ALL: [Self; 2] = [Self::Language, Self::Color];
}

#[derive(Clone, Debug)]
pub struct App {
    navigation: NavigationState,
    screen: Screen,
    local_mode: LocalMode,
    help_overlay: OverlayState,
    requested_window: TimeWindow,
    selected_handler: Option<HandlerRef>,
    view: ResourceState<ReliabilityCenterViewModel>,
    diagnostics: ResourceState<DiagnosticsReport>,
    hooks_query: HooksQuery,
    visible_hooks: Vec<HookRowViewModel>,
    search_editing: bool,
    detail_scroll_lines: u16,
    accepted_language: InterfaceLanguage,
    draft_language: InterfaceLanguage,
    accepted_color: InterfaceColor,
    draft_color: InterfaceColor,
    settings_editor: SettingsEditor<SettingsField>,
    settings_save_state: SettingsSaveState,
}

impl App {
    #[cfg(test)]
    pub fn from_report(report: MachineReport) -> Self {
        Self::from_view_model(ReliabilityCenterViewModel::from_report(report))
    }

    pub fn from_snapshot(snapshot: RefreshSnapshot) -> Self {
        Self::from_view_model(snapshot.into_view_model())
    }

    /// Creates an interactive shell before any receipt, SQLite, diagnostics,
    /// or analytics work has completed.
    pub fn loading(window: TimeWindow) -> Self {
        Self {
            navigation: NavigationState::new(),
            screen: Screen::Overview,
            local_mode: LocalMode::None,
            help_overlay: OverlayState::None,
            requested_window: window,
            selected_handler: None,
            view: ResourceState::Loading {
                last_accepted: None,
            },
            diagnostics: ResourceState::Loading {
                last_accepted: None,
            },
            hooks_query: HooksQuery::default(),
            visible_hooks: Vec::new(),
            search_editing: false,
            detail_scroll_lines: 0,
            accepted_language: InterfaceLanguage::Auto,
            draft_language: InterfaceLanguage::Auto,
            accepted_color: InterfaceColor::Auto,
            draft_color: InterfaceColor::Auto,
            settings_editor: SettingsEditor::new(SettingsField::Language),
            settings_save_state: SettingsSaveState::Clean,
        }
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
        let diagnostics = view_model.diagnostics.clone();
        Self {
            navigation: NavigationState::new(),
            screen: Screen::Overview,
            local_mode: LocalMode::None,
            help_overlay: OverlayState::None,
            requested_window: view_model.overview.window,
            selected_handler,
            view: ResourceState::Ready(view_model),
            diagnostics: ResourceState::Ready(diagnostics),
            hooks_query,
            visible_hooks,
            search_editing: false,
            detail_scroll_lines: 0,
            accepted_language: InterfaceLanguage::Auto,
            draft_language: InterfaceLanguage::Auto,
            accepted_color: InterfaceColor::Auto,
            draft_color: InterfaceColor::Auto,
            settings_editor: SettingsEditor::new(SettingsField::Language),
            settings_save_state: SettingsSaveState::Clean,
        }
    }

    pub const fn navigation(&self) -> NavigationState {
        self.navigation
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

    pub const fn requested_window(&self) -> TimeWindow {
        self.requested_window
    }

    pub const fn diagnostics_state(&self) -> &ResourceState<DiagnosticsReport> {
        &self.diagnostics
    }

    pub fn diagnostics(&self) -> Option<&DiagnosticsReport> {
        self.diagnostics.accepted()
    }

    pub fn selected_handler(&self) -> Option<&HandlerRef> {
        self.selected_handler.as_ref()
    }

    pub const fn detail_scroll_lines(&self) -> u16 {
        self.detail_scroll_lines
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

    pub const fn accepted_language(&self) -> InterfaceLanguage {
        self.accepted_language
    }

    pub const fn draft_language(&self) -> InterfaceLanguage {
        self.draft_language
    }

    pub const fn accepted_color(&self) -> InterfaceColor {
        self.accepted_color
    }

    pub const fn draft_color(&self) -> InterfaceColor {
        self.draft_color
    }

    pub const fn settings_field(&self) -> SettingsField {
        self.settings_editor.selected_field()
    }

    pub const fn settings_editing(&self) -> bool {
        self.settings_editor.is_editing()
    }

    pub const fn help_open(&self) -> bool {
        self.help_overlay.is_open()
    }

    pub const fn discard_confirmation_open(&self) -> bool {
        self.settings_editor.awaiting_discard_confirmation()
    }

    pub const fn local_list_active(&self) -> bool {
        matches!(self.local_mode, LocalMode::HooksList)
    }

    const fn hooks_list_active(&self) -> bool {
        self.local_list_active()
    }

    pub const fn settings_save_state(&self) -> SettingsSaveState {
        self.settings_save_state
    }

    pub fn settings_dirty(&self) -> bool {
        self.draft_language != self.accepted_language || self.draft_color != self.accepted_color
    }

    pub fn set_persisted_interface(&mut self, language: InterfaceLanguage, color: InterfaceColor) {
        self.accepted_language = language;
        self.draft_language = language;
        self.accepted_color = color;
        self.draft_color = color;
        self.settings_save_state = SettingsSaveState::Clean;
    }

    pub fn language_saved(&mut self) {
        self.accepted_language = self.draft_language;
        self.accepted_color = self.draft_color;
        self.settings_save_state = SettingsSaveState::Saved;
    }

    pub fn language_save_conflict(&mut self) {
        self.settings_save_state = SettingsSaveState::Conflict;
    }

    pub fn language_save_failed(&mut self) {
        self.settings_save_state = SettingsSaveState::Failed;
    }

    pub fn handle(&mut self, command: Command) -> AppEffect {
        if self.help_overlay.is_open() {
            return self.handle_help_overlay(command);
        }
        if self.settings_editor.awaiting_discard_confirmation() {
            return self.handle_discard_confirmation(command);
        }
        match command {
            Command::Quit => self.request_quit(),
            Command::Help => {
                self.help_overlay.open_help();
                AppEffect::None
            }
            Command::Discard => AppEffect::None,
            Command::Up => {
                self.move_direction(-1);
                AppEffect::None
            }
            Command::Down => {
                self.move_direction(1);
                AppEffect::None
            }
            Command::PageUp => {
                self.page_content(-1);
                AppEffect::None
            }
            Command::PageDown => {
                self.page_content(1);
                AppEffect::None
            }
            Command::Enter => {
                if self.search_editing {
                    self.search_editing = false;
                } else if self.screen == Screen::Settings {
                    self.settings_editor.enter_or_finish();
                } else if self.screen == Screen::Hooks && !self.hooks_list_active() {
                    self.local_mode = LocalMode::HooksList;
                    self.repair_handler_selection();
                } else if matches!(self.screen, Screen::Overview | Screen::Hooks)
                    && self.selected_handler.is_some()
                {
                    self.navigation.activate(Route::Hooks);
                    self.screen = Screen::HookDetail;
                    self.local_mode = LocalMode::HooksList;
                    self.detail_scroll_lines = 0;
                }
                AppEffect::None
            }
            Command::Back => {
                if self.search_editing {
                    self.search_editing = false;
                } else if self.screen == Screen::HookDetail {
                    self.navigation.activate(Route::Hooks);
                    self.screen = Screen::Hooks;
                    self.local_mode = LocalMode::HooksList;
                    self.detail_scroll_lines = 0;
                    self.repair_handler_selection();
                } else if self.screen == Screen::Hooks && self.hooks_list_active() {
                    self.local_mode = LocalMode::None;
                } else if self.screen == Screen::Settings && self.settings_editor.is_editing() {
                    self.settings_editor.enter_or_finish();
                }
                AppEffect::None
            }
            Command::Refresh
                if self.screen == Screen::Settings && self.settings_editor.is_editing() =>
            {
                self.revert_settings();
                AppEffect::None
            }
            Command::Refresh if self.screen == Screen::Diagnostics => {
                self.request_diagnostics(DiagnosticsRefreshReason::Explicit)
            }
            Command::Refresh => self.request_refresh(RefreshReason::Manual(self.requested_window)),
            Command::Window(window) => {
                if self.screen == Screen::Settings && self.settings_editor.is_editing() {
                    return self.apply_interface();
                }
                self.requested_window = window;
                self.request_refresh(RefreshReason::Window(window))
            }
            Command::Search => {
                if self.screen == Screen::Hooks && self.hooks_list_active() {
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
                if self.screen == Screen::Hooks && self.hooks_list_active() {
                    self.hooks_query.failures_only = !self.hooks_query.failures_only;
                    self.rebuild_visible_hooks();
                    self.repair_handler_selection();
                }
                AppEffect::None
            }
            Command::Sort => {
                if self.screen == Screen::Hooks && self.hooks_list_active() {
                    self.hooks_query.sort = self.hooks_query.sort.next();
                    self.rebuild_visible_hooks();
                    self.repair_handler_selection();
                }
                AppEffect::None
            }
            Command::PreviousSetting => {
                if self.screen == Screen::Settings && self.settings_editor.is_editing() {
                    self.cycle_current_setting(-1);
                }
                AppEffect::None
            }
            Command::NextSetting => {
                if self.screen == Screen::Settings && self.settings_editor.is_editing() {
                    self.cycle_current_setting(1);
                }
                AppEffect::None
            }
        }
    }

    pub fn apply_refresh(&mut self, snapshot: RefreshSnapshot) {
        if snapshot.view_model.overview.window != self.requested_window {
            // Belt-and-braces request ownership: a worker response for an old
            // period cannot overwrite a newer visible request even if a future
            // transport implementation becomes concurrent.
            return;
        }
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

    pub fn apply_diagnostics(&mut self, diagnostics: DiagnosticsReport) {
        self.diagnostics =
            std::mem::replace(&mut self.diagnostics, ResourceState::Empty).ready(diagnostics);
    }

    pub fn reject_diagnostics(&mut self) {
        self.diagnostics = std::mem::replace(&mut self.diagnostics, ResourceState::Empty)
            .error("diagnostics_refresh_failed");
    }

    fn handle_help_overlay(&mut self, command: Command) -> AppEffect {
        let dismissal = match command {
            Command::Back => Some(OverlayDismissKey::Escape),
            Command::Help => Some(OverlayDismissKey::Help),
            Command::Quit => Some(OverlayDismissKey::Quit),
            _ => None,
        };
        if let Some(dismissal) = dismissal {
            let _ = self.help_overlay.dismiss_with(dismissal);
        }
        AppEffect::None
    }

    fn handle_discard_confirmation(&mut self, command: Command) -> AppEffect {
        match command {
            Command::Back | Command::Quit => {
                let _ = self
                    .settings_editor
                    .resolve_discard(DiscardDecision::Cancel);
                AppEffect::None
            }
            Command::Enter | Command::Discard => {
                if self
                    .settings_editor
                    .resolve_discard(DiscardDecision::Discard)
                {
                    self.revert_settings();
                    AppEffect::Quit
                } else {
                    AppEffect::None
                }
            }
            _ => AppEffect::None,
        }
    }

    fn request_quit(&mut self) -> AppEffect {
        match self.settings_editor.request_quit(self.settings_dirty()) {
            QuitDisposition::Quit => AppEffect::Quit,
            QuitDisposition::ConfirmDiscard => AppEffect::None,
        }
    }

    fn revert_settings(&mut self) {
        self.draft_language = self.accepted_language;
        self.draft_color = self.accepted_color;
        self.settings_save_state = SettingsSaveState::Clean;
    }

    fn move_direction(&mut self, delta: isize) {
        if self.screen == Screen::HookDetail {
            self.move_detail_scroll(delta);
        } else if self.screen == Screen::Settings && self.settings_editor.is_editing() {
            let _ = self.settings_editor.move_field(&SettingsField::ALL, delta);
        } else if self.screen == Screen::Hooks && self.hooks_list_active() {
            self.move_content(delta);
        } else {
            self.navigation.move_by(delta);
            self.screen = match self.navigation.current() {
                Route::Overview => Screen::Overview,
                Route::Hooks => Screen::Hooks,
                Route::Diagnostics => Screen::Diagnostics,
                Route::Settings => Screen::Settings,
            };
            self.local_mode = LocalMode::None;
            self.repair_handler_selection();
        }
    }

    fn request_refresh(&mut self, reason: RefreshReason) -> AppEffect {
        self.view = std::mem::replace(&mut self.view, ResourceState::Empty).loading();
        AppEffect::RequestRefresh(reason)
    }

    fn request_diagnostics(&mut self, reason: DiagnosticsRefreshReason) -> AppEffect {
        // Diagnostics are an independently owned, read-only resource. A
        // refresh keeps the last accepted report visible until its replacement
        // arrives, just as reliability refreshes retain their accepted view.
        self.diagnostics = std::mem::replace(&mut self.diagnostics, ResourceState::Empty).loading();
        AppEffect::RequestDiagnostics(reason)
    }

    fn cycle_current_setting(&mut self, delta: isize) {
        match self.settings_editor.selected_field() {
            SettingsField::Language => self.cycle_language(delta),
            SettingsField::Color => self.cycle_color(delta),
        }
    }

    fn cycle_language(&mut self, delta: isize) {
        const ALL: [InterfaceLanguage; 3] = [
            InterfaceLanguage::Auto,
            InterfaceLanguage::EnUs,
            InterfaceLanguage::ZhCn,
        ];
        let current = ALL
            .iter()
            .position(|language| *language == self.draft_language)
            .unwrap_or_default();
        let next = (current as isize + delta).rem_euclid(ALL.len() as isize) as usize;
        self.draft_language = ALL[next];
        self.settings_save_state = if self.settings_dirty() {
            SettingsSaveState::Dirty
        } else {
            SettingsSaveState::Clean
        };
    }

    fn cycle_color(&mut self, delta: isize) {
        const ALL: [InterfaceColor; 3] = [
            InterfaceColor::Auto,
            InterfaceColor::Always,
            InterfaceColor::Never,
        ];
        let current = ALL
            .iter()
            .position(|color| *color == self.draft_color)
            .unwrap_or_default();
        let next = (current as isize + delta).rem_euclid(ALL.len() as isize) as usize;
        self.draft_color = ALL[next];
        self.settings_save_state = if self.settings_dirty() {
            SettingsSaveState::Dirty
        } else {
            SettingsSaveState::Clean
        };
    }

    fn apply_interface(&mut self) -> AppEffect {
        if self.settings_dirty() {
            AppEffect::ApplyInterface {
                language: self.draft_language,
                color: self.draft_color,
            }
        } else {
            AppEffect::None
        }
    }

    fn move_content(&mut self, delta: isize) {
        if self.screen == Screen::HookDetail {
            self.move_detail_scroll(delta);
            return;
        }
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

    fn page_content(&mut self, direction: isize) {
        if self.screen == Screen::HookDetail {
            self.move_detail_scroll(direction.saturating_mul(6));
        } else if self.screen == Screen::Hooks && self.hooks_list_active() {
            self.move_content(direction.saturating_mul(5));
        }
    }

    fn move_detail_scroll(&mut self, delta: isize) {
        self.detail_scroll_lines = if delta.is_negative() {
            self.detail_scroll_lines
                .saturating_sub(delta.unsigned_abs() as u16)
        } else {
            self.detail_scroll_lines.saturating_add(delta as u16)
        };
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
            Screen::Settings => Vec::new(),
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
        app.handle(Command::Down);
        app.handle(Command::Enter);
        app.handle(Command::Down);
        let selected = app.selected_handler().cloned();
        app.apply_refresh(RefreshSnapshot::from_report(report));
        assert_eq!(app.selected_handler(), selected.as_ref());
    }

    #[test]
    fn hooks_search_filter_and_sort_are_ui_state_not_snapshot_mutation() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.handle(Command::Down);
        app.handle(Command::Enter);
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
    fn loading_shell_accepts_input_before_initial_data_arrives() {
        let mut app = App::loading(TimeWindow::Last7Days);
        assert!(app.view_model().is_none());
        assert_eq!(app.requested_window(), TimeWindow::Last7Days);
        assert_eq!(
            app.handle(Command::Window(TimeWindow::Today)),
            AppEffect::RequestRefresh(RefreshReason::Window(TimeWindow::Today))
        );
        assert_eq!(app.requested_window(), TimeWindow::Today);
        assert_eq!(app.handle(Command::Quit), AppEffect::Quit);
    }

    #[test]
    fn latest_requested_period_rejects_out_of_order_snapshot_completion() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.handle(Command::Window(TimeWindow::Last7Days));
        app.handle(Command::Window(TimeWindow::Last30Days));
        app.handle(Command::Window(TimeWindow::Today));

        let mut stale = synthetic_fixture_report(1_000);
        stale.window = TimeWindow::Last30Days;
        app.apply_refresh(RefreshSnapshot::from_report(stale));
        assert_eq!(app.requested_window(), TimeWindow::Today);
        assert_eq!(
            app.view_model().unwrap().overview.window,
            TimeWindow::Last7Days
        );

        let mut newest = synthetic_fixture_report(1_000);
        newest.window = TimeWindow::Today;
        app.apply_refresh(RefreshSnapshot::from_report(newest));
        assert_eq!(app.view_model().unwrap().overview.window, TimeWindow::Today);
    }

    #[test]
    fn period_switches_only_request_reliability_and_leave_diagnostics_independent() {
        let mut app = App::loading(TimeWindow::Last7Days);
        for window in [
            TimeWindow::Last7Days,
            TimeWindow::Last30Days,
            TimeWindow::Today,
        ] {
            assert_eq!(
                app.handle(Command::Window(window)),
                AppEffect::RequestRefresh(RefreshReason::Window(window))
            );
        }
        assert!(matches!(
            app.diagnostics_state(),
            ResourceState::Loading { .. }
        ));

        app.screen = Screen::Diagnostics;
        assert_eq!(
            app.handle(Command::Refresh),
            AppEffect::RequestDiagnostics(DiagnosticsRefreshReason::Explicit)
        );
    }

    #[test]
    fn cross_resource_failures_preserve_the_other_accepted_snapshot() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.apply_diagnostics(DiagnosticsReport::empty(1_000));
        app.reject_refresh();
        assert!(app.diagnostics().is_some());

        let accepted_window = app.view_model().unwrap().overview.window;
        app.reject_diagnostics();
        assert_eq!(app.view_model().unwrap().overview.window, accepted_window);
        assert!(matches!(
            app.diagnostics_state(),
            ResourceState::Error { .. }
        ));
    }

    #[test]
    fn navigation_opens_the_read_only_diagnostics_route() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.handle(Command::Down);
        app.handle(Command::Down);
        assert_eq!(app.navigation().current(), Route::Diagnostics);
        assert_eq!(app.screen(), Screen::Diagnostics);
        assert!(app.view_model().is_some());
    }

    #[test]
    fn language_switch_is_staged_without_losing_hooks_state() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.handle(Command::Down);
        app.handle(Command::Enter);
        app.handle(Command::Search);
        app.handle(Command::SearchInput('a'));
        app.handle(Command::CloseSearch);
        let search = app.hooks_query().search.clone();
        let selection = app.selected_handler().cloned();
        app.navigation.activate(Route::Settings);
        app.screen = Screen::Settings;
        app.handle(Command::Enter);
        app.handle(Command::NextSetting);
        assert!(app.settings_dirty());
        assert_eq!(app.hooks_query().search, search);
        assert_eq!(app.selected_handler(), selection.as_ref());
        assert_eq!(
            app.handle(Command::Window(TimeWindow::All)),
            AppEffect::ApplyInterface {
                language: InterfaceLanguage::EnUs,
                color: InterfaceColor::Auto,
            }
        );
        app.language_saved();
        assert!(!app.settings_dirty());
    }

    #[test]
    fn color_policy_is_staged_and_applied_without_touching_hook_state() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.navigation.activate(Route::Settings);
        app.screen = Screen::Settings;
        app.settings_editor = SettingsEditor::new(SettingsField::Color);
        app.handle(Command::Enter);
        app.handle(Command::NextSetting);
        assert_eq!(app.draft_color(), InterfaceColor::Always);
        assert_eq!(
            app.handle(Command::Window(TimeWindow::All)),
            AppEffect::ApplyInterface {
                language: InterfaceLanguage::Auto,
                color: InterfaceColor::Always,
            }
        );
    }

    #[test]
    fn detail_scrolling_is_bounded_and_uses_press_commands() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.handle(Command::Enter);
        assert_eq!(app.screen(), Screen::HookDetail);
        app.handle(Command::PageDown);
        assert_eq!(app.detail_scroll_lines(), 6);
        app.handle(Command::Up);
        assert_eq!(app.detail_scroll_lines(), 5);
        app.handle(Command::PageUp);
        assert_eq!(app.detail_scroll_lines(), 0);
    }

    #[test]
    fn top_level_navigation_is_direct_without_global_focus_or_active_split() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.handle(Command::Down);
        assert_eq!(app.screen(), Screen::Hooks);
        assert_eq!(app.navigation().current(), Route::Hooks);
        assert!(!app.local_list_active());
        app.handle(Command::Up);
        assert_eq!(app.screen(), Screen::Overview);
        assert_eq!(app.navigation().current(), Route::Overview);
    }

    #[test]
    fn settings_require_explicit_edit_and_guard_dirty_quit() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.navigation.activate(Route::Settings);
        app.screen = Screen::Settings;
        app.handle(Command::NextSetting);
        assert!(!app.settings_dirty());
        app.handle(Command::Enter);
        assert!(app.settings_editing());
        app.handle(Command::NextSetting);
        assert!(app.settings_dirty());
        assert_eq!(app.handle(Command::Quit), AppEffect::None);
        assert!(app.discard_confirmation_open());
        app.handle(Command::Back);
        assert!(!app.discard_confirmation_open());
        assert!(app.settings_dirty());
        app.handle(Command::Quit);
        assert_eq!(app.handle(Command::Discard), AppEffect::Quit);
        assert!(!app.settings_dirty());
    }

    #[test]
    fn help_overlay_owns_normal_keys_until_dismissed() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.handle(Command::Help);
        assert!(app.help_open());
        app.handle(Command::Down);
        assert_eq!(app.screen(), Screen::Overview);
        app.handle(Command::Quit);
        assert!(!app.help_open());
        assert_eq!(app.screen(), Screen::Overview);
    }
}
