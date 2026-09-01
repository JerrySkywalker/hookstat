//! Deterministic whole-frame visual regression coverage for the production TUI.
//!
//! This module is test-only. Every frame is constructed from repository-owned,
//! sanitized fixtures and rendered through [`super::rendering::draw`].

use super::app::{App, ChangesSnapshot};
use super::keymap::Command;
use super::layout::{ApplicationShell, ShellAreas, ShellLayout};
use super::localization::{
    InterfaceLanguage, LanguageState, MessageKey, ResolvedLocale, known_runtime_event_description,
    known_runtime_event_name, t,
};
use super::rendering::draw;
use super::theme::Theme;
use crate::analytics::TimeWindow;
use crate::diagnostics::{DiagnosticCheck, DiagnosticCheckId, DiagnosticStatus, DiagnosticsReport};
use crate::domain::{
    EvidenceCoverage, EvidenceKind, ExecutionMode, HandlerIdentity, HookEvent, HookInvocation,
    Runtime, TerminalStatus,
};
use crate::report::{instrumented_report, synthetic_fixture_report};
use crate::runtime_presentation::{
    KnownRuntimeEvent, RuntimeHandlerKind, RuntimePresentationSnapshot,
};
use ratatui::{Terminal, backend::TestBackend, layout::Rect, style::Modifier};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use unicode_width::UnicodeWidthStr;

const PRESENTATION_NOW_UNIX_MS: i64 = 1_725_000_123_456;
const CATALOG_CAPTURED_AT_UNIX_MS: i64 = 1_725_000_987_654;
const REAL_WIRE_CAPTURED_AT_UNIX_MS: i64 = 1_725_001_111_222;
const CHANGE_NOW_UNIX_MS: i64 = 20 * 24 * 60 * 60 * 1_000;
const BASELINE_UPDATE_ENV: &str = "HOOKSTAT_UPDATE_VISUAL_BASELINES";
const REAL_WIRE_FIXTURE_JSON: &str =
    include_str!("../../tests/fixtures/codex/v0.151.0/hooks-list.json");
const REAL_WIRE_CONTRACT_JSON: &str =
    include_str!("../../tests/fixtures/codex/v0.151.0/wire-contract.json");

const KNOWN_RUNTIME_EVENTS: [KnownRuntimeEvent; 12] = [
    KnownRuntimeEvent::PreToolUse,
    KnownRuntimeEvent::PermissionRequest,
    KnownRuntimeEvent::PostToolUse,
    KnownRuntimeEvent::PreCompact,
    KnownRuntimeEvent::PostCompact,
    KnownRuntimeEvent::SessionStart,
    KnownRuntimeEvent::SessionEnd,
    KnownRuntimeEvent::UserPromptSubmit,
    KnownRuntimeEvent::SubagentStart,
    KnownRuntimeEvent::SubagentStop,
    KnownRuntimeEvent::Stop,
    KnownRuntimeEvent::Interrupt,
];

#[derive(Clone, Copy)]
enum Scenario {
    OverviewReady,
    OverviewLoading,
    OverviewErrorWithAcceptedData,
    OverviewZeroTerminalSamples,
    HooksEventsReady,
    HooksEventsLoading,
    HooksEventsErrorWithAcceptedData,
    HooksHandlersCommand,
    HooksHandlersNeedsReview,
    HooksHandlersManaged,
    HookDetailLongConfiguration,
    ChangesReady,
    ChangesLoadingWithAcceptedData,
    ChangesErrorWithAcceptedData,
    DiagnosticsReady,
    DiagnosticsLoadingWithAcceptedData,
    DiagnosticsErrorWithAcceptedData,
    SettingsReady,
    RealWireEvents,
    RealWireHandlersPreToolUse,
    RealWireManagedDetail,
}

#[derive(Clone, Copy)]
struct FrameCase {
    id: &'static str,
    width: u16,
    height: u16,
    locale: ResolvedLocale,
    scenario: Scenario,
    selected_row_invariant: bool,
    event_display_count_invariant: bool,
    visible_event_duplicate_invariant: bool,
    selected_event_display_count_invariant: bool,
    runtime_before_reliability: bool,
    zero_sample_invariant: bool,
}

impl FrameCase {
    const fn new(
        id: &'static str,
        width: u16,
        height: u16,
        locale: ResolvedLocale,
        scenario: Scenario,
    ) -> Self {
        Self {
            id,
            width,
            height,
            locale,
            scenario,
            selected_row_invariant: false,
            event_display_count_invariant: false,
            visible_event_duplicate_invariant: false,
            selected_event_display_count_invariant: false,
            runtime_before_reliability: false,
            zero_sample_invariant: false,
        }
    }

    const fn selected_row(mut self) -> Self {
        self.selected_row_invariant = true;
        self
    }

    const fn event_display_counts(mut self) -> Self {
        self.event_display_count_invariant = true;
        self
    }

    const fn selected_event_display_count(mut self) -> Self {
        self.selected_event_display_count_invariant = true;
        self
    }

    const fn visible_event_duplicates(mut self) -> Self {
        self.visible_event_duplicate_invariant = true;
        self
    }

    const fn ordered_detail(mut self) -> Self {
        self.runtime_before_reliability = true;
        self
    }

    const fn zero_samples(mut self) -> Self {
        self.zero_sample_invariant = true;
        self
    }

    const fn locale_tag(self) -> &'static str {
        match self.locale {
            ResolvedLocale::EnUs => "en-US",
            ResolvedLocale::ZhCn => "zh-CN",
        }
    }
}

struct RenderedFrame {
    baseline: String,
    plain_text: String,
    selected_rows: BTreeSet<u16>,
    cell_count: usize,
    row_terminal_cells: Vec<usize>,
}

fn canonical_cases() -> Vec<FrameCase> {
    vec![
        FrameCase::new(
            "overview-ready-wide-en-us",
            140,
            58,
            ResolvedLocale::EnUs,
            Scenario::OverviewReady,
        ),
        FrameCase::new(
            "overview-ready-standard-zh-cn",
            100,
            32,
            ResolvedLocale::ZhCn,
            Scenario::OverviewReady,
        ),
        FrameCase::new(
            "overview-loading-narrow-en-us",
            60,
            30,
            ResolvedLocale::EnUs,
            Scenario::OverviewLoading,
        ),
        FrameCase::new(
            "overview-error-very-narrow-zh-cn",
            44,
            44,
            ResolvedLocale::ZhCn,
            Scenario::OverviewErrorWithAcceptedData,
        ),
        FrameCase::new(
            "overview-zero-samples-standard-en-us",
            100,
            32,
            ResolvedLocale::EnUs,
            Scenario::OverviewZeroTerminalSamples,
        )
        .zero_samples(),
        FrameCase::new(
            "hooks-events-ready-wide-en-us",
            140,
            58,
            ResolvedLocale::EnUs,
            Scenario::HooksEventsReady,
        )
        .selected_row()
        .event_display_counts(),
        FrameCase::new(
            "hooks-events-ready-standard-zh-cn",
            100,
            32,
            ResolvedLocale::ZhCn,
            Scenario::HooksEventsReady,
        )
        .selected_row()
        .visible_event_duplicates()
        .selected_event_display_count(),
        FrameCase::new(
            "hooks-events-loading-narrow-en-us",
            60,
            30,
            ResolvedLocale::EnUs,
            Scenario::HooksEventsLoading,
        ),
        FrameCase::new(
            "hooks-events-stale-error-very-narrow-zh-cn",
            44,
            44,
            ResolvedLocale::ZhCn,
            Scenario::HooksEventsErrorWithAcceptedData,
        )
        .selected_row()
        .visible_event_duplicates()
        .selected_event_display_count(),
        FrameCase::new(
            "hooks-handlers-command-wide-en-us",
            140,
            58,
            ResolvedLocale::EnUs,
            Scenario::HooksHandlersCommand,
        )
        .selected_row(),
        FrameCase::new(
            "hooks-handlers-command-standard-zh-cn",
            100,
            32,
            ResolvedLocale::ZhCn,
            Scenario::HooksHandlersCommand,
        )
        .selected_row(),
        FrameCase::new(
            "hooks-handlers-review-narrow-en-us",
            60,
            30,
            ResolvedLocale::EnUs,
            Scenario::HooksHandlersNeedsReview,
        ),
        FrameCase::new(
            "hooks-handlers-managed-standard-en-us",
            100,
            32,
            ResolvedLocale::EnUs,
            Scenario::HooksHandlersManaged,
        )
        .selected_row(),
        FrameCase::new(
            "hook-detail-long-wide-en-us",
            140,
            58,
            ResolvedLocale::EnUs,
            Scenario::HookDetailLongConfiguration,
        )
        .ordered_detail(),
        FrameCase::new(
            "hook-detail-long-standard-zh-cn",
            100,
            32,
            ResolvedLocale::ZhCn,
            Scenario::HookDetailLongConfiguration,
        )
        .ordered_detail(),
        FrameCase::new(
            "hook-detail-long-very-narrow-en-us",
            44,
            44,
            ResolvedLocale::EnUs,
            Scenario::HookDetailLongConfiguration,
        )
        .ordered_detail(),
        FrameCase::new(
            "changes-ready-wide-en-us",
            140,
            58,
            ResolvedLocale::EnUs,
            Scenario::ChangesReady,
        )
        .selected_row(),
        FrameCase::new(
            "changes-ready-narrow-zh-cn",
            60,
            30,
            ResolvedLocale::ZhCn,
            Scenario::ChangesReady,
        ),
        FrameCase::new(
            "changes-stale-loading-standard-en-us",
            100,
            32,
            ResolvedLocale::EnUs,
            Scenario::ChangesLoadingWithAcceptedData,
        ),
        FrameCase::new(
            "changes-stale-error-very-narrow-zh-cn",
            44,
            44,
            ResolvedLocale::ZhCn,
            Scenario::ChangesErrorWithAcceptedData,
        ),
        FrameCase::new(
            "diagnostics-ready-wide-en-us",
            140,
            58,
            ResolvedLocale::EnUs,
            Scenario::DiagnosticsReady,
        ),
        FrameCase::new(
            "diagnostics-ready-narrow-zh-cn",
            60,
            30,
            ResolvedLocale::ZhCn,
            Scenario::DiagnosticsReady,
        ),
        FrameCase::new(
            "diagnostics-stale-loading-standard-en-us",
            100,
            32,
            ResolvedLocale::EnUs,
            Scenario::DiagnosticsLoadingWithAcceptedData,
        ),
        FrameCase::new(
            "diagnostics-stale-error-very-narrow-zh-cn",
            44,
            44,
            ResolvedLocale::ZhCn,
            Scenario::DiagnosticsErrorWithAcceptedData,
        ),
        FrameCase::new(
            "settings-ready-wide-en-us",
            140,
            58,
            ResolvedLocale::EnUs,
            Scenario::SettingsReady,
        ),
        FrameCase::new(
            "settings-ready-very-narrow-zh-cn",
            44,
            44,
            ResolvedLocale::ZhCn,
            Scenario::SettingsReady,
        ),
        FrameCase::new(
            "real-wire-events-wide-en-us",
            140,
            58,
            ResolvedLocale::EnUs,
            Scenario::RealWireEvents,
        )
        .selected_row()
        .event_display_counts(),
        FrameCase::new(
            "real-wire-events-wide-zh-cn",
            140,
            58,
            ResolvedLocale::ZhCn,
            Scenario::RealWireEvents,
        )
        .selected_row()
        .event_display_counts(),
        FrameCase::new(
            "real-wire-handlers-narrow-zh-cn",
            60,
            30,
            ResolvedLocale::ZhCn,
            Scenario::RealWireHandlersPreToolUse,
        )
        .selected_row(),
        FrameCase::new(
            "real-wire-detail-wide-en-us",
            140,
            58,
            ResolvedLocale::EnUs,
            Scenario::RealWireManagedDetail,
        )
        .ordered_detail(),
    ]
}

fn runtime_catalog_fixture() -> Value {
    json!({"result":{"data":[{
        "cwd":"C:/synthetic/workspace",
        "warnings":["synthetic catalog warning"],
        "errors":["synthetic catalog error"],
        "hooks":[
            {
                "key":"fixture:0:0",
                "eventName":"preToolUse",
                "handlerType":"command",
                "command":"synthetic command --very-long-safe-argument=1234567890 --another-safe-argument=abcdefghijklmnopqrstuvwxyz",
                "matcher":"^SyntheticToolWithAnIntentionallyLongSafeName$",
                "source":"project",
                "sourcePath":"C:/synthetic/very/long/source/hooks.json",
                "enabled":true,
                "isManaged":false,
                "trustStatus":"trusted",
                "async":false,
                "timeoutSec":9,
                "additionalContextLimit":64
            },
            {
                "key":"fixture:0:1",
                "eventName":"postToolUse",
                "handlerType":"mcp_tool",
                "mcpServer":"synthetic-server",
                "mcpTool":"synthetic-tool",
                "source":"project",
                "enabled":false,
                "isManaged":false,
                "trustStatus":"untrusted"
            },
            {
                "key":"fixture:0:2",
                "eventName":"userPromptSubmit",
                "handlerType":"prompt",
                "source":"user",
                "enabled":true,
                "isManaged":false,
                "trustStatus":"modified"
            },
            {
                "key":"fixture:0:3",
                "eventName":"subagentStart",
                "handlerType":"agent",
                "source":"managed",
                "enabled":true,
                "isManaged":true,
                "trustStatus":"trusted"
            },
            {
                "key":"fixture:0:4",
                "eventName":"interrupt",
                "handlerType":"command",
                "command":"synthetic interrupt handler",
                "enabled":true,
                "isManaged":false,
                "trustStatus":"trusted"
            },
            {
                "key":"fixture:0:5",
                "eventName":"FutureRuntimeEvent",
                "handlerType":"future_handler",
                "enabled":true,
                "isManaged":false,
                "trustStatus":"trusted"
            }
        ]
    }]}})
}

fn runtime_catalog() -> RuntimePresentationSnapshot {
    RuntimePresentationSnapshot::from_codex_hooks_list(
        &runtime_catalog_fixture(),
        CATALOG_CAPTURED_AT_UNIX_MS,
    )
    .expect("sanitized visual runtime catalog must parse")
}

fn real_wire_fixture() -> Value {
    serde_json::from_str(REAL_WIRE_FIXTURE_JSON)
        .expect("committed Codex v0.151.0 hooks/list fixture must be valid JSON")
}

fn real_wire_catalog() -> RuntimePresentationSnapshot {
    RuntimePresentationSnapshot::from_codex_hooks_list(
        &real_wire_fixture(),
        REAL_WIRE_CAPTURED_AT_UNIX_MS,
    )
    .expect("committed Codex v0.151.0 hooks/list fixture must parse")
}

fn real_wire_ready_app() -> App {
    let mut app = App::from_report(synthetic_fixture_report(PRESENTATION_NOW_UNIX_MS));
    app.apply_runtime_catalog(real_wire_catalog());
    app
}

fn real_wire_zero_sample_app() -> App {
    let mut app = App::from_report(instrumented_report(
        &[],
        PRESENTATION_NOW_UNIX_MS,
        TimeWindow::Last7Days,
        0,
        0,
    ));
    app.apply_runtime_catalog(real_wire_catalog());
    app
}

fn ready_app() -> App {
    let mut app = App::from_report(synthetic_fixture_report(PRESENTATION_NOW_UNIX_MS));
    app.apply_runtime_catalog(runtime_catalog());
    app
}

fn enter_hooks_events(app: &mut App) {
    app.handle(Command::Down);
    app.handle(Command::Enter);
}

fn select_runtime_event(app: &mut App, event_name: &str) {
    let event_count = app
        .runtime_catalog()
        .map_or(0, |catalog| catalog.events.len());
    for _ in 0..event_count {
        if app
            .selected_runtime_event()
            .is_some_and(|event| event.runtime_event_name == event_name)
        {
            return;
        }
        app.handle(Command::Down);
    }
    panic!("visual fixture could not select runtime event {event_name}");
}

fn hooks_handlers_app(event_name: &str) -> App {
    let mut app = ready_app();
    enter_hooks_events(&mut app);
    select_runtime_event(&mut app, event_name);
    app.handle(Command::Enter);
    app
}

fn hook_detail_app() -> App {
    let mut app = hooks_handlers_app("preToolUse");
    app.handle(Command::Enter);
    app
}

fn real_wire_events_app() -> App {
    let mut app = real_wire_ready_app();
    enter_hooks_events(&mut app);
    app
}

fn real_wire_handlers_app(event_name: &str) -> App {
    let mut app = real_wire_events_app();
    select_runtime_event(&mut app, event_name);
    app.handle(Command::Enter);
    app
}

fn real_wire_managed_detail_app() -> App {
    let mut app = real_wire_handlers_app("sessionStart");
    app.handle(Command::Enter);
    app
}

fn changes_values() -> Vec<HookInvocation> {
    const DAY: i64 = 24 * 60 * 60 * 1_000;
    (0..12)
        .map(|index| HookInvocation {
            source_key: "visual-fixture".into(),
            source_record_id: format!("change-{index}"),
            runtime: Runtime::Codex,
            evidence_kind: EvidenceKind::SyntheticFixture,
            evidence_generation: crate::domain::EvidenceGeneration::SyntheticFixture,
            coverage: EvidenceCoverage::Complete,
            handler: HandlerIdentity {
                key: "hk_visual_changes".into(),
                revision: if index < 6 { "r1" } else { "r2" }.into(),
                label: "Synthetic deployment stop hook".into(),
                source_kind: "synthetic_fixture".into(),
                event: HookEvent::Stop,
                matcher_identity: "synthetic".into(),
                structural_identity: "synthetic:changes".into(),
                execution_mode: ExecutionMode::Sync,
            },
            occurred_at_unix_ms: if index < 6 {
                CHANGE_NOW_UNIX_MS - 8 * DAY + i64::from(index)
            } else {
                CHANGE_NOW_UNIX_MS - DAY + i64::from(index)
            },
            terminal_status: if index < 6 {
                TerminalStatus::Completed
            } else {
                TerminalStatus::Failed
            },
            duration_ms: None,
            error_fingerprint: (index >= 6).then(|| "synthetic_failure".into()),
        })
        .collect()
}

fn changes_app() -> App {
    let mut app = App::from_report(synthetic_fixture_report(CHANGE_NOW_UNIX_MS));
    app.handle(Command::Down);
    app.handle(Command::Down);
    app.apply_changes(ChangesSnapshot::from_values(
        changes_values(),
        CHANGE_NOW_UNIX_MS,
        TimeWindow::Last7Days,
        EvidenceCoverage::Complete,
    ));
    app.handle(Command::Enter);
    app
}

fn failing_diagnostics() -> DiagnosticsReport {
    let mut diagnostics = DiagnosticsReport::empty(PRESENTATION_NOW_UNIX_MS);
    diagnostics.overall_status = DiagnosticStatus::Fail;
    diagnostics.checks = vec![DiagnosticCheck {
        id: DiagnosticCheckId::ReceiptIntegrity,
        status: DiagnosticStatus::Fail,
        facts: Vec::new(),
    }];
    diagnostics
}

fn diagnostics_app() -> App {
    let mut app = App::from_report(synthetic_fixture_report(PRESENTATION_NOW_UNIX_MS));
    app.apply_diagnostics(failing_diagnostics());
    for _ in 0..3 {
        app.handle(Command::Down);
    }
    app
}

fn settings_app() -> App {
    let mut app = App::from_report(synthetic_fixture_report(PRESENTATION_NOW_UNIX_MS));
    for _ in 0..4 {
        app.handle(Command::Down);
    }
    app
}

fn app_for(case: FrameCase) -> App {
    match case.scenario {
        Scenario::OverviewReady => ready_app(),
        Scenario::OverviewLoading => App::loading(TimeWindow::Last7Days),
        Scenario::OverviewErrorWithAcceptedData => {
            let mut app = ready_app();
            app.reject_refresh();
            app
        }
        Scenario::OverviewZeroTerminalSamples => App::from_report(instrumented_report(
            &[],
            PRESENTATION_NOW_UNIX_MS,
            TimeWindow::Last7Days,
            0,
            0,
        )),
        Scenario::HooksEventsReady => {
            let mut app = ready_app();
            enter_hooks_events(&mut app);
            app
        }
        Scenario::HooksEventsLoading => {
            let mut app = App::from_report(synthetic_fixture_report(PRESENTATION_NOW_UNIX_MS));
            enter_hooks_events(&mut app);
            app
        }
        Scenario::HooksEventsErrorWithAcceptedData => {
            let mut app = ready_app();
            enter_hooks_events(&mut app);
            app.reject_runtime_catalog();
            app
        }
        Scenario::HooksHandlersCommand => hooks_handlers_app("preToolUse"),
        Scenario::HooksHandlersNeedsReview => hooks_handlers_app("postToolUse"),
        Scenario::HooksHandlersManaged => hooks_handlers_app("subagentStart"),
        Scenario::HookDetailLongConfiguration => hook_detail_app(),
        Scenario::ChangesReady => changes_app(),
        Scenario::ChangesLoadingWithAcceptedData => {
            let mut app = changes_app();
            app.handle(Command::Refresh);
            app
        }
        Scenario::ChangesErrorWithAcceptedData => {
            let mut app = changes_app();
            app.reject_changes();
            app
        }
        Scenario::DiagnosticsReady => diagnostics_app(),
        Scenario::DiagnosticsLoadingWithAcceptedData => {
            let mut app = diagnostics_app();
            app.handle(Command::Refresh);
            app
        }
        Scenario::DiagnosticsErrorWithAcceptedData => {
            let mut app = diagnostics_app();
            app.reject_diagnostics();
            app
        }
        Scenario::SettingsReady => settings_app(),
        Scenario::RealWireEvents => real_wire_events_app(),
        Scenario::RealWireHandlersPreToolUse => real_wire_handlers_app("preToolUse"),
        Scenario::RealWireManagedDetail => real_wire_managed_detail_app(),
    }
}

fn language_state(locale: ResolvedLocale) -> LanguageState {
    LanguageState::resolve(
        match locale {
            ResolvedLocale::EnUs => InterfaceLanguage::EnUs,
            ResolvedLocale::ZhCn => InterfaceLanguage::ZhCn,
        },
        None,
        None,
        None,
    )
}

fn render_frame(case: FrameCase, app: &App) -> RenderedFrame {
    let backend = TestBackend::new(case.width, case.height);
    let mut terminal = Terminal::new(backend).expect("visual TestBackend must initialize");
    terminal
        .draw(|frame| {
            draw(
                frame,
                app,
                language_state(case.locale),
                Theme::default_color(),
            );
        })
        .expect("production TUI draw must fit the TestBackend");

    let buffer = terminal.backend().buffer();
    assert_eq!(buffer.area.width, case.width, "{} width", case.id);
    assert_eq!(buffer.area.height, case.height, "{} height", case.id);

    let mut plain_rows = Vec::with_capacity(usize::from(case.height));
    let mut selected_rows = BTreeSet::new();
    let mut row_terminal_cells = Vec::with_capacity(usize::from(case.height));
    for y in 0..case.height {
        let mut row = String::new();
        for x in 0..case.width {
            let index = usize::from(y) * usize::from(case.width) + usize::from(x);
            let cell = &buffer.content[index];
            row.push_str(cell.symbol());
            if cell.modifier.contains(Modifier::REVERSED) {
                selected_rows.insert(y);
            }
        }
        plain_rows.push(row);
        let mut terminal_x = 0;
        while terminal_x < usize::from(case.width) {
            let index = usize::from(y) * usize::from(case.width) + terminal_x;
            let symbol_width = UnicodeWidthStr::width(buffer.content[index].symbol()).max(1);
            terminal_x += symbol_width;
        }
        row_terminal_cells.push(terminal_x);
    }
    for (row_index, row) in plain_rows.iter().enumerate() {
        if content_segment(row).is_some_and(|content| content.trim_start().starts_with('>')) {
            selected_rows.insert(u16::try_from(row_index).expect("terminal row must fit u16"));
        }
    }

    let mut baseline = format!(
        "# hookstat-tui-frame-v1 id={} geometry={}x{} locale={}\n",
        case.id,
        case.width,
        case.height,
        case.locale_tag(),
    );
    for (row_index, row) in plain_rows.iter().enumerate() {
        baseline.push_str(&format!("{row_index:03}|{row}|\n"));
    }

    RenderedFrame {
        baseline,
        plain_text: plain_rows.join("\n"),
        selected_rows,
        cell_count: buffer.content.len(),
        row_terminal_cells,
    }
}

fn baseline_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tui_visual")
}

fn baseline_path(case: FrameCase) -> PathBuf {
    baseline_root().join(format!("{}.frame", case.id))
}

fn normalize_for_leak_check(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn content_segment(row: &str) -> Option<&str> {
    row.split_once("││")
        .map(|(_, content)| content.trim_end_matches('│'))
}

fn semantic_event_duplicate_failures(keys: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    keys.into_iter()
        .filter_map(|key| (!seen.insert(key.clone())).then_some(key))
        .collect()
}

fn event_display_identity_failures(app: &App) -> Vec<String> {
    let Some(catalog) = app.runtime_catalog() else {
        return Vec::new();
    };
    semantic_event_duplicate_failures(catalog.events.iter().map(|event| {
        let presentation = event.known_event.map_or_else(
            || format!("raw:{}", event.runtime_event_name),
            |known| format!("known:{known:?}"),
        );
        format!("{}::{presentation}", event.runtime_context)
    }))
}

fn known_event_english_leaks(frame: &str) -> Vec<String> {
    let normalized_frame = normalize_for_leak_check(frame);
    KNOWN_RUNTIME_EVENTS
        .into_iter()
        .flat_map(|event| {
            [
                known_runtime_event_name(ResolvedLocale::EnUs, event),
                known_runtime_event_description(ResolvedLocale::EnUs, event),
            ]
        })
        .filter(|english| normalized_frame.contains(&normalize_for_leak_check(english)))
        .map(str::to_owned)
        .collect()
}

fn event_display_count_failures(locale: ResolvedLocale, app: &App, frame: &str) -> Vec<String> {
    let Some(catalog) = app.runtime_catalog() else {
        return vec!["EVENT_DISPLAY_COUNT_CATALOG_MISSING=true".into()];
    };
    let content_rows = frame
        .lines()
        .filter_map(content_segment)
        .map(normalize_for_leak_check)
        .collect::<Vec<_>>();
    catalog
        .events
        .iter()
        .filter_map(|event| {
            let display_name = event.known_event.map_or_else(
                || event.runtime_event_name.as_str(),
                |known| known_runtime_event_name(locale, known),
            );
            let normalized_name = normalize_for_leak_check(display_name);
            let display_count = content_rows
                .iter()
                .filter(|row| row.starts_with(&normalized_name))
                .count();
            (display_count != 1).then(|| {
                format!(
                    "EVENT_DISPLAY_COUNT_{}={display_count} expected=1",
                    event.runtime_event_name
                )
            })
        })
        .collect()
}

fn runtime_event_display_count(
    locale: ResolvedLocale,
    app: &App,
    frame: &str,
    runtime_event_name: &str,
) -> usize {
    let event = app
        .runtime_catalog()
        .and_then(|catalog| {
            catalog
                .events
                .iter()
                .find(|event| event.runtime_event_name == runtime_event_name)
        })
        .unwrap_or_else(|| panic!("runtime event missing from visual app: {runtime_event_name}"));
    let display_name = event.known_event.map_or_else(
        || event.runtime_event_name.as_str(),
        |known| known_runtime_event_name(locale, known),
    );
    let normalized_name = normalize_for_leak_check(display_name);
    frame
        .lines()
        .filter_map(content_segment)
        .map(normalize_for_leak_check)
        .map(|row| row.trim_start_matches('>').trim_start().to_owned())
        .filter(|row| row.starts_with(&normalized_name))
        .count()
}

fn selected_event_display_count_failures(
    locale: ResolvedLocale,
    app: &App,
    frame: &str,
) -> Vec<String> {
    let Some(event) = app.selected_runtime_event() else {
        return vec!["SELECTED_EVENT_DISPLAY_COUNT_EVENT_MISSING=true".into()];
    };
    let display_name = event.known_event.map_or_else(
        || event.runtime_event_name.as_str(),
        |known| known_runtime_event_name(locale, known),
    );
    let normalized_name = normalize_for_leak_check(display_name);
    let display_count = frame
        .lines()
        .filter_map(content_segment)
        .map(normalize_for_leak_check)
        .map(|row| row.trim_start_matches('>').trim_start().to_owned())
        .filter(|row| row.starts_with(&normalized_name))
        .count();
    if display_count == 1 {
        Vec::new()
    } else {
        vec![format!(
            "SELECTED_EVENT_DISPLAY_COUNT_{}={display_count} expected=1",
            event.runtime_event_name
        )]
    }
}

fn visible_event_duplicate_failures(locale: ResolvedLocale, app: &App, frame: &str) -> Vec<String> {
    let Some(catalog) = app.runtime_catalog() else {
        return vec!["VISIBLE_EVENT_DUPLICATE_CATALOG_MISSING=true".into()];
    };
    let content_rows = frame
        .lines()
        .filter_map(content_segment)
        .map(normalize_for_leak_check)
        .map(|row| row.trim_start_matches('>').trim().to_owned())
        .collect::<Vec<_>>();
    catalog
        .events
        .iter()
        .filter_map(|event| {
            let display_name = event.known_event.map_or_else(
                || event.runtime_event_name.as_str(),
                |known| known_runtime_event_name(locale, known),
            );
            let normalized_name = normalize_for_leak_check(display_name);
            let display_count = content_rows
                .iter()
                .filter(|row| row.as_str() == normalized_name)
                .count();
            (display_count > 1).then(|| {
                format!(
                    "VISIBLE_EVENT_DISPLAY_DUPLICATE_{}={display_count} expected<=1",
                    event.runtime_event_name
                )
            })
        })
        .collect()
}

fn selected_row_failures(selected_rows: &BTreeSet<u16>) -> Vec<String> {
    if selected_rows.len() == 1 {
        Vec::new()
    } else {
        vec![format!(
            "SELECTED_ROW_COUNT={} expected=1 rows={selected_rows:?}",
            selected_rows.len()
        )]
    }
}

fn footer_visibility_failures(case: FrameCase, frame: &str) -> Vec<String> {
    let (action, key) = match case.scenario {
        Scenario::HookDetailLongConfiguration | Scenario::RealWireManagedDetail => {
            ("Esc", MessageKey::FooterBack)
        }
        Scenario::ChangesReady
        | Scenario::ChangesLoadingWithAcceptedData
        | Scenario::ChangesErrorWithAcceptedData => ("Enter", MessageKey::FooterOpen),
        _ => ("q", MessageKey::FooterQuit),
    };
    let footer = frame.lines().rev().take(2).collect::<Vec<_>>().join(" ");
    let normalized = normalize_for_leak_check(&footer);
    let action_visible = normalized.contains(&normalize_for_leak_check(action));
    let localized_action_visible =
        normalized.contains(&normalize_for_leak_check(t(case.locale, key)));
    if action_visible && localized_action_visible {
        Vec::new()
    } else {
        vec![format!(
            "FOOTER_VISIBLE=false action={action} localized={} excerpt={}",
            t(case.locale, key),
            compact_line(&footer)
        )]
    }
}

fn raw_unix_ms_failures(frame: &str) -> Vec<String> {
    [
        PRESENTATION_NOW_UNIX_MS,
        CATALOG_CAPTURED_AT_UNIX_MS,
        REAL_WIRE_CAPTURED_AT_UNIX_MS,
        CHANGE_NOW_UNIX_MS,
    ]
    .into_iter()
    .filter(|raw_unix_ms| frame.contains(&raw_unix_ms.to_string()))
    .map(|raw_unix_ms| format!("RAW_UNIX_MS_VISIBLE={raw_unix_ms}"))
    .collect()
}

fn zero_sample_failures(locale: ResolvedLocale, frame: &str) -> Vec<String> {
    let normalized = normalize_for_leak_check(frame);
    let mut failures = Vec::new();
    let false_metric = normalize_for_leak_check(&format!(
        "{}: 0.00%",
        t(locale, MessageKey::FieldFailureRate)
    ));
    if normalized.contains(&false_metric) {
        failures.push("ZERO_SAMPLE_FAILURE_RATE_FALSELY_NUMERIC=true".into());
    }
    for required in [
        MessageKey::FailureRateUnavailableZeroSamples,
        MessageKey::StatusNoTerminalSamples,
    ] {
        if !normalized.contains(&normalize_for_leak_check(t(locale, required))) {
            failures.push(format!("ZERO_SAMPLE_EXPLICIT_STATE_MISSING={required:?}"));
        }
    }
    failures
}

fn shell_partition_failures(terminal: Rect, areas: ShellAreas) -> Vec<String> {
    let right = |rect: Rect| u32::from(rect.x) + u32::from(rect.width);
    let bottom = |rect: Rect| u32::from(rect.y) + u32::from(rect.height);
    let terminal_right = right(terminal);
    let terminal_bottom = bottom(terminal);
    let inside = |rect: Rect| {
        rect.x >= terminal.x
            && rect.y >= terminal.y
            && right(rect) <= terminal_right
            && bottom(rect) <= terminal_bottom
    };
    let regions = [areas.title, areas.navigation, areas.content, areas.footer];
    let region_cells = regions
        .iter()
        .map(|rect| u32::from(rect.width) * u32::from(rect.height))
        .sum::<u32>();
    let terminal_cells = u32::from(terminal.width) * u32::from(terminal.height);
    let partitions_terminal = regions.iter().copied().all(inside)
        && regions.iter().all(|rect| rect.width > 0 && rect.height > 0)
        && areas.title.x == terminal.x
        && areas.title.y == terminal.y
        && areas.title.width == terminal.width
        && bottom(areas.title) == u32::from(areas.navigation.y)
        && areas.navigation.x == terminal.x
        && areas.navigation.y == areas.content.y
        && areas.navigation.height == areas.content.height
        && right(areas.navigation) == u32::from(areas.content.x)
        && right(areas.content) == terminal_right
        && bottom(areas.navigation) == u32::from(areas.footer.y)
        && bottom(areas.content) == u32::from(areas.footer.y)
        && areas.footer.x == terminal.x
        && areas.footer.width == terminal.width
        && bottom(areas.footer) == terminal_bottom
        && region_cells == terminal_cells;
    if partitions_terminal {
        Vec::new()
    } else {
        vec![format!(
            "SHELL_LAYOUT_OUT_OF_BOUNDS=true terminal={terminal:?} areas={areas:?} region_cells={region_cells} terminal_cells={terminal_cells}"
        )]
    }
}

fn geometry_failures(case: FrameCase, rendered: &RenderedFrame) -> Vec<String> {
    let expected_cells = usize::from(case.width) * usize::from(case.height);
    let terminal = Rect::new(0, 0, case.width, case.height);
    let mut failures = match ApplicationShell::new().layout(terminal) {
        ShellLayout::Ready(areas) => shell_partition_failures(terminal, areas),
        ShellLayout::TooSmall { available } => vec![format!(
            "SHELL_LAYOUT_UNEXPECTEDLY_TOO_SMALL=true available={available:?}"
        )],
    };
    let buffer_integrity = rendered.cell_count == expected_cells
        && rendered.baseline.lines().skip(1).count() == usize::from(case.height)
        && rendered.row_terminal_cells.len() == usize::from(case.height)
        && rendered
            .row_terminal_cells
            .iter()
            .all(|width| *width == usize::from(case.width));
    if buffer_integrity {
        failures
    } else {
        failures.push(format!(
            "OUT_OF_BOUNDS_RENDERING=true cells={} expected={expected_cells} rows={} expected_rows={} row_terminal_cells={:?} expected_width={}",
            rendered.cell_count,
            rendered.baseline.lines().skip(1).count(),
            case.height,
            rendered.row_terminal_cells,
            case.width
        ));
        failures
    }
}

fn validate_structural_invariants(
    case: FrameCase,
    app: &App,
    rendered: &RenderedFrame,
) -> Vec<String> {
    let mut failures = Vec::new();
    let duplicates = event_display_identity_failures(app);
    if !duplicates.is_empty() {
        failures.push(format!(
            "EVENT_DISPLAY_IDENTITY_DUPLICATES={}",
            duplicates.join(",")
        ));
    }
    if case.event_display_count_invariant {
        failures.extend(event_display_count_failures(
            case.locale,
            app,
            &rendered.plain_text,
        ));
    }
    if case.selected_event_display_count_invariant {
        failures.extend(selected_event_display_count_failures(
            case.locale,
            app,
            &rendered.plain_text,
        ));
    }
    if case.visible_event_duplicate_invariant {
        failures.extend(visible_event_duplicate_failures(
            case.locale,
            app,
            &rendered.plain_text,
        ));
    }
    if case.selected_row_invariant {
        failures.extend(selected_row_failures(&rendered.selected_rows));
    }
    failures.extend(footer_visibility_failures(case, &rendered.plain_text));
    failures.extend(raw_unix_ms_failures(&rendered.plain_text));
    if case.locale == ResolvedLocale::ZhCn
        && matches!(
            case.scenario,
            Scenario::HooksEventsReady
                | Scenario::HooksEventsLoading
                | Scenario::HooksEventsErrorWithAcceptedData
                | Scenario::HooksHandlersCommand
                | Scenario::HooksHandlersNeedsReview
                | Scenario::HooksHandlersManaged
                | Scenario::HookDetailLongConfiguration
                | Scenario::RealWireEvents
                | Scenario::RealWireHandlersPreToolUse
                | Scenario::RealWireManagedDetail
        )
    {
        let leaks = known_event_english_leaks(&rendered.plain_text);
        if !leaks.is_empty() {
            failures.push(format!(
                "KNOWN_EVENT_ENGLISH_LEAK_IN_ZH_CN={}",
                leaks.join(",")
            ));
        }
    }
    if case.runtime_before_reliability {
        let normalized = normalize_for_leak_check(&rendered.plain_text);
        let runtime =
            normalize_for_leak_check(t(case.locale, MessageKey::SectionRuntimeConfiguration));
        let reliability =
            normalize_for_leak_check(t(case.locale, MessageKey::SectionReliabilitySummary));
        match (normalized.find(&runtime), normalized.find(&reliability)) {
            (Some(runtime_index), Some(reliability_index)) if runtime_index < reliability_index => {
            }
            _ => failures.push("CURRENT_RUNTIME_SECTION_PRECEDES_RELIABILITY=false".into()),
        }
    }
    if case.zero_sample_invariant {
        failures.extend(zero_sample_failures(case.locale, &rendered.plain_text));
    }
    failures.extend(geometry_failures(case, rendered));
    failures
}

fn compact_line(value: &str) -> String {
    let mut clipped = value.chars().take(180).collect::<String>();
    if value.chars().count() > 180 {
        clipped.push('…');
    }
    clipped
}

fn bounded_frame_diff(expected: &str, actual: &str) -> String {
    let expected_lines = expected.lines().collect::<Vec<_>>();
    let actual_lines = actual.lines().collect::<Vec<_>>();
    let mut differences = Vec::new();
    for index in 0..expected_lines.len().max(actual_lines.len()) {
        let expected = expected_lines.get(index).copied().unwrap_or("<missing>");
        let actual = actual_lines.get(index).copied().unwrap_or("<missing>");
        if expected != actual {
            differences.push(format!(
                "line {}\n- {}\n+ {}",
                index + 1,
                compact_line(expected),
                compact_line(actual),
            ));
            if differences.len() == 12 {
                differences.push("... diff truncated after 12 changed lines".into());
                break;
            }
        }
    }
    differences.join("\n")
}

fn compare_baseline(case: FrameCase, actual: &str) -> Result<(), String> {
    let path = baseline_path(case);
    let expected = fs::read_to_string(&path).map_err(|error| {
        format!(
            "baseline={} geometry={}x{} locale={} missing_or_unreadable={error}",
            case.id,
            case.width,
            case.height,
            case.locale_tag(),
        )
    })?;
    if expected == actual {
        Ok(())
    } else {
        Err(format!(
            "baseline={} geometry={}x{} locale={}\n{}",
            case.id,
            case.width,
            case.height,
            case.locale_tag(),
            bounded_frame_diff(&expected, actual),
        ))
    }
}

fn canonical_baseline_inventory_failures(cases: &[FrameCase]) -> Vec<String> {
    let expected = cases
        .iter()
        .map(|case| format!("{}.frame", case.id))
        .collect::<BTreeSet<_>>();
    let actual = match fs::read_dir(baseline_root()) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| name.ends_with(".frame"))
            .collect::<BTreeSet<_>>(),
        Err(error) => return vec![format!("baseline inventory unreadable: {error}")],
    };
    expected.symmetric_difference(&actual).cloned().collect()
}

fn write_baseline_if_changed(path: &Path, content: &str) -> std::io::Result<bool> {
    if fs::read_to_string(path).is_ok_and(|existing| existing == content) {
        return Ok(false);
    }
    fs::write(path, content)?;
    Ok(true)
}

#[test]
fn tui_visual_canonical_frames_match() {
    let cases = canonical_cases();
    assert_eq!(cases.len(), 30, "CANONICAL_FRAME_COUNT must remain bounded");
    let mut failures = canonical_baseline_inventory_failures(&cases);
    for case in cases {
        let app = app_for(case);
        let rendered = render_frame(case, &app);
        failures.extend(
            validate_structural_invariants(case, &app, &rendered)
                .into_iter()
                .map(|failure| format!("{}: {failure}", case.id)),
        );
        if let Err(failure) = compare_baseline(case, &rendered.baseline) {
            failures.push(failure);
        }
    }
    assert!(
        failures.is_empty(),
        "TUI visual regression failures:\n{}",
        failures.join("\n\n")
    );
}

#[test]
#[ignore = "explicit baseline update only; use scripts/tui/update-visual-baselines.ps1"]
fn update_tui_visual_baselines() {
    assert_eq!(
        std::env::var(BASELINE_UPDATE_ENV).as_deref(),
        Ok("1"),
        "refusing baseline mutation without {BASELINE_UPDATE_ENV}=1"
    );
    let cases = canonical_cases();
    let rendered_cases = cases
        .iter()
        .copied()
        .map(|case| {
            let app = app_for(case);
            let rendered = render_frame(case, &app);
            let failures = validate_structural_invariants(case, &app, &rendered);
            assert!(
                failures.is_empty(),
                "refusing to bless structurally invalid baseline {}: {}",
                case.id,
                failures.join(", ")
            );
            (case, rendered)
        })
        .collect::<Vec<_>>();
    fs::create_dir_all(baseline_root()).expect("visual baseline directory must be creatable");
    for (case, rendered) in rendered_cases {
        let path = baseline_path(case);
        if write_baseline_if_changed(&path, &rendered.baseline)
            .expect("intended visual baseline must be writable")
        {
            println!("UPDATED={}", path.display());
        } else {
            println!("UNCHANGED={}", path.display());
        }
    }
    println!("CANONICAL_FRAME_COUNT=30");
}

#[test]
fn tui_visual_harness_fails_closed_on_changes_and_missing_baselines() {
    let diff = bounded_frame_diff("alpha\nbeta\n", "alpha\ngamma\n");
    assert!(diff.contains("- beta"));
    assert!(diff.contains("+ gamma"));

    let missing = FrameCase::new(
        "missing-baseline-contract-probe",
        44,
        44,
        ResolvedLocale::EnUs,
        Scenario::OverviewReady,
    );
    assert!(compare_baseline(missing, "probe").is_err());
}

#[test]
fn tui_visual_duplicate_and_selection_invariants_reject_crafted_defects() {
    let duplicates = semantic_event_duplicate_failures([
        "context-a::known:PreToolUse".to_owned(),
        "context-a::known:PreToolUse".to_owned(),
    ]);
    assert_eq!(duplicates, ["context-a::known:PreToolUse"]);

    let app = ready_app();
    let clean_rows = app
        .runtime_catalog()
        .expect("ready fixture has runtime catalog")
        .events
        .iter()
        .map(|event| {
            let display_name = event.known_event.map_or_else(
                || event.runtime_event_name.as_str(),
                |known| known_runtime_event_name(ResolvedLocale::EnUs, known),
            );
            format!("││{display_name}    │")
        })
        .collect::<Vec<_>>();
    assert!(
        event_display_count_failures(ResolvedLocale::EnUs, &app, &clean_rows.join("\n")).is_empty()
    );
    let mut duplicated_rows = clean_rows;
    duplicated_rows.push(duplicated_rows[0].clone());
    let duplicate_name = app
        .runtime_catalog()
        .and_then(|catalog| catalog.events.first())
        .expect("ready fixture has at least one event")
        .runtime_event_name
        .clone();
    let rendered = RenderedFrame {
        baseline: "# malformed geometry\n".into(),
        plain_text: duplicated_rows.join("\n"),
        selected_rows: BTreeSet::from([7_u16, 9_u16]),
        cell_count: 1,
        row_terminal_cells: vec![1],
    };
    let case = FrameCase::new(
        "crafted-duplicate-selection-probe",
        140,
        58,
        ResolvedLocale::EnUs,
        Scenario::HooksEventsReady,
    )
    .selected_row()
    .event_display_counts();
    let failures = validate_structural_invariants(case, &app, &rendered);
    assert!(failures.iter().any(|failure| {
        failure.starts_with(&format!("EVENT_DISPLAY_COUNT_{duplicate_name}=2"))
    }));
    assert!(
        failures
            .iter()
            .any(|failure| failure.starts_with("SELECTED_ROW_COUNT=2"))
    );

    let mut compact_app = ready_app();
    enter_hooks_events(&mut compact_app);
    let selected = compact_app
        .selected_runtime_event()
        .expect("compact fixture has a selected event");
    let selected_display = selected.known_event.map_or_else(
        || selected.runtime_event_name.as_str(),
        |known| known_runtime_event_name(ResolvedLocale::ZhCn, known),
    );
    let compact_rendered = RenderedFrame {
        baseline: "# malformed geometry\n".into(),
        plain_text: format!("││{selected_display} │\n││{selected_display} │"),
        selected_rows: BTreeSet::new(),
        cell_count: 1,
        row_terminal_cells: vec![1],
    };
    let compact_case = FrameCase::new(
        "crafted-compact-duplicate-probe",
        100,
        32,
        ResolvedLocale::ZhCn,
        Scenario::HooksEventsReady,
    )
    .visible_event_duplicates()
    .selected_event_display_count();
    let compact_failures =
        validate_structural_invariants(compact_case, &compact_app, &compact_rendered);
    assert!(compact_failures.iter().any(|failure| {
        failure.starts_with("SELECTED_EVENT_DISPLAY_COUNT_") && failure.contains("=2")
    }));
    assert!(compact_failures.iter().any(|failure| {
        failure.starts_with("VISIBLE_EVENT_DISPLAY_DUPLICATE_") && failure.contains("=2")
    }));
}

#[test]
fn tui_visual_structural_guards_reject_crafted_frame_defects() {
    let case = FrameCase::new(
        "crafted-defect-probe",
        44,
        44,
        ResolvedLocale::EnUs,
        Scenario::OverviewZeroTerminalSamples,
    )
    .zero_samples();

    let malformed = RenderedFrame {
        baseline: "# header\n000|short|\n".into(),
        plain_text: format!(
            "no footer actions here\nFailure rate: 0.00%\n{PRESENTATION_NOW_UNIX_MS}"
        ),
        selected_rows: BTreeSet::new(),
        cell_count: 1,
        row_terminal_cells: vec![1],
    };
    let failures = validate_structural_invariants(case, &ready_app(), &malformed);
    for expected in [
        "FOOTER_VISIBLE=false",
        "RAW_UNIX_MS_VISIBLE=",
        "ZERO_SAMPLE_FAILURE_RATE_FALSELY_NUMERIC=true",
        "ZERO_SAMPLE_EXPLICIT_STATE_MISSING=",
        "OUT_OF_BOUNDS_RENDERING=true",
    ] {
        assert!(
            failures.iter().any(|failure| failure.starts_with(expected)),
            "integrated structural validator missed {expected}: {failures:?}"
        );
    }
    let invalid_shell = ShellAreas {
        title: Rect::new(0, 0, 44, 2),
        navigation: Rect::new(0, 2, 20, 40),
        content: Rect::new(20, 2, 30, 40),
        footer: Rect::new(0, 42, 44, 2),
    };
    assert!(
        shell_partition_failures(Rect::new(0, 0, 44, 44), invalid_shell)
            .iter()
            .any(|failure| failure.starts_with("SHELL_LAYOUT_OUT_OF_BOUNDS=true"))
    );
}

#[test]
fn tui_visual_baseline_writer_changes_only_intended_content() {
    let directory = tempfile::tempdir().expect("temporary baseline directory");
    let baseline = directory.path().join("probe.frame");
    assert!(write_baseline_if_changed(&baseline, "frame-v1").expect("initial baseline write"));
    assert!(!write_baseline_if_changed(&baseline, "frame-v1").expect("stable baseline check"));
    assert_eq!(fs::read_to_string(&baseline).unwrap(), "frame-v1");
    assert!(write_baseline_if_changed(&baseline, "frame-v2").expect("updated baseline write"));
    assert_eq!(fs::read_to_string(&baseline).unwrap(), "frame-v2");
}

#[test]
fn tui_visual_zh_cn_leak_invariant_rejects_crafted_english_semantics() {
    let mut app = ready_app();
    enter_hooks_events(&mut app);
    let rendered = RenderedFrame {
        baseline: "# malformed geometry\n".into(),
        plain_text: "Before a tool executes".into(),
        selected_rows: BTreeSet::new(),
        cell_count: 1,
        row_terminal_cells: vec![1],
    };
    let case = FrameCase::new(
        "crafted-zh-cn-leak-probe",
        100,
        32,
        ResolvedLocale::ZhCn,
        Scenario::HooksEventsReady,
    );
    let failures = validate_structural_invariants(case, &app, &rendered);
    assert!(
        failures
            .iter()
            .any(|failure| failure.starts_with("KNOWN_EVENT_ENGLISH_LEAK_IN_ZH_CN="))
    );
}

#[test]
fn codex_v0151_real_wire_contract_is_pinned_offline_and_deliberate() {
    let contract: Value = serde_json::from_str(REAL_WIRE_CONTRACT_JSON)
        .expect("committed Codex v0.151.0 wire contract must be valid JSON");
    assert_eq!(contract["contractVersion"], 1);
    assert_eq!(contract["codexVersion"], "0.151.0");
    assert_eq!(contract["codexTag"], "rust-v0.151.0");
    assert_eq!(
        contract["codexSourceCommit"],
        "78c290807ce710180111df227df3b7a4fe845452"
    );
    assert_eq!(contract["fixture"], "hooks-list.json");

    let pinned_wire_names = contract["knownWireEventNames"]
        .as_array()
        .expect("wire contract must list known event names")
        .iter()
        .map(|value| value.as_str().expect("wire name must be text"))
        .collect::<Vec<_>>();
    let product_wire_names = KNOWN_RUNTIME_EVENTS
        .into_iter()
        .map(KnownRuntimeEvent::wire_name)
        .collect::<Vec<_>>();
    assert_eq!(pinned_wire_names, product_wire_names);

    let fixture = real_wire_fixture();
    let hooks = fixture["result"]["data"][0]["hooks"]
        .as_array()
        .expect("official-shaped fixture must contain hooks");
    let raw_event_names = hooks
        .iter()
        .map(|hook| {
            hook["eventName"]
                .as_str()
                .expect("fixture eventName must be text")
        })
        .collect::<Vec<_>>();
    for required in ["preToolUse", "permissionRequest", "interrupt"] {
        assert!(
            raw_event_names.contains(&required),
            "pinned fixture missing exact wire event {required}"
        );
    }
    for raw_name in raw_event_names {
        if let Some(known) = KnownRuntimeEvent::from_codex_wire_name(raw_name) {
            assert_eq!(
                raw_name,
                known.wire_name(),
                "official-shaped fixture must use exact v0.151.0 camelCase"
            );
        }
    }

    let affected_frames = contract["affectedGoldenFrames"]
        .as_array()
        .expect("wire contract must list affected frames")
        .iter()
        .map(|value| value.as_str().expect("frame id must be text"))
        .collect::<BTreeSet<_>>();
    let committed_real_wire_cases = canonical_cases()
        .into_iter()
        .filter(|case| case.id.starts_with("real-wire-"))
        .map(|case| case.id)
        .collect::<BTreeSet<_>>();
    assert_eq!(affected_frames, committed_real_wire_cases);
    assert_eq!(
        contract["deliberateUpdateRequirements"],
        json!([
            "sourcePin",
            "wireFixture",
            "knownMapping",
            "affectedGoldenFrames"
        ])
    );
    assert!(
        !REAL_WIRE_FIXTURE_JSON.contains("http://") && !REAL_WIRE_FIXTURE_JSON.contains("https://"),
        "ordinary visual CI must consume only the committed fixture"
    );
}

#[test]
fn codex_v0151_real_wire_reaches_events_handlers_detail_and_whole_frames() {
    let fixture = real_wire_fixture();
    let fixture_handlers = fixture["result"]["data"][0]["hooks"]
        .as_array()
        .expect("official-shaped fixture must contain hooks");
    let catalog =
        RuntimePresentationSnapshot::from_codex_hooks_list(&fixture, REAL_WIRE_CAPTURED_AT_UNIX_MS)
            .expect("official-shaped fixture must parse through the product boundary");
    let event = |name: &str| {
        catalog
            .events
            .iter()
            .find(|event| event.runtime_event_name == name)
            .unwrap_or_else(|| panic!("parsed event missing: {name}"))
    };

    let pre_tool_use = event("preToolUse");
    assert_eq!(pre_tool_use.installed_count(), 2);
    assert_eq!(pre_tool_use.active_count(), 1);
    assert_eq!(pre_tool_use.needs_review_count(), 1);
    assert!(matches!(
        &pre_tool_use.handlers[0].handler_kind,
        RuntimeHandlerKind::Command { command }
            if command.starts_with("fixture-command --safe-long-argument=")
    ));
    assert!(matches!(
        &pre_tool_use.handlers[1].handler_kind,
        RuntimeHandlerKind::McpTool { server, tool }
            if server == "fixture-server" && tool == "fixture-tool"
    ));
    assert!(!pre_tool_use.handlers[1].enabled);
    assert!(pre_tool_use.handlers[1].needs_review);

    let permission = event("permissionRequest");
    assert!(matches!(
        permission.handlers[0].handler_kind,
        RuntimeHandlerKind::Prompt
    ));
    assert_eq!(permission.needs_review_count(), 1);
    let managed = event("sessionStart");
    assert!(matches!(
        managed.handlers[0].handler_kind,
        RuntimeHandlerKind::Agent
    ));
    assert!(managed.handlers[0].managed);
    assert!(managed.handlers[0].enabled);

    let interrupt = event("interrupt");
    assert_eq!(interrupt.known_event, Some(KnownRuntimeEvent::Interrupt));
    assert_eq!(interrupt.canonical_event, None);
    let future = event("futureCodexEventV2");
    assert_eq!(future.known_event, None);
    assert_eq!(future.canonical_event, None);
    assert_eq!(future.runtime_event_name, "futureCodexEventV2");
    assert_eq!(
        future.description.as_deref(),
        Some("Synthetic future event supplied verbatim by the fixture.")
    );

    assert_eq!(
        catalog
            .events
            .iter()
            .map(|event| event.installed_count())
            .sum::<usize>(),
        fixture_handlers.len(),
        "INSTALLED_COUNT_MATCHES_FIXTURE"
    );
    assert_eq!(
        catalog
            .events
            .iter()
            .map(|event| event.active_count())
            .sum::<usize>(),
        4,
        "ACTIVE_COUNT_MATCHES_FIXTURE"
    );
    assert_eq!(
        catalog
            .events
            .iter()
            .map(|event| event.needs_review_count())
            .sum::<usize>(),
        2,
        "REVIEW_COUNT_MATCHES_FIXTURE"
    );
    assert_eq!(catalog.issues.len(), 2);
    assert!(catalog.events.iter().all(|event| {
        event.runtime_context == "C:/hookstat-fixtures/codex-v0.151/project-alpha"
    }));

    let mut two_context_fixture = fixture.clone();
    let mut second_context = fixture["result"]["data"][0].clone();
    second_context["cwd"] = json!("C:/hookstat-fixtures/codex-v0.151/project-beta");
    second_context["warnings"] = json!([]);
    second_context["errors"] = json!([]);
    let mut second_handler = fixture_handlers[0].clone();
    second_handler["key"] = json!("fixture-beta:0:0");
    second_handler["sourcePath"] =
        json!("C:/hookstat-fixtures/codex-v0.151/project-beta/.codex/hooks.json");
    second_context["hooks"] = json!([second_handler]);
    two_context_fixture["result"]["data"]
        .as_array_mut()
        .expect("fixture data must be mutable")
        .push(second_context);
    let two_context_catalog = RuntimePresentationSnapshot::from_codex_hooks_list(
        &two_context_fixture,
        REAL_WIRE_CAPTURED_AT_UNIX_MS,
    )
    .expect("two official-shaped contexts must parse");
    let pre_tool_use_contexts = two_context_catalog
        .events
        .iter()
        .filter(|event| event.runtime_event_name == "preToolUse")
        .map(|event| event.runtime_context.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        pre_tool_use_contexts,
        BTreeSet::from([
            "C:/hookstat-fixtures/codex-v0.151/project-alpha",
            "C:/hookstat-fixtures/codex-v0.151/project-beta",
        ]),
        "CROSS_CONTEXT_SILENT_MERGE=false"
    );

    let cases = canonical_cases();
    for case_id in [
        "real-wire-events-wide-en-us",
        "real-wire-events-wide-zh-cn",
        "real-wire-handlers-narrow-zh-cn",
        "real-wire-detail-wide-en-us",
    ] {
        let case = *cases
            .iter()
            .find(|case| case.id == case_id)
            .unwrap_or_else(|| panic!("real-wire case missing: {case_id}"));
        let app = app_for(case);
        let rendered = render_frame(case, &app);
        assert!(
            validate_structural_invariants(case, &app, &rendered).is_empty(),
            "real-wire structural invariants failed for {case_id}"
        );
        compare_baseline(case, &rendered.baseline).unwrap_or_else(|failure| panic!("{failure}"));
    }

    let en_case = *cases
        .iter()
        .find(|case| case.id == "real-wire-events-wide-en-us")
        .unwrap();
    let en_app = app_for(en_case);
    let en_frame = render_frame(en_case, &en_app);
    assert_eq!(
        runtime_event_display_count(
            ResolvedLocale::EnUs,
            &en_app,
            &en_frame.plain_text,
            "preToolUse"
        ),
        1,
        "PRE_TOOL_USE_DISPLAY_COUNT"
    );
    assert_eq!(
        runtime_event_display_count(
            ResolvedLocale::EnUs,
            &en_app,
            &en_frame.plain_text,
            "interrupt"
        ),
        1,
        "INTERRUPT_DISPLAY_COUNT"
    );
    assert_eq!(
        runtime_event_display_count(
            ResolvedLocale::EnUs,
            &en_app,
            &en_frame.plain_text,
            "futureCodexEventV2"
        ),
        1,
        "UNKNOWN_EVENT_DISPLAY_COUNT"
    );
    assert!(
        normalize_for_leak_check(&en_frame.plain_text).contains(&normalize_for_leak_check(
            known_runtime_event_description(ResolvedLocale::EnUs, KnownRuntimeEvent::PreToolUse)
        ))
    );

    let zh_case = *cases
        .iter()
        .find(|case| case.id == "real-wire-events-wide-zh-cn")
        .unwrap();
    let zh_app = app_for(zh_case);
    let zh_frame = render_frame(zh_case, &zh_app);
    assert!(
        normalize_for_leak_check(&zh_frame.plain_text).contains(&normalize_for_leak_check(
            known_runtime_event_description(ResolvedLocale::ZhCn, KnownRuntimeEvent::PreToolUse)
        ))
    );
    assert!(known_event_english_leaks(&zh_frame.plain_text).is_empty());

    let mut navigation = real_wire_ready_app();
    enter_hooks_events(&mut navigation);
    assert!(navigation.hooks_events_active());
    select_runtime_event(&mut navigation, "preToolUse");
    navigation.handle(Command::Enter);
    assert!(navigation.hooks_handlers_active());
    assert_eq!(
        navigation
            .selected_runtime_event()
            .expect("preToolUse must remain selected")
            .handlers
            .len(),
        2
    );
    navigation.handle(Command::Enter);
    assert!(navigation.runtime_hook_detail_active());
    assert!(navigation.selected_runtime_handler().is_some());

    let managed_detail = real_wire_managed_detail_app();
    assert!(managed_detail.runtime_hook_detail_active());
    assert!(
        managed_detail
            .selected_runtime_handler()
            .expect("managed detail must retain selected handler")
            .managed
    );
    let detail_case = *cases
        .iter()
        .find(|case| case.id == "real-wire-detail-wide-en-us")
        .unwrap();
    let detail_frame = render_frame(detail_case, &managed_detail);
    let detail = normalize_for_leak_check(&detail_frame.plain_text);
    let section_positions = [
        MessageKey::SectionRuntimeConfiguration,
        MessageKey::SectionReliabilitySummary,
        MessageKey::SectionObservationHistory,
        MessageKey::SectionIntelligence,
    ]
    .map(|key| {
        detail
            .find(&normalize_for_leak_check(t(ResolvedLocale::EnUs, key)))
            .unwrap_or_else(|| panic!("detail section missing: {key:?}"))
    });
    assert!(section_positions.windows(2).all(|pair| pair[0] < pair[1]));

    let zero_case = *cases
        .iter()
        .find(|case| case.id == "overview-zero-samples-standard-en-us")
        .unwrap();
    let zero_app = real_wire_zero_sample_app();
    let zero_frame = render_frame(zero_case, &zero_app);
    assert!(
        validate_structural_invariants(zero_case, &zero_app, &zero_frame).is_empty(),
        "real-wire zero-sample structural contract failed"
    );
    compare_baseline(zero_case, &zero_frame.baseline).unwrap_or_else(|failure| panic!("{failure}"));
    assert!(
        normalize_for_leak_check(&zero_frame.plain_text).contains(&normalize_for_leak_check(t(
            ResolvedLocale::EnUs,
            MessageKey::FieldMetricScope
        )))
    );
}

#[test]
fn tui_visual_fixture_is_sanitized_and_uses_exact_wire_case() {
    let serialized = runtime_catalog_fixture().to_string();
    assert!(serialized.contains("\"eventName\":\"preToolUse\""));
    assert!(serialized.contains("\"eventName\":\"interrupt\""));
    for forbidden in [
        "JerrySkywalker",
        "C:/Users/",
        "C:\\\\Users\\\\",
        "V:/src/",
        "V:\\\\src\\\\",
        "raw prompt",
        "tool payload",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "visual fixture contains forbidden private marker {forbidden}"
        );
    }

    let real_wire = format!("{REAL_WIRE_FIXTURE_JSON}\n{REAL_WIRE_CONTRACT_JSON}");
    for forbidden in [
        "JerrySkywalker",
        "C:/Users/",
        "C:\\\\Users\\\\",
        "V:/src/",
        "V:\\\\src\\\\",
        "raw prompt",
        "tool payload",
        "currentHash",
        "github_pat_",
        "ghp_",
    ] {
        assert!(
            !real_wire.contains(forbidden),
            "real-wire fixture contains forbidden private marker {forbidden}"
        );
    }
}
