//! Terminal-independent Reliability Center state.

use crate::analytics::TimeWindow;
use crate::diagnostics::DiagnosticsReport;
use crate::domain::{HookInvocation, Runtime};
use crate::interface_preferences::InterfaceColor;
use crate::report::{MachineReport, instrumented_report};
use crate::runtime_presentation::{
    HistoricalHandlerIdentity, ReliabilityHistory, ReliabilityJoinState, RuntimeCatalogResource,
    RuntimeCatalogResourceState, RuntimeEventPresentation, RuntimeHandlerPresentation,
    RuntimePresentationSnapshot,
};
use crate::workbench::{ChangesWorkbench, changes_workbench};
use terminal_ui_contract::interaction::{
    DiscardDecision, OverlayDismissKey, OverlayState, QuitDisposition, SettingsEditor,
};

use super::keymap::Command;
use super::localization::InterfaceLanguage;
use super::navigation::{NavigationState, Route};
use super::state::ResourceState;
use super::view_model::{
    CatalogHistoryViewModel, ChangeRef, ChangeRowViewModel, ChangesViewModel, DisplayIdentity,
    FailureClusterRef, FailureClusterViewModel, HandlerRef, HookRowViewModel, HooksQuery,
    ReliabilityCenterViewModel,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Screen {
    Overview,
    Hooks,
    Changes,
    ChangeDetail,
    HookDetail,
    FailureClusters,
    FailureClusterDetail,
    Diagnostics,
    Settings,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LocalMode {
    None,
    HooksEvents,
    HooksHandlers,
    ChangesList,
    FailureClusters,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangesRefreshReason {
    Entered(TimeWindow),
    Window(TimeWindow),
    Explicit(TimeWindow),
}

/// Runtime catalog discovery is deliberately independent from reliability
/// period loading. The catalog carries local-only raw presentation material and
/// never enters a ledger, receipt, or diagnostic snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeCatalogRefreshReason {
    Initial,
    Explicit,
}

impl ChangesRefreshReason {
    pub const fn window(self) -> TimeWindow {
        match self {
            Self::Entered(window) | Self::Window(window) | Self::Explicit(window) => window,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RefreshSnapshot {
    view_model: ReliabilityCenterViewModel,
    alias_annotations: Vec<AliasAnnotation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AliasAnnotation {
    pub runtime: Runtime,
    pub handler_key: String,
    pub display_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AliasApplyRequest {
    pub runtime: Runtime,
    pub handler_key: String,
    pub draft: String,
    pub expected_alias: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AliasApplyOutcome {
    Saved,
    Conflict,
    Failed,
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
            alias_annotations: Vec::new(),
        }
    }

    pub fn from_report_with_aliases(
        report: MachineReport,
        alias_annotations: Vec<AliasAnnotation>,
    ) -> Self {
        Self {
            view_model: ReliabilityCenterViewModel::from_report(report),
            alias_annotations,
        }
    }

    pub fn from_report_with_diagnostics(
        report: MachineReport,
        diagnostics: DiagnosticsReport,
    ) -> Self {
        let mut view_model = ReliabilityCenterViewModel::from_report(report);
        view_model.diagnostics = diagnostics;
        Self {
            view_model,
            alias_annotations: Vec::new(),
        }
    }

    fn into_parts(self) -> (ReliabilityCenterViewModel, Vec<AliasAnnotation>) {
        (self.view_model, self.alias_annotations)
    }
}

#[derive(Clone, Debug)]
pub struct ChangesSnapshot {
    view_model: ChangesViewModel,
}

impl ChangesSnapshot {
    pub fn from_values(
        values: Vec<HookInvocation>,
        generated_at_unix_ms: i64,
        window: TimeWindow,
        coverage: crate::domain::EvidenceCoverage,
    ) -> Self {
        Self::from_workbench(changes_workbench(
            &values,
            generated_at_unix_ms,
            window,
            coverage,
        ))
    }

    pub fn from_workbench(workbench: ChangesWorkbench) -> Self {
        Self {
            view_model: ChangesViewModel::from_workbench(workbench),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppEffect {
    None,
    Quit,
    RequestRefresh(RefreshReason),
    RequestDiagnostics(DiagnosticsRefreshReason),
    RequestChanges(ChangesRefreshReason),
    RequestRefreshAndChanges(RefreshReason, ChangesRefreshReason),
    RequestRefreshAndRuntimeCatalog(RefreshReason, RuntimeCatalogRefreshReason),
    ApplyAlias(AliasApplyRequest),
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AliasField {
    Name,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AliasSaveState {
    Clean,
    Dirty,
    Saved,
    Conflict,
    Failed,
}

impl SettingsField {
    const ALL: [Self; 2] = [Self::Language, Self::Color];
}

#[derive(Clone)]
pub struct App {
    navigation: NavigationState,
    screen: Screen,
    local_mode: LocalMode,
    help_overlay: OverlayState,
    requested_window: TimeWindow,
    selected_handler: Option<HandlerRef>,
    selected_change: Option<ChangeRef>,
    selected_failure_cluster: Option<FailureClusterRef>,
    selected_runtime_event: Option<(String, String)>,
    selected_runtime_handler: Option<String>,
    view: ResourceState<ReliabilityCenterViewModel>,
    runtime_catalog: RuntimeCatalogResource,
    diagnostics: ResourceState<DiagnosticsReport>,
    changes: ResourceState<ChangesViewModel>,
    hooks_query: HooksQuery,
    visible_hooks: Vec<HookRowViewModel>,
    visible_changes: Vec<ChangeRowViewModel>,
    search_editing: bool,
    alias_text_editing: bool,
    detail_scroll_lines: u16,
    changes_detail_scroll_lines: u16,
    accepted_language: InterfaceLanguage,
    draft_language: InterfaceLanguage,
    accepted_color: InterfaceColor,
    draft_color: InterfaceColor,
    settings_editor: SettingsEditor<SettingsField>,
    settings_save_state: SettingsSaveState,
    alias_annotations: Vec<AliasAnnotation>,
    alias_editor: SettingsEditor<AliasField>,
    alias_handler: Option<HandlerRef>,
    alias_expected: Option<String>,
    alias_base_label: String,
    alias_draft: String,
    alias_save_state: AliasSaveState,
}

impl App {
    #[cfg(test)]
    pub fn from_report(report: MachineReport) -> Self {
        Self::from_view_model(ReliabilityCenterViewModel::from_report(report))
    }

    pub fn from_snapshot(snapshot: RefreshSnapshot) -> Self {
        let (view_model, alias_annotations) = snapshot.into_parts();
        let mut app = Self::from_view_model(view_model);
        app.alias_annotations = alias_annotations;
        app
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
            selected_change: None,
            selected_failure_cluster: None,
            selected_runtime_event: None,
            selected_runtime_handler: None,
            view: ResourceState::Loading {
                last_accepted: None,
            },
            runtime_catalog: {
                let mut catalog = RuntimeCatalogResource::default();
                catalog.request_initial_load();
                catalog
            },
            diagnostics: ResourceState::Loading {
                last_accepted: None,
            },
            changes: ResourceState::Empty,
            hooks_query: HooksQuery::default(),
            visible_hooks: Vec::new(),
            visible_changes: Vec::new(),
            search_editing: false,
            alias_text_editing: false,
            detail_scroll_lines: 0,
            changes_detail_scroll_lines: 0,
            accepted_language: InterfaceLanguage::Auto,
            draft_language: InterfaceLanguage::Auto,
            accepted_color: InterfaceColor::Auto,
            draft_color: InterfaceColor::Auto,
            settings_editor: SettingsEditor::new(SettingsField::Language),
            settings_save_state: SettingsSaveState::Clean,
            alias_annotations: Vec::new(),
            alias_editor: SettingsEditor::new(AliasField::Name),
            alias_handler: None,
            alias_expected: None,
            alias_base_label: String::new(),
            alias_draft: String::new(),
            alias_save_state: AliasSaveState::Clean,
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
            selected_change: None,
            selected_failure_cluster: None,
            selected_runtime_event: None,
            selected_runtime_handler: None,
            view: ResourceState::Ready(view_model),
            runtime_catalog: {
                let mut catalog = RuntimeCatalogResource::default();
                catalog.request_initial_load();
                catalog
            },
            diagnostics: ResourceState::Ready(diagnostics),
            changes: ResourceState::Empty,
            hooks_query,
            visible_hooks,
            visible_changes: Vec::new(),
            search_editing: false,
            alias_text_editing: false,
            detail_scroll_lines: 0,
            changes_detail_scroll_lines: 0,
            accepted_language: InterfaceLanguage::Auto,
            draft_language: InterfaceLanguage::Auto,
            accepted_color: InterfaceColor::Auto,
            draft_color: InterfaceColor::Auto,
            settings_editor: SettingsEditor::new(SettingsField::Language),
            settings_save_state: SettingsSaveState::Clean,
            alias_annotations: Vec::new(),
            alias_editor: SettingsEditor::new(AliasField::Name),
            alias_handler: None,
            alias_expected: None,
            alias_base_label: String::new(),
            alias_draft: String::new(),
            alias_save_state: AliasSaveState::Clean,
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

    #[cfg(test)]
    pub const fn runtime_catalog_state(&self) -> RuntimeCatalogResourceState {
        self.runtime_catalog.state()
    }

    pub fn runtime_catalog(&self) -> Option<&RuntimePresentationSnapshot> {
        self.runtime_catalog.accepted_snapshot()
    }

    pub fn runtime_catalog_loading(&self) -> bool {
        self.runtime_catalog.state() == RuntimeCatalogResourceState::Loading
    }

    pub fn runtime_catalog_error(&self) -> bool {
        self.runtime_catalog.state() == RuntimeCatalogResourceState::Error
    }

    pub fn selected_runtime_event(&self) -> Option<&RuntimeEventPresentation> {
        let (context, name) = self.selected_runtime_event.as_ref()?;
        self.runtime_catalog()?
            .events
            .iter()
            .find(|event| &event.runtime_context == context && &event.runtime_event_name == name)
    }

    pub fn selected_runtime_handler(&self) -> Option<&RuntimeHandlerPresentation> {
        let selected = self.selected_runtime_handler.as_deref()?;
        self.selected_runtime_event()?
            .handlers
            .iter()
            .find(|handler| handler.runtime_catalog_id == selected)
    }

    /// The accepted reliability row for this exact live handler identity.
    /// This is deliberately an optional projection: live handlers with no
    /// admitted, unambiguous bridge never acquire borrowed history or health.
    pub fn runtime_handler_reliability_row(
        &self,
        event: &RuntimeEventPresentation,
        handler: &RuntimeHandlerPresentation,
    ) -> Option<&HookRowViewModel> {
        let ReliabilityJoinState::Matched { handler_key } = self
            .runtime_event_reliability(event)
            .into_iter()
            .find(|(catalog_id, _)| catalog_id == &handler.runtime_catalog_id)
            .map(|(_, join)| join)?
        else {
            return None;
        };
        let canonical_event = event.canonical_event?;
        self.view_model()?
            .hooks
            .rows
            .iter()
            .find(|row| row.internal_ref.handler_key == handler_key && row.event == canonical_event)
    }

    pub fn runtime_event_reliability(
        &self,
        event: &RuntimeEventPresentation,
    ) -> Vec<(String, ReliabilityJoinState)> {
        let Some(snapshot) = self.runtime_catalog() else {
            return Vec::new();
        };
        let history = self.view_model().map(|view| {
            view.hooks
                .rows
                .iter()
                .map(|row| HistoricalHandlerIdentity {
                    handler_key: row.internal_ref.handler_key.clone(),
                    event: row.event,
                })
                .collect::<Vec<_>>()
        });
        let history = history
            .as_deref()
            .map(ReliabilityHistory::Available)
            .unwrap_or(ReliabilityHistory::Unavailable);
        snapshot
            .join_reliability_with_history(history)
            .into_iter()
            .filter(|joined| {
                event
                    .handlers
                    .iter()
                    .any(|handler| handler.runtime_catalog_id == joined.handler.runtime_catalog_id)
            })
            .map(|joined| (joined.handler.runtime_catalog_id.clone(), joined.join))
            .collect()
    }

    pub fn matched_reliability_detail(&self) -> Option<&super::view_model::HookDetailViewModel> {
        let event = self.selected_runtime_event()?;
        let handler = self.selected_runtime_handler()?;
        let reference = self
            .runtime_handler_reliability_row(event, handler)?
            .internal_ref
            .clone();
        self.view_model()?.detail(&reference)
    }

    pub fn matched_runtime_catalog_history(&self) -> Option<&CatalogHistoryViewModel> {
        let detail = self.matched_reliability_detail()?;
        self.changes()
            .and_then(|changes| changes.catalog_history(&detail.internal_ref))
    }

    pub const fn diagnostics_state(&self) -> &ResourceState<DiagnosticsReport> {
        &self.diagnostics
    }

    pub fn diagnostics(&self) -> Option<&DiagnosticsReport> {
        self.diagnostics.accepted()
    }

    pub const fn changes_state(&self) -> &ResourceState<ChangesViewModel> {
        &self.changes
    }

    pub fn changes(&self) -> Option<&ChangesViewModel> {
        self.changes.accepted()
    }

    pub fn selected_handler(&self) -> Option<&HandlerRef> {
        self.selected_handler.as_ref()
    }

    /// Historical catalog metadata is intentionally sourced from the lazy
    /// long-history snapshot rather than the bounded normal report.
    pub fn selected_catalog_history(&self) -> Option<&CatalogHistoryViewModel> {
        self.selected_handler.as_ref().and_then(|reference| {
            self.changes()
                .and_then(|changes| changes.catalog_history(reference))
        })
    }

    pub fn selected_change(&self) -> Option<&ChangeRef> {
        self.selected_change.as_ref()
    }

    pub const fn selected_failure_cluster(&self) -> Option<FailureClusterRef> {
        self.selected_failure_cluster
    }

    pub fn failure_clusters(&self) -> &[FailureClusterViewModel] {
        self.view_model()
            .map(ReliabilityCenterViewModel::failure_clusters)
            .unwrap_or_default()
    }

    pub const fn detail_scroll_lines(&self) -> u16 {
        self.detail_scroll_lines
    }

    pub const fn changes_detail_scroll_lines(&self) -> u16 {
        self.changes_detail_scroll_lines
    }

    pub fn runtime_event_selection_index(&self) -> Option<usize> {
        let selected = self.selected_runtime_event()?;
        self.runtime_catalog()?.events.iter().position(|event| {
            event.runtime_context == selected.runtime_context
                && event.runtime_event_name == selected.runtime_event_name
        })
    }

    pub fn runtime_handler_selection_index(&self) -> Option<usize> {
        let selected = self.selected_runtime_handler()?;
        self.selected_runtime_event()?
            .handlers
            .iter()
            .position(|handler| handler.runtime_catalog_id == selected.runtime_catalog_id)
    }

    #[cfg(test)]
    pub const fn hooks_query(&self) -> &HooksQuery {
        &self.hooks_query
    }

    #[cfg(test)]
    pub fn visible_hooks(&self) -> &[HookRowViewModel] {
        &self.visible_hooks
    }

    pub const fn is_text_editing(&self) -> bool {
        self.search_editing || self.alias_text_editing
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
            || self.alias_editor.awaiting_discard_confirmation()
    }

    pub const fn local_list_active(&self) -> bool {
        matches!(
            self.local_mode,
            LocalMode::HooksEvents
                | LocalMode::HooksHandlers
                | LocalMode::ChangesList
                | LocalMode::FailureClusters
        )
    }

    pub const fn hooks_events_active(&self) -> bool {
        matches!(self.local_mode, LocalMode::HooksEvents)
    }

    pub const fn hooks_handlers_active(&self) -> bool {
        matches!(self.local_mode, LocalMode::HooksHandlers)
    }

    const fn changes_list_active(&self) -> bool {
        matches!(self.local_mode, LocalMode::ChangesList)
    }

    const fn failure_clusters_active(&self) -> bool {
        matches!(self.local_mode, LocalMode::FailureClusters)
    }

    pub const fn settings_save_state(&self) -> SettingsSaveState {
        self.settings_save_state
    }

    pub const fn alias_editing(&self) -> bool {
        self.alias_handler.is_some()
    }

    pub const fn alias_text_editing(&self) -> bool {
        self.alias_text_editing
    }

    pub fn alias_draft(&self) -> Option<&str> {
        self.alias_handler
            .as_ref()
            .map(|_| self.alias_draft.as_str())
    }

    pub const fn alias_save_state(&self) -> AliasSaveState {
        self.alias_save_state
    }

    pub fn alias_dirty(&self) -> bool {
        self.alias_handler.is_some()
            && self.alias_draft
                != self
                    .alias_expected
                    .as_deref()
                    .unwrap_or(&self.alias_base_label)
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
        if self.discard_confirmation_open() {
            return self.handle_discard_confirmation(command);
        }
        match command {
            Command::Quit => self.request_quit(),
            Command::Help => {
                self.help_overlay.open_help();
                AppEffect::None
            }
            Command::Discard => AppEffect::None,
            Command::Up => self.move_direction(-1),
            Command::Down => self.move_direction(1),
            Command::PageUp => {
                self.page_content(-1);
                AppEffect::None
            }
            Command::PageDown => {
                self.page_content(1);
                AppEffect::None
            }
            Command::Enter => {
                if self.alias_text_editing {
                    self.finish_alias_text_edit();
                } else if self.search_editing {
                    self.search_editing = false;
                } else if self.screen == Screen::Settings {
                    self.settings_editor.enter_or_finish();
                } else if self.screen == Screen::Hooks && self.local_mode == LocalMode::None {
                    self.local_mode = LocalMode::HooksEvents;
                    self.detail_scroll_lines = 0;
                    self.repair_runtime_event_selection();
                } else if self.screen == Screen::Hooks
                    && self.hooks_events_active()
                    && self.selected_runtime_event().is_some()
                {
                    self.local_mode = LocalMode::HooksHandlers;
                    self.detail_scroll_lines = 0;
                    self.repair_runtime_handler_selection();
                } else if self.screen == Screen::Hooks
                    && self.hooks_handlers_active()
                    && self.selected_runtime_handler().is_some()
                {
                    self.screen = Screen::HookDetail;
                    self.detail_scroll_lines = 0;
                    if matches!(self.changes_state(), ResourceState::Empty) {
                        return self
                            .request_changes(ChangesRefreshReason::Entered(self.requested_window));
                    }
                } else if self.screen == Screen::Changes && !self.changes_list_active() {
                    self.local_mode = LocalMode::ChangesList;
                    self.repair_change_selection();
                } else if self.screen == Screen::Changes
                    && self.changes_list_active()
                    && self.selected_change.is_some()
                {
                    self.screen = Screen::ChangeDetail;
                    self.changes_detail_scroll_lines = 0;
                } else if self.screen == Screen::FailureClusters
                    && self.failure_clusters_active()
                    && self.selected_failure_cluster.is_some()
                {
                    self.screen = Screen::FailureClusterDetail;
                    self.detail_scroll_lines = 0;
                } else if self.screen == Screen::Overview && self.selected_handler.is_some() {
                    self.navigation.activate(Route::Hooks);
                    self.screen = Screen::HookDetail;
                    self.local_mode = LocalMode::HooksEvents;
                    self.detail_scroll_lines = 0;
                    if matches!(self.changes_state(), ResourceState::Empty) {
                        return self
                            .request_changes(ChangesRefreshReason::Entered(self.requested_window));
                    }
                }
                AppEffect::None
            }
            Command::Back => {
                if self.alias_text_editing || self.alias_editing() {
                    self.cancel_alias_edit();
                } else if self.search_editing {
                    self.search_editing = false;
                } else if self.screen == Screen::HookDetail {
                    self.navigation.activate(Route::Hooks);
                    self.screen = Screen::Hooks;
                    self.local_mode = if self.selected_runtime_handler.is_some() {
                        LocalMode::HooksHandlers
                    } else {
                        LocalMode::HooksEvents
                    };
                    self.detail_scroll_lines = 0;
                    self.repair_runtime_handler_selection();
                } else if self.screen == Screen::Hooks && self.hooks_handlers_active() {
                    self.local_mode = LocalMode::HooksEvents;
                    self.repair_runtime_event_selection();
                } else if self.screen == Screen::Hooks && self.hooks_events_active() {
                    self.local_mode = LocalMode::None;
                } else if self.screen == Screen::ChangeDetail {
                    self.navigation.activate(Route::Changes);
                    self.screen = Screen::Changes;
                    self.local_mode = LocalMode::ChangesList;
                    self.changes_detail_scroll_lines = 0;
                    self.repair_change_selection();
                } else if self.screen == Screen::Changes && self.changes_list_active() {
                    self.local_mode = LocalMode::None;
                } else if self.screen == Screen::FailureClusterDetail {
                    self.screen = Screen::FailureClusters;
                    self.local_mode = LocalMode::FailureClusters;
                    self.detail_scroll_lines = 0;
                    self.repair_failure_cluster_selection();
                } else if self.screen == Screen::FailureClusters && self.failure_clusters_active() {
                    self.screen = Screen::HookDetail;
                    self.local_mode = LocalMode::HooksEvents;
                } else if self.screen == Screen::Settings && self.settings_editor.is_editing() {
                    self.settings_editor.enter_or_finish();
                }
                AppEffect::None
            }
            Command::Refresh if self.alias_editing() => {
                self.revert_alias();
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
            Command::Refresh if self.screen == Screen::Changes => {
                self.request_changes(ChangesRefreshReason::Explicit(self.requested_window))
            }
            Command::Refresh if matches!(self.screen, Screen::Hooks | Screen::HookDetail) => {
                self.runtime_catalog.request_explicit_refresh();
                self.request_refresh_and_runtime_catalog(
                    RefreshReason::Manual(self.requested_window),
                    RuntimeCatalogRefreshReason::Explicit,
                )
            }
            Command::Refresh => self.request_refresh(RefreshReason::Manual(self.requested_window)),
            Command::Window(window) => {
                if self.alias_editing() {
                    return self.apply_alias();
                }
                if self.screen == Screen::Settings && self.settings_editor.is_editing() {
                    return self.apply_interface();
                }
                self.requested_window = window;
                if matches!(
                    self.screen,
                    Screen::Hooks | Screen::HookDetail | Screen::Changes | Screen::ChangeDetail
                ) {
                    self.request_refresh_and_changes(
                        RefreshReason::Window(window),
                        ChangesRefreshReason::Window(window),
                    )
                } else {
                    self.request_refresh(RefreshReason::Window(window))
                }
            }
            Command::Search => AppEffect::None,
            Command::SearchInput(value) => {
                if self.alias_text_editing {
                    self.alias_draft.push(value);
                    self.alias_save_state = if self.alias_dirty() {
                        AliasSaveState::Dirty
                    } else {
                        AliasSaveState::Clean
                    };
                } else if self.search_editing {
                    self.hooks_query.search.push(value);
                    self.rebuild_visible_hooks();
                    self.repair_handler_selection();
                }
                AppEffect::None
            }
            Command::SearchBackspace => {
                if self.alias_text_editing {
                    self.alias_draft.pop();
                    self.alias_save_state = if self.alias_dirty() {
                        AliasSaveState::Dirty
                    } else {
                        AliasSaveState::Clean
                    };
                } else if self.search_editing {
                    self.hooks_query.search.pop();
                    self.rebuild_visible_hooks();
                    self.repair_handler_selection();
                }
                AppEffect::None
            }
            Command::CloseSearch => {
                if self.alias_text_editing {
                    self.finish_alias_text_edit();
                } else {
                    self.search_editing = false;
                }
                AppEffect::None
            }
            Command::Filter => {
                if self.screen == Screen::HookDetail && self.selected_runtime_handler.is_none() {
                    self.screen = Screen::FailureClusters;
                    self.local_mode = LocalMode::FailureClusters;
                    self.repair_failure_cluster_selection();
                }
                AppEffect::None
            }
            Command::Sort => AppEffect::None,
            Command::EditAlias => {
                if self.screen == Screen::HookDetail {
                    self.begin_alias_edit();
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
        let (view_model, alias_annotations) = snapshot.into_parts();
        if view_model.overview.window != self.requested_window {
            // Belt-and-braces request ownership: a worker response for an old
            // period cannot overwrite a newer visible request even if a future
            // transport implementation becomes concurrent.
            return;
        }
        self.requested_window = view_model.overview.window;
        self.view = std::mem::replace(&mut self.view, ResourceState::Empty).ready(view_model);
        self.alias_annotations = alias_annotations;
        self.rebuild_visible_hooks();
        self.repair_handler_selection();
    }

    /// Accepts a local-only runtime catalog. Selection is retained by the
    /// runtime's exact context/event and catalog handler identity whenever the
    /// refreshed catalog still contains them.
    pub fn apply_runtime_catalog(&mut self, snapshot: RuntimePresentationSnapshot) {
        self.runtime_catalog.accepted(snapshot);
        self.repair_runtime_event_selection();
        self.repair_runtime_handler_selection();
    }

    /// A catalog refresh error never erases the last accepted runtime truth,
    /// and never makes the separate reliability resource unavailable.
    pub fn reject_runtime_catalog(&mut self) {
        self.runtime_catalog.failed();
    }

    pub fn runtime_catalog_initial_load_pending(&self) -> bool {
        self.runtime_catalog.state() == RuntimeCatalogResourceState::Loading
            && self.runtime_catalog.accepted_snapshot().is_none()
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

    pub fn apply_changes(&mut self, snapshot: ChangesSnapshot) {
        if snapshot.view_model.window != self.requested_window {
            return;
        }
        self.changes =
            std::mem::replace(&mut self.changes, ResourceState::Empty).ready(snapshot.view_model);
        self.rebuild_visible_changes();
        self.repair_change_selection();
    }

    pub fn reject_changes(&mut self) {
        self.changes = std::mem::replace(&mut self.changes, ResourceState::Empty)
            .error("changes_refresh_failed");
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
        if self.alias_editor.awaiting_discard_confirmation() {
            return match command {
                Command::Back | Command::Quit => {
                    let _ = self.alias_editor.resolve_discard(DiscardDecision::Cancel);
                    AppEffect::None
                }
                Command::Enter | Command::Discard => {
                    if self.alias_editor.resolve_discard(DiscardDecision::Discard) {
                        self.cancel_alias_edit();
                        AppEffect::Quit
                    } else {
                        AppEffect::None
                    }
                }
                _ => AppEffect::None,
            };
        }
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
        if self.alias_editing() {
            return match self.alias_editor.request_quit(self.alias_dirty()) {
                QuitDisposition::Quit => AppEffect::Quit,
                QuitDisposition::ConfirmDiscard => AppEffect::None,
            };
        }
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

    fn move_direction(&mut self, delta: isize) -> AppEffect {
        if self.screen == Screen::HookDetail || self.screen == Screen::FailureClusterDetail {
            self.move_detail_scroll(delta);
            AppEffect::None
        } else if self.screen == Screen::ChangeDetail {
            self.move_changes_detail_scroll(delta);
            AppEffect::None
        } else if self.screen == Screen::Settings && self.settings_editor.is_editing() {
            let _ = self.settings_editor.move_field(&SettingsField::ALL, delta);
            AppEffect::None
        } else if self.screen == Screen::Hooks && self.hooks_events_active() {
            self.move_runtime_event(delta);
            AppEffect::None
        } else if self.screen == Screen::Hooks && self.hooks_handlers_active() {
            self.move_runtime_handler(delta);
            AppEffect::None
        } else if self.screen == Screen::Changes && self.changes_list_active() {
            self.move_change(delta);
            AppEffect::None
        } else if self.screen == Screen::FailureClusters && self.failure_clusters_active() {
            self.move_failure_cluster(delta);
            AppEffect::None
        } else {
            self.navigation.move_by(delta);
            self.screen = match self.navigation.current() {
                Route::Overview => Screen::Overview,
                Route::Hooks => Screen::Hooks,
                Route::Changes => Screen::Changes,
                Route::Diagnostics => Screen::Diagnostics,
                Route::Settings => Screen::Settings,
            };
            self.local_mode = LocalMode::None;
            self.repair_handler_selection();
            if matches!(self.screen, Screen::Changes | Screen::Hooks)
                && !self.changes_state().is_loading()
            {
                self.request_changes(ChangesRefreshReason::Entered(self.requested_window))
            } else {
                AppEffect::None
            }
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

    fn request_changes(&mut self, reason: ChangesRefreshReason) -> AppEffect {
        self.changes = std::mem::replace(&mut self.changes, ResourceState::Empty).loading();
        AppEffect::RequestChanges(reason)
    }

    fn request_refresh_and_changes(
        &mut self,
        refresh: RefreshReason,
        changes: ChangesRefreshReason,
    ) -> AppEffect {
        self.view = std::mem::replace(&mut self.view, ResourceState::Empty).loading();
        self.changes = std::mem::replace(&mut self.changes, ResourceState::Empty).loading();
        AppEffect::RequestRefreshAndChanges(refresh, changes)
    }

    fn request_refresh_and_runtime_catalog(
        &mut self,
        refresh: RefreshReason,
        catalog: RuntimeCatalogRefreshReason,
    ) -> AppEffect {
        self.view = std::mem::replace(&mut self.view, ResourceState::Empty).loading();
        AppEffect::RequestRefreshAndRuntimeCatalog(refresh, catalog)
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

    fn begin_alias_edit(&mut self) {
        let Some(reference) = self.selected_handler.clone() else {
            return;
        };
        let Some(detail) = self.view_model().and_then(|view| view.detail(&reference)) else {
            return;
        };
        let base_label = match &detail.display_identity {
            DisplayIdentity::ExistingMetadata(label) => label.clone(),
            DisplayIdentity::EventFallback(event) => event.as_storage().replace('_', " "),
        };
        let expected_alias = self
            .alias_annotations
            .iter()
            .find(|alias| {
                alias.runtime == reference.runtime && alias.handler_key == reference.handler_key
            })
            .map(|alias| alias.display_name.clone());
        self.alias_handler = Some(reference);
        self.alias_expected = expected_alias.clone();
        self.alias_base_label = base_label;
        self.alias_draft = expected_alias.unwrap_or_else(|| self.alias_base_label.clone());
        self.alias_editor = SettingsEditor::new(AliasField::Name);
        self.alias_editor.enter_or_finish();
        self.alias_text_editing = true;
        self.alias_save_state = AliasSaveState::Clean;
    }

    fn finish_alias_text_edit(&mut self) {
        self.alias_text_editing = false;
        if self.alias_editor.is_editing() {
            self.alias_editor.enter_or_finish();
        }
        self.alias_save_state = if self.alias_dirty() {
            AliasSaveState::Dirty
        } else {
            AliasSaveState::Clean
        };
    }

    fn cancel_alias_edit(&mut self) {
        self.alias_text_editing = false;
        self.alias_editor = SettingsEditor::new(AliasField::Name);
        self.alias_handler = None;
        self.alias_expected = None;
        self.alias_base_label.clear();
        self.alias_draft.clear();
        self.alias_save_state = AliasSaveState::Clean;
    }

    fn revert_alias(&mut self) {
        if self.alias_handler.is_none() {
            return;
        }
        self.alias_draft = self
            .alias_expected
            .clone()
            .unwrap_or_else(|| self.alias_base_label.clone());
        self.alias_text_editing = false;
        if self.alias_editor.is_editing() {
            self.alias_editor.enter_or_finish();
        }
        self.alias_save_state = AliasSaveState::Clean;
    }

    fn apply_alias(&mut self) -> AppEffect {
        if !self.alias_dirty() {
            return AppEffect::None;
        }
        let Some(reference) = self.alias_handler.as_ref() else {
            return AppEffect::None;
        };
        AppEffect::ApplyAlias(AliasApplyRequest {
            runtime: reference.runtime,
            handler_key: reference.handler_key.clone(),
            draft: self.alias_draft.clone(),
            expected_alias: self.alias_expected.clone(),
        })
    }

    pub fn alias_apply_result(&mut self, outcome: AliasApplyOutcome) {
        match outcome {
            AliasApplyOutcome::Saved => {
                if let Some(reference) = &self.alias_handler {
                    self.alias_annotations.retain(|alias| {
                        alias.runtime != reference.runtime
                            || alias.handler_key != reference.handler_key
                    });
                    self.alias_annotations.push(AliasAnnotation {
                        runtime: reference.runtime,
                        handler_key: reference.handler_key.clone(),
                        display_name: self.alias_draft.clone(),
                    });
                }
                self.alias_expected = Some(self.alias_draft.clone());
                self.alias_save_state = AliasSaveState::Saved;
                self.alias_text_editing = false;
            }
            AliasApplyOutcome::Conflict => self.alias_save_state = AliasSaveState::Conflict,
            AliasApplyOutcome::Failed => self.alias_save_state = AliasSaveState::Failed,
        }
    }

    fn page_content(&mut self, direction: isize) {
        if self.screen == Screen::HookDetail || self.screen == Screen::FailureClusterDetail {
            self.move_detail_scroll(direction.saturating_mul(6));
        } else if self.screen == Screen::ChangeDetail {
            self.move_changes_detail_scroll(direction.saturating_mul(6));
        } else if self.screen == Screen::Hooks && self.hooks_events_active() {
            self.move_runtime_event(direction.saturating_mul(5));
        } else if self.screen == Screen::Hooks && self.hooks_handlers_active() {
            self.move_runtime_handler(direction.saturating_mul(5));
        } else if self.screen == Screen::Changes && self.changes_list_active() {
            self.move_change(direction.saturating_mul(5));
        } else if self.screen == Screen::FailureClusters && self.failure_clusters_active() {
            self.move_failure_cluster(direction.saturating_mul(5));
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

    fn move_changes_detail_scroll(&mut self, delta: isize) {
        self.changes_detail_scroll_lines = if delta.is_negative() {
            self.changes_detail_scroll_lines
                .saturating_sub(delta.unsigned_abs() as u16)
        } else {
            self.changes_detail_scroll_lines
                .saturating_add(delta as u16)
        };
    }

    fn rebuild_visible_hooks(&mut self) {
        self.visible_hooks = self
            .view
            .accepted()
            .map(|view| view.filtered_hooks(&self.hooks_query))
            .unwrap_or_default();
        self.detail_scroll_lines = 0;
    }

    fn runtime_event_keys(&self) -> Vec<(String, String)> {
        self.runtime_catalog()
            .map(|catalog| {
                catalog
                    .events
                    .iter()
                    .map(|event| {
                        (
                            event.runtime_context.clone(),
                            event.runtime_event_name.clone(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn move_runtime_event(&mut self, delta: isize) {
        let events = self.runtime_event_keys();
        if events.is_empty() {
            self.selected_runtime_event = None;
            self.selected_runtime_handler = None;
            return;
        }
        let current = self
            .selected_runtime_event
            .as_ref()
            .and_then(|selected| events.iter().position(|event| event == selected))
            .unwrap_or(0);
        let next = (current as isize + delta).rem_euclid(events.len() as isize) as usize;
        self.selected_runtime_event = Some(events[next].clone());
        self.selected_runtime_handler = None;
        self.detail_scroll_lines = 0;
    }

    fn repair_runtime_event_selection(&mut self) {
        let events = self.runtime_event_keys();
        if !self
            .selected_runtime_event
            .as_ref()
            .is_some_and(|selected| events.iter().any(|event| event == selected))
        {
            self.selected_runtime_event = events.into_iter().next();
            self.selected_runtime_handler = None;
        }
    }

    fn runtime_handler_ids(&self) -> Vec<String> {
        self.selected_runtime_event()
            .map(|event| {
                event
                    .handlers
                    .iter()
                    .map(|handler| handler.runtime_catalog_id.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn move_runtime_handler(&mut self, delta: isize) {
        let handlers = self.runtime_handler_ids();
        if handlers.is_empty() {
            self.selected_runtime_handler = None;
            return;
        }
        let current = self
            .selected_runtime_handler
            .as_ref()
            .and_then(|selected| handlers.iter().position(|handler| handler == selected))
            .unwrap_or(0);
        let next = (current as isize + delta).rem_euclid(handlers.len() as isize) as usize;
        self.selected_runtime_handler = Some(handlers[next].clone());
        self.detail_scroll_lines = 0;
    }

    fn repair_runtime_handler_selection(&mut self) {
        let handlers = self.runtime_handler_ids();
        if !self
            .selected_runtime_handler
            .as_ref()
            .is_some_and(|selected| handlers.iter().any(|handler| handler == selected))
        {
            self.selected_runtime_handler = handlers.into_iter().next();
        }
    }

    fn rebuild_visible_changes(&mut self) {
        self.visible_changes = self
            .changes
            .accepted()
            .map(|view| view.rows.clone())
            .unwrap_or_default();
    }

    fn move_change(&mut self, delta: isize) {
        if self.visible_changes.is_empty() {
            self.selected_change = None;
            return;
        }
        let current = self
            .selected_change
            .as_ref()
            .and_then(|selected| {
                self.visible_changes
                    .iter()
                    .position(|candidate| candidate.reference == *selected)
            })
            .unwrap_or(0);
        let next =
            (current as isize + delta).rem_euclid(self.visible_changes.len() as isize) as usize;
        self.selected_change = Some(self.visible_changes[next].reference.clone());
    }

    fn repair_change_selection(&mut self) {
        let preserved = self.selected_change.as_ref().is_some_and(|selected| {
            self.visible_changes
                .iter()
                .any(|candidate| candidate.reference == *selected)
        });
        if !preserved {
            self.selected_change = self
                .visible_changes
                .first()
                .map(|row| row.reference.clone());
        }
    }

    fn move_failure_cluster(&mut self, delta: isize) {
        let clusters = self.failure_clusters();
        if clusters.is_empty() {
            self.selected_failure_cluster = None;
            return;
        }
        let current = self
            .selected_failure_cluster
            .and_then(|selected| {
                clusters
                    .iter()
                    .position(|cluster| cluster.reference == selected)
            })
            .unwrap_or(0);
        let next = (current as isize + delta).rem_euclid(clusters.len() as isize) as usize;
        self.selected_failure_cluster = Some(clusters[next].reference);
    }

    fn repair_failure_cluster_selection(&mut self) {
        let preserved = self.selected_failure_cluster.is_some_and(|selected| {
            self.failure_clusters()
                .iter()
                .any(|cluster| cluster.reference == selected)
        });
        if !preserved {
            self.selected_failure_cluster = self
                .failure_clusters()
                .first()
                .map(|cluster| cluster.reference);
        }
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
            Screen::Changes
            | Screen::ChangeDetail
            | Screen::FailureClusters
            | Screen::FailureClusterDetail
            | Screen::Diagnostics => Vec::new(),
            Screen::Settings => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::synthetic_fixture_report;
    use serde_json::json;

    fn runtime_catalog() -> RuntimePresentationSnapshot {
        RuntimePresentationSnapshot::from_codex_hooks_list(
            &json!({"result":{"data":[{
                "cwd":"C:/synthetic/workspace",
                "warnings":["synthetic warning"],
                "errors":["synthetic error"],
                "hooks":[
                    {"key":"fixture:0:0","eventName":"PreToolUse","handlerType":"command","command":"synthetic command --long-safe-argument","matcher":"^SyntheticTool$","source":"user","sourcePath":"C:/synthetic/hooks.json","enabled":true,"isManaged":false,"trustStatus":"trusted","async":false},
                    {"key":"fixture:0:1","eventName":"Interrupt","handlerType":"agent","source":"managed","enabled":true,"isManaged":true,"trustStatus":"trusted"},
                    {"key":"fixture:0:2","eventName":"FutureRuntimeEvent","handlerType":"prompt","source":"project","enabled":false,"isManaged":false,"trustStatus":"modified"},
                    {"key":"fixture:0:3","eventName":"Stop","handlerType":"command","command":"synthetic stop","enabled":true,"isManaged":false,"trustStatus":"trusted"}
                ]
            }]}}),
            1_000,
        )
        .unwrap()
    }

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
    fn hooks_control_center_navigates_events_handlers_and_detail_without_mutating_history() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.apply_runtime_catalog(runtime_catalog());
        app.handle(Command::Down);
        app.handle(Command::Enter);
        assert_eq!(app.screen(), Screen::Hooks);
        assert!(app.hooks_events_active());
        assert_eq!(
            app.selected_runtime_event()
                .map(|event| event.runtime_event_name.as_str()),
            Some("FutureRuntimeEvent")
        );
        app.handle(Command::Down);
        assert_eq!(
            app.selected_runtime_event()
                .map(|event| event.runtime_event_name.as_str()),
            Some("Interrupt")
        );
        app.handle(Command::Enter);
        assert!(app.hooks_handlers_active());
        assert!(app.selected_runtime_handler().is_some());
        app.handle(Command::Enter);
        assert_eq!(app.screen(), Screen::HookDetail);
        app.handle(Command::Back);
        assert_eq!(app.screen(), Screen::Hooks);
        assert!(app.hooks_handlers_active());
        app.handle(Command::Back);
        assert!(app.hooks_events_active());
        app.handle(Command::Back);
        assert!(!app.local_list_active());
        assert_eq!(app.visible_hooks().len(), 2);
    }

    #[test]
    fn runtime_catalog_refresh_is_explicit_and_selection_survives_non_destructive_update() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        let catalog = runtime_catalog();
        app.apply_runtime_catalog(catalog.clone());
        assert_eq!(
            app.runtime_catalog_state(),
            RuntimeCatalogResourceState::Ready
        );
        app.handle(Command::Down);
        app.handle(Command::Enter);
        app.handle(Command::Enter);
        app.handle(Command::Down);
        let selected = app.selected_runtime_event().map(|event| {
            (
                event.runtime_context.clone(),
                event.runtime_event_name.clone(),
            )
        });
        assert!(matches!(
            app.handle(Command::Refresh),
            AppEffect::RequestRefreshAndRuntimeCatalog(_, RuntimeCatalogRefreshReason::Explicit)
        ));
        app.apply_runtime_catalog(catalog);
        assert_eq!(
            app.selected_runtime_event().map(|event| (
                event.runtime_context.clone(),
                event.runtime_event_name.clone()
            )),
            selected
        );
        let period = app.handle(Command::Window(TimeWindow::Last30Days));
        assert!(matches!(period, AppEffect::RequestRefreshAndChanges(_, _)));
        assert_eq!(
            app.runtime_catalog_state(),
            RuntimeCatalogResourceState::Ready
        );
    }

    #[test]
    fn runtime_and_reliability_failures_do_not_erase_the_other_accepted_resource() {
        let report = synthetic_fixture_report(1_000);
        let mut app = App::from_report(report);
        app.apply_runtime_catalog(runtime_catalog());
        app.reject_runtime_catalog();
        assert!(app.runtime_catalog().is_some());
        assert!(app.view_model().is_some());

        app.reject_refresh();
        assert!(app.runtime_catalog().is_some());
        assert!(app.view_model().is_some());
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
    fn catalog_and_changes_navigation_share_one_lazy_history_snapshot() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        assert_eq!(
            app.handle(Command::Down),
            AppEffect::RequestChanges(ChangesRefreshReason::Entered(TimeWindow::Last7Days))
        );
        assert_eq!(app.screen(), Screen::Hooks);
        assert!(app.changes_state().is_loading());
        assert_eq!(app.handle(Command::Down), AppEffect::None);
        assert_eq!(app.screen(), Screen::Changes);
        assert_eq!(app.navigation().current(), Route::Changes);
        assert!(app.changes_state().is_loading());
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

    #[test]
    fn alias_edit_is_draft_only_until_conflict_safe_apply_result() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.handle(Command::Enter);
        assert_eq!(app.screen(), Screen::HookDetail);
        app.handle(Command::EditAlias);
        assert!(app.alias_editing());
        app.handle(Command::SearchInput('!'));
        assert!(app.alias_dirty());
        app.handle(Command::Enter);
        let effect = app.handle(Command::Window(TimeWindow::All));
        let AppEffect::ApplyAlias(request) = effect else {
            panic!("alias apply must be explicit");
        };
        assert!(request.expected_alias.is_none());
        app.alias_apply_result(AliasApplyOutcome::Saved);
        assert_eq!(app.alias_save_state(), AliasSaveState::Saved);
        assert!(!app.alias_dirty());

        app.handle(Command::EditAlias);
        app.handle(Command::SearchInput('?'));
        app.handle(Command::Enter);
        app.alias_apply_result(AliasApplyOutcome::Conflict);
        assert_eq!(app.alias_save_state(), AliasSaveState::Conflict);
        app.handle(Command::Back);
        assert!(!app.alias_editing());
    }

    #[test]
    fn dirty_alias_quit_uses_the_shared_discard_confirmation() {
        let mut app = App::from_report(synthetic_fixture_report(1_000));
        app.handle(Command::Enter);
        app.handle(Command::EditAlias);
        app.handle(Command::SearchInput('!'));
        app.handle(Command::Enter);
        assert!(app.alias_dirty());

        assert_eq!(app.handle(Command::Quit), AppEffect::None);
        assert!(app.discard_confirmation_open());
        app.handle(Command::Back);
        assert!(!app.discard_confirmation_open());
        assert!(app.alias_dirty());

        assert_eq!(app.handle(Command::Quit), AppEffect::None);
        assert_eq!(app.handle(Command::Discard), AppEffect::Quit);
        assert!(!app.alias_editing());
    }
}
