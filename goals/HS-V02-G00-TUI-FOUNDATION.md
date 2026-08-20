# HS-V02-G00 — Terminal UI Foundation

## Status

PLANNED after the v0.2 design foundation train.

## Objective

Create HookStat's internal implementation of the Jerry Terminal UI System so later Reliability Center views use the same framework, layout, theme, navigation, localization boundary, terminal lifecycle, and refresh contract as TabBeacon-compatible tools.

## Scope

- Refactor the single `src/tui.rs` boundary into coherent internal modules without changing reliability semantics.
- Keep Ratatui + Crossterm and select a tested compatible version pair; do not change versions only for visual parity.
- Add the application-title / navigation / content / shortcut-footer shell.
- Add semantic theme and typography roles, including monochrome/no-color behavior.
- Add typed commands for Up/Down, Enter, Back, Refresh, Search, Filter, Quit, and `j/k` aliases.
- Process ordinary navigation on key press only.
- Add an RAII terminal guard with partial-entry and all-cleanup-path tests.
- Add generation-tagged, coalesced, off-thread refresh infrastructure; no receipt scan, SQLite access, or aggregation may run in render/key handling.
- Establish typed locale/catalog interfaces with representative `en-US` and `zh-CN` shell strings; G03 owns full translation and persistence.
- Add display-cell/grapheme-safe width primitives and normal/narrow/minimum-size render fixtures.
- Keep a compatibility path for existing v0.1 report semantics while G01 replaces the views.

## Non-goals

- Do not implement the full Reliability Center views.
- Do not add human hook identity/schema migrations.
- Do not complete all translations or persist Interface preferences.
- Do not add diagnostics, risk scoring, trend analysis, or revision comparison.
- Do not change Codex instrumentation, trust, `hooks.json`, receipts, proxy behavior, or ledger schema.
- Do not create or publish an external `jerry-terminal-ui` crate.
- Do not add Tokio, a daemon, network telemetry, or a launcher wrapper.

## Acceptance criteria

```text
TUI_FRAMEWORK=ratatui+crossterm
TUI_FRAMEWORK_SHARED=true
THEME_SYSTEM_SHARED=true
LAYOUT_SYSTEM_SHARED=true
NAVIGATION_MODEL_SHARED=true
APPLICATION_SHELL=PASS
THEME_SYSTEM=PASS
TYPOGRAPHY_ROLES=PASS
NAVIGATION_MODEL=PASS
PRESS_ONLY_NAVIGATION=true
TERMINAL_GUARD=PASS
DATABASE_IN_RENDER=false
ASYNC_REFRESH=PASS
STALE_REFRESH_REJECTED=true
REFRESH_COALESCED=true
LOADING_EMPTY_ERROR_STATES=PASS
RESIZE_REFLOW=PASS
NORMAL_NARROW_MINIMUM_BUFFERS=PASS
MONOCHROME_SEMANTICS=PASS
LOCALE_INTERFACE=PASS
EXTERNAL_SHARED_CRATE=false
V01_RELIABILITY_SEMANTICS_UNCHANGED=true
```

Required local gate: format, Clippy warnings-as-errors, all tests, locked build, plus focused render/refresh/terminal-guard tests. One hosted Windows/Linux code CI run binds to the settled candidate. A real Windows Terminal smoke is required if visible or lifecycle behavior changes.

## Dependencies

- `docs/design/TUI_ALIGNMENT_AUDIT.md`
- `docs/design/JERRY_TERMINAL_UI_SYSTEM.md`
- `docs/design/HOOKSTAT_V02_VIEW_MODEL.md`
- `docs/design/I18N_DESIGN.md`
- `docs/adr/0004-terminal-ui-system-strategy.md`
- Accepted HookStat v0.1 behavior at tag `v0.1.0`

## Next

`HS-V02-G01 — Reliability Center TUI`.
