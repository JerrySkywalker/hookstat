# Terminal UI Contract System

Status: implemented shared contract for HookStat v0.3 and TabBeacon. The
dependency-neutral implementation is `JerrySkywalker/terminal-ui-contract` at
the released `0.1.0` binding recorded in `HS-G21-SHARED-BOUNDARY.md`.

This contract captures the common terminal experience. It does not claim that every current TabBeacon component is already packaged for reuse. Applications may implement the contract internally until two conforming consumers establish a stable extraction boundary.

## Principles

1. Product/domain state is typed and independent of terminal widgets, colors, and localized prose.
2. Rendering is pure over an accepted in-memory snapshot. It performs no database, filesystem, Git, provider, or network operation.
3. Color reinforces meaning but never carries meaning alone.
4. Machine contracts remain locale-neutral.
5. Refresh preserves the last accepted snapshot and never turns incomplete evidence into a healthy zero.
6. Each application keeps its domain model; the shared system owns only presentation and interaction concepts.

## Standard layout

Every full-screen application uses four logical regions:

```text
Application title
------------------------------------------------------------
Navigation pane | Main content
------------------------------------------------------------
Shortcut footer
```

### Application title

- Identifies the product or control center, never the selected row.
- May include one short overall status after a separator.
- Uses the `ApplicationTitle` typography role.
- Remains stable while navigating so the terminal does not visually jump.

### Navigation pane

- Contains the ordered top-level views.
- The one current item is marked by `>`; there is no selected-vs-active split
  or global Navigation/Content focus state.
- Normal width is 21 display cells. Narrow behavior is owned by the shared
  shell contract, not per-product 21/12/4 breakpoints.
- Nested detail views do not become permanent top-level entries. Back returns to the parent list and preserves selection by stable identity.

### Main content

- Owns the selected view's title and body.
- Uses bounded, width-aware components.
- Preserves the primary facts before optional metadata when space shrinks.
- Scroll or pagination state belongs to the view model, not the domain model.

### Shortcut footer

- Shows only commands valid in the current context.
- Uses localized key descriptions while keeping literal key glyphs stable.
- Loading, editing, overlay, conflict, and error contexts may replace the normal footer with the relevant safe actions.

### Minimum and responsive behavior

- `24x10` display cells is the shared absolute minimum inherited from TabBeacon. Applications may require a larger per-view recommended size.
- Below the absolute minimum, render only a bordered application identity, required size, and resize instruction.
- At narrow widths, preserve: status text/glyph, identity, numerator/denominator, and failure rate before trend, latency, or metadata.
- Width means terminal display cells, not bytes or Unicode scalar count. Truncation must not split a grapheme cluster.

## Typography hierarchy

Typography is semantic. A theme maps these roles to Ratatui `Style` values.

| Role | Purpose | Default treatment |
| --- | --- | --- |
| `ApplicationTitle` | Product/control-center identity | Bold, primary foreground |
| `SectionTitle` | Navigation/content group heading | Bold or accent, never larger by spacing alone |
| `FieldLabel` | Stable label paired with a value | Secondary foreground; padded by display width where useful |
| `Value` | Primary data a user is reading | Normal primary foreground |
| `Status` | Health, warning, failure, loading, or selection state | Semantic color plus explicit word/glyph |
| `Metadata` | Provenance, timestamps, coverage detail, IDs | Gray/dim secondary foreground |

Rules:

- Do not use bold for every value; it destroys hierarchy.
- Internal IDs use `Metadata`, never `ApplicationTitle` or `SectionTitle`.
- A failure rate and its sample count are one semantic value and must remain visually adjacent.
- Monochrome rendering must retain the same words, markers, ordering, and state distinctions.

## Color semantics

The palette is resolved through semantic tokens. View code does not name terminal colors directly.

| Semantic color | Shared meaning | Required non-color signal |
| --- | --- | --- |
| Green | healthy, successful, completed | `Healthy`, `Success`, or `✓` |
| Yellow | warning, degraded, incomplete, needs attention | `Warning`, `Degraded`, `Partial`, or `!` |
| Red | failure, critical, operation failed | `Failed`, `Critical`, or `×` |
| Blue | selection, focus, information, active navigation | `>`, focus border, or `Info` text |
| Gray | secondary metadata, timestamps, provenance, disabled context | placement/label such as `Metadata` or `Unavailable` |

Theme interface, conceptually:

```text
Theme
  application_title
  section_title
  field_label
  value
  metadata
  selection
  status_healthy
  status_warning
  status_failure
  status_information
  border
```

The first shared themes are:

- `default`: semantic terminal colors on an unchanged terminal background;
- `monochrome`: no foreground/background color assumptions;
- `no-color`: selected automatically when the user preference or `NO_COLOR` disables color.

Application-specific terminal tab/frame palettes are separate from the full-screen TUI palette even when they share semantic names.

## Navigation and input

Only `KeyEventKind::Press` performs ordinary navigation. `Repeat` and `Release` are ignored unless a later list-specific contract admits bounded repeat with deterministic timing.

| Command | Keys | Behavior |
| --- | --- | --- |
| Up | Up or `k` | Previous navigation item or row in the active focus region |
| Down | Down or `j` | Next navigation item or row in the active focus region |
| Enter | Enter | Activate the selected view, row, control, or explicit action |
| Back | Esc or Backspace | Close overlay, leave detail, then return to the parent view |
| Refresh | `r` | Request a non-blocking refresh; never perform collection inline |
| Search | `/` | Open a search input scoped to the active searchable view |
| Filter | `f` | Open or cycle the active view's typed filters |
| Quit | `q`; Ctrl+C when safe | Exit, or request explicit discard if a staged preference is dirty |

Additional rules:

- An overlay owns key events until dismissed.
- Top-level Up/Down/j/k switch the current screen immediately. Local list and
  detail modes are explicit product states, never a global focus toggle.
- Selection follows a stable item identity across refresh, not only a numeric index.
- Search text and filters are UI state. They never alter the accepted source snapshot.
- The footer is the discoverable command authority; `?` may open a localized expanded help overlay.

## Rendering and refresh behavior

### Pure render boundary

The render function accepts only:

```text
Frame
ApplicationState
Theme
Catalog/Locale
```

It may format bounded view-model data. It must not open SQLite, scan receipts, walk directories, invoke Git, call a runtime, sleep, or wait on a channel.

### Asynchronous refresh

Refreshing is message-based and daemonless. A standard-library worker thread and bounded channel are sufficient; Tokio or another async runtime is not required solely for a TUI.

```text
UI event loop
  -> RefreshRequest { generation, reason }
worker/executor
  -> collect and build a view snapshot
  -> RefreshResult { generation, outcome }
UI event loop
  -> accept newest relevant result
  -> merge state
  -> redraw
```

Contract:

- No database or other blocking collection runs on the render/event thread.
- At most one collection is in flight by default.
- Multiple refresh requests while busy are coalesced into at most one pending refresh.
- Results carry a monotonic generation. An older result never replaces a newer accepted snapshot.
- The worker has no terminal handle and cannot render.
- Shutdown is bounded; the UI must not hang indefinitely waiting for a worker.
- Periodic refresh, if enabled, uses a bounded local cadence and performs no network operation unless a future application contract explicitly permits it.

### Shared resource states

Each independently refreshed projection uses one of:

```text
Idle
Loading { requested_at }
Ready { value, refreshed_at }
Empty { explanation_key, refreshed_at }
Error { message_key, last_good?, failed_at }
```

Loading state:

- Shows the operation and retains a prior accepted snapshot when one exists.
- Does not blank the whole screen for a background refresh.

Empty state:

- Explains what was searched and what the user can safely do next.
- Is distinct from healthy, successful, and zero percent.

Error state:

- Uses a safe localized summary rather than raw private payloads or paths.
- Preserves `last_good` data with an explicit stale/error marker when available.
- Does not erase accepted history because the newest collection failed.

### Terminal resize

- `Event::Resize` requests immediate reflow and redraw.
- Layout is derived from `frame.area()` on every frame; pixel assumptions are prohibited.
- Selected stable identity, search, filters, and accepted snapshots survive resize.
- Narrow and too-small states are regular render states covered by deterministic buffer tests.

### Terminal lifecycle

One RAII-style guard owns raw mode, alternate-screen entry, cursor visibility, and restoration. It must:

- track partial setup;
- attempt all cleanup steps even if one fails;
- be idempotent;
- run on normal quit, errors, and unwinding;
- have deterministic unit tests plus a representative real-terminal smoke before release.

## Internationalization

### Locale identifiers

Supported initial concrete locales:

```text
en-US
zh-CN
```

`auto` is a preference, not a concrete rendering locale.

### Typed keys

Views request typed locale keys; they do not embed user-visible sentences. Keys are semantic and stable, for example:

```text
app.control_center.title
nav.overview
nav.hooks
state.loading
state.empty.no_evidence
status.coverage.partial
shortcut.refresh
```

Interpolation accepts typed/bounded values. Dynamic runtime values, user annotations, and machine IDs are data, not translation keys.

### Catalog organization

Each application keeps application wording in a locale boundary compatible with:

```text
locale/
  mod.rs
  en_us.rs   # locale tag en-US
  zh_cn.rs   # locale tag zh-CN
```

The files may be compile-time Rust catalogs, provided missing/duplicate key tests enforce parity. A larger catalog format may be adopted later without changing the typed key interface.

### Locale resolution

Follow the TabBeacon precedence, using an application-specific environment variable:

```text
explicit admitted CLI override
  -> JERRY application language environment variable
  -> user-local interface preference
  -> operating-system locale
  -> en-US
```

For HookStat the environment variable is `HOOKSTAT_LANG`. Unsupported spellings do not partially match; resolution continues to the next source.

### Runtime switching and persistence

- Changing language in Settings updates the next frame immediately.
- The choice remains staged until explicit Apply.
- Explicit Apply persists `auto`, `en-US`, or `zh-CN` in HookStat-owned user-local interface state.
- Merely reading defaults or launching the TUI creates no preference file.
- Failed or malformed preference reads use the safe fallback without rewriting the file.
- Machine JSON fields, stable plain keys, command names, schema IDs, runtime IDs, handler keys, and persisted enums are never localized.

### Fallback behavior

- Missing key in `zh-CN` falls back to the `en-US` value for that exact key.
- Missing key in `en-US` is a development/test failure and renders a bounded visible placeholder only in a non-release build.
- Catalog parity is a release gate.
- CJK width, combining characters, and emoji grapheme boundaries are tested at normal and narrow widths.

## Component contract

The shared conceptual components are:

- `ApplicationShell`: computes title/body/footer regions;
- `NavigationPane`: renders typed routes and stable selection;
- `ShortcutBar`: renders context-valid commands;
- `StatePanel`: renders loading/empty/error/stale states;
- `StatusBadge`: maps semantic status to text/glyph/theme;
- `KeyValueList`: aligns field labels by display width;
- `DataTable`: preserves required columns and stable row selection;
- `HelpOverlay`: localized, modal, event-isolating help;
- `TerminalGuard`: owns terminal setup/restoration;
- `RefreshController`: coalesces requests and rejects stale generations.

Application code supplies view-model rows, column definitions, locale keys, and semantic status. Components must not depend on HookStat ledger types, Codex types, TabBeacon management types, or provider-specific values.

## Conformance tests

A conforming application proves:

- every top-level view at normal, narrow, and minimum sizes;
- loading, empty, ready, stale-error, and fatal-initial-error states;
- selection identity survives refresh and resize;
- stale refresh generation is rejected and repeated refresh is coalesced;
- no blocking collector is called from render or key handling;
- press/repeat/release behavior;
- `en-US` and `zh-CN` catalog parity and CJK display width;
- default, monochrome, and no-color semantics;
- status meaning remains in text/glyphs;
- terminal restoration after normal exit and injected failures;
- application-specific safety invariants, including HookStat sample denominators and coverage truthfulness.

## Contract disposition

```text
LAYOUT_CONTRACT=FROZEN
TYPOGRAPHY_CONTRACT=FROZEN
COLOR_SEMANTICS=FROZEN
NAVIGATION_CONTRACT=FROZEN
ASYNC_REFRESH_CONTRACT=FROZEN
I18N_CONTRACT=FROZEN
EXTERNAL_SHARED_CRATE=IMPLEMENTED_G21
```
