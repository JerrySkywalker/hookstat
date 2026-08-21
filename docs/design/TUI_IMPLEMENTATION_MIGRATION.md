# HookStat TUI Implementation Migration

Status: implemented foundation plan for `HS-V02-G00`.

## Current architecture audit

Before this goal, HookStat's interactive terminal experience is a single
`src/tui.rs` module:

```text
main::tui_command
  -> tui::run_with_refresh
    -> raw mode + alternate-screen setup
    -> synchronous event loop
      -> draw() recomputes MachineReport from HookInvocation values
      -> key handling mutates Home/Detail state directly
      -> r invokes SQLite/receipt refresh directly on the event-loop thread
    -> manual terminal cleanup
```

The module contains the rendering entry point, application state, keyboard
mapping, Home/Detail widgets, direct Ratatui colors, resize branches, refresh
callback, and terminal lifecycle. The report itself remains evidence-source
neutral, but its aggregation is invoked from the renderer. That couples
interaction timing to analytics work and leaves no reusable shell, route, or
locale boundary.

The v0.1 Home and Detail output is retained only as a compatibility content
view during G00. It is not the v0.2 Overview or Hook Detail redesign; those
views remain G01 work.

## Migration target

```text
analytics/domain
  -> MachineReport and typed UI state
  -> tui::app / tui::state / tui::navigation
  -> tui::rendering / tui::widgets
  -> Ratatui + Crossterm
```

```text
current HookStat TUI
        ↓
migration target
        ↓
Jerry Terminal UI System
```

The foundation separates the following responsibilities:

| Current concern | G00 owner |
| --- | --- |
| state and v0.1 compatibility selection | `tui::app`, `tui::state` |
| typed press-only commands | `tui::keymap`, `tui::navigation` |
| semantic color and typography mapping | `tui::theme` |
| responsive title/navigation/content/footer geometry | `tui::layout` |
| loading, empty, stale-error, and placeholder containers | `tui::widgets` |
| pure frame rendering | `tui::rendering` |
| coalesced background refresh and stale-result rejection | `tui::refresh` |
| RAII raw-mode/alternate-screen restoration | `tui::terminal` |
| typed locale keys and en-US/zh-CN fallback catalog | `tui::localization` |

## Explicit boundaries

- `app`, `state`, `navigation`, and `refresh` do not import Ratatui widgets.
- Rendering accepts an accepted in-memory `MachineReport`; it never reads
  SQLite, scans receipts, runs analytics, or waits on a channel.
- Refresh work owns the source callback and computes the replacement report
  off the event-loop thread. A failed refresh retains the last accepted view.
- Theme consumers request semantic tokens; terminal colors are confined to
  `tui::theme`.
- The placeholder route registry includes Overview, Hooks, Trends,
  Diagnostics, and Settings, but G00 intentionally implements none of their
  business content.
- Locale infrastructure is limited to shared shell/state text. G03 owns full
  catalog migration, preference persistence, and runtime language switching.

## Dependency decision

ADR 0004 selects Option B: keep this as HookStat's internal `src/tui/`
module. G00 adds no workspace or `jerry-terminal-ui` crate. Extraction is
reconsidered only after two conforming consumers prove a generic API and a
shared release policy.
