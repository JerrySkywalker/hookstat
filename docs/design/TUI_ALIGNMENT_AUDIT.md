# HookStat v0.2 TUI Alignment Audit

Status: design foundation for `HS-V02-G00-TUI-FOUNDATION`.

## Evidence basis

This audit compares repository code, not screenshots or inferred product behavior.

- HookStat: `195fe0dc0269556f5ca01a50d93160d15f74f60f` (`v0.1.0` plus the committed v0.2 roadmap).
- TabBeacon reference: remote `main` at `e70461b7e79a3c6a8a38ab5387aa2b1065e5671b` on 2026-08-21.
- The local TabBeacon checkout was nine commits behind that remote. The GitHub compare showed that the only changes in the relevant TUI files were test fixture construction in `control_center.rs` and several catalog strings in `human_presentation.rs`; the architectural observations below therefore use the locally inspected implementation plus those remote patches.

Primary HookStat evidence:

- `Cargo.toml`
- `src/tui.rs`
- `src/main.rs::tui_command` and `load_current_report`
- `src/domain.rs`, `src/analytics.rs`, `src/report.rs`, and `src/ledger.rs`
- `docs/design/TUI_SPEC.md`

Primary TabBeacon evidence (read-only; no TabBeacon files were changed):

- `Cargo.toml`
- `src/control_center.rs`
- `src/human_presentation.rs`
- `src/interface_preferences.rs`
- `src/presentation/mod.rs`
- `src/main.rs::ui` and `collect_control_center_refresh`
- `docs/adr/0011-human-interface-and-guided-management.md`
- `docs/adr/0012-localized-human-presentation-and-workspace-preferences.md`

## Executive finding

HookStat and TabBeacon already share the same terminal framework family: Ratatui over Crossterm. They do not yet share a reusable UI package, component model, version pair, semantic TUI palette, or refresh executor. HookStat is a compact two-screen reliability viewer. TabBeacon is a multi-screen Control Center with a persistent navigation pane, typed application state, localized catalog, user-local interface preferences, a centralized terminal lifecycle guard, and substantially broader render tests.

The correct v0.2 direction is to align HookStat with TabBeacon's information architecture and state boundaries while formalizing missing cross-application contracts. In particular, the requested asynchronous refresh contract is a target improvement: TabBeacon currently refreshes on a bounded deadline but calls its collector synchronously on the event-loop thread. HookStat must not describe that implementation as already asynchronous.

## Detailed comparison

| Concern | HookStat v0.1 | TabBeacon reference | Alignment conclusion |
| --- | --- | --- | --- |
| Framework | `ratatui 0.30`, `crossterm 0.29` | `ratatui 0.29.0`, `crossterm 0.28.1` | Framework choice matches; dependency versions do not. Do not force a version change until G00 proves the compatibility surface. |
| Top-level layout | Four vertical areas: title, coverage text, table/content, help footer (`src/tui.rs::draw_home`) | Three vertical areas: title, body, footer; body is a 21-cell navigation pane plus main content (`control_center.rs::render`) | HookStat needs the shared title/navigation/content/footer shell. |
| Screens | `Home`, `Detail` | Eight typed screens plus modal overlays | HookStat needs typed top-level views and a nested detail route; it does not need TabBeacon-specific screens. |
| Application state | Raw invocations, counts, window, selected row index, screen, one error string | Separate observed snapshot, accepted baselines, drafts, conflicts, focus, overlays, and commands | Adopt typed view state and explicit loading/empty/error/ready resources. Keep HookStat read-only unless a later goal explicitly adds a safe preference write. |
| Domain/view boundary | `App::report()` recomputes `MachineReport` from raw invocations during rendering and input handling | The Control Center consumes typed projections (`ManagementSnapshot`, `ManagementOverview`, sessions, workspace) collected outside render | Build HookStat view models before rendering. A frame must not query the ledger or aggregate an unbounded history. |
| Widgets | `Paragraph` and `Table`; selection styling is inline | `List`, `Paragraph`, semantic Human text helpers, overlays; application-specific rendering remains mostly in one file | Introduce a small internal component layer for shell, navigation, status, tables, state panels, and footer. Do not copy TabBeacon domain widgets verbatim. |
| Keyboard handling | Reads any key event; `j/k`, arrows, Enter, Esc/Backspace, range keys, `r`, `q` | Processes only `KeyEventKind::Press`; ignores repeat/release for ordinary navigation; overlays own events | Standardize press-only commands, context ownership, `j/k` aliases, Enter, back, refresh, search, filter, and quit. |
| Theme | Hardcoded yellow coverage text and dark-gray selected-row background | TUI chooses cyan or monochrome through `HumanColor`; terminal presentation has a separate semantic `PresentationTheme` RGB mapping | Neither application currently exposes the complete shared TUI semantic palette. G00 must add semantic tokens; colors may never be the only state signal. |
| Typography | Inline bold modifiers and literal titles | Typed Human tones for normal Human output; Control Center uses bold title and localized labels but no reusable TUI typography type | Freeze named typography roles and map them through the theme rather than styling individual views ad hoc. |
| Localization | UI, time-window labels, runtime labels, and status strings are hardcoded English | Typed `HumanMessageKey`, `ResolvedLocale`, deterministic resolution, `en-US`/`zh-CN` catalog, user-local `InterfacePreferences`, CJK/grapheme width helpers | Follow the typed-key and preference model. Keep machine JSON keys, IDs, enum spellings, and CLI command names locale-neutral. |
| Refresh | `r` directly invokes a synchronous closure. That closure scans receipts, ingests SQLite, and reads all invocations on the event-loop thread. | A 750 ms deadline triggers a synchronous read-only collector and merges the result without overwriting dirty drafts | Both loops can block. HookStat's target must use an off-thread refresh worker and generation-tagged results; render only accepted snapshots. |
| Loading/empty/error | Empty and refresh-error text exist; there is no loading state. Refresh failure preserves accepted in-memory history. | Initial collection occurs before the TUI; live refresh has no explicit loading/error resource state and propagates collection errors out of the loop | Add explicit `Loading`, `Empty`, `Ready`, and stale-data `Error` states while preserving the last accepted snapshot. |
| Resize | Width below 52 selects a compact home layout. Resize events are not explicitly handled. | Minimum `24x10` state, normal/narrow render tests, CJK-width handling. Resize events are not explicitly handled, but frames re-read the terminal area. | Handle `Event::Resize` as an immediate reflow/redraw signal and retain a localized too-small state. |
| Terminal lifecycle | Manual setup recovery and chained cleanup | Central `TerminalGuard` tracks partial entry and attempts every restoration step on drop | Adopt the guard pattern and its deterministic lifecycle tests. |
| Test coverage | Three buffer tests at width 80/44, covering empty, sample count, refresh error, detail, and replacement | Screen matrix, normal/narrow/minimum sizes, Chinese, monochrome, press/repeat behavior, overlays, terminal guard failures, real terminal smoke | G00/G01 must add state, size, locale, monochrome, input, resize, async result ordering, and cleanup families. |

## Current HookStat deviations

1. There is no persistent navigation pane or shared application shell.
2. The `App` stores domain records rather than view-ready projections, and `report()` is called from draw and key-handling paths.
3. Refresh performs receipt scanning, SQLite writes to the HookStat-owned ledger, a full ledger read, and aggregation synchronously in the UI thread.
4. There is no loading state and no typed stale-data/error state.
5. Screen selection is row-index based; a refresh can retain the same numeric index while changing the selected handler identity.
6. Theme and typography decisions are embedded directly in render functions.
7. All TUI strings are hardcoded English. `Runtime::label`, `HookEvent::label`, and `TimeWindow::label` also mix domain values with English presentation.
8. Input handling does not filter key press from repeat/release and has no search or filter state.
9. Terminal restoration is careful but not represented by one independently testable guard.
10. The compact layout changes only one table representation and does not define a general responsive shell or explicit minimum-size state.
11. Handler display uses an opaque generated label/key suffix, so internal and human identity remain coupled.
12. HookStat's current `render.rs` text renderer and `tui.rs` renderer duplicate presentation wording without a shared typed Human layer.

## Target TabBeacon-compatible architecture

The target is an internal modular UI subsystem first:

```text
src/tui/
  mod.rs                 terminal admission and public run API
  app.rs                 typed navigation and immutable accepted snapshots
  command.rs             semantic key commands
  event_loop.rs          event, resize, tick, worker-result orchestration
  refresh.rs             off-thread request/result protocol
  layout.rs              title/nav/content/footer geometry
  theme.rs               semantic colors and typography roles
  components/            navigation, shortcut bar, state panel, tables
  views/                  Overview, Hooks, Hook Detail, Diagnostics, Settings
  locale/                 typed keys and en-US/zh-CN catalogs
```

The important boundary is logical, not the exact filenames:

```text
HookStat domain/ledger/analytics
            |
            v
read-only view-model builder (worker)
            |
            v
generation-tagged snapshot/result
            |
            v
UI app state -> pure render(frame, state, theme, catalog)
            |
            v
semantic commands requested by key handling
```

Required compatibility with TabBeacon:

- Ratatui/Crossterm terminal stack;
- application-title / navigation / main-content / shortcut-footer layout;
- typed screens and press-only keyboard semantics;
- centralized terminal lifecycle restoration;
- typed semantic presentation and locale keys;
- `auto`, `en-US`, and `zh-CN` preference semantics;
- CJK display-cell and grapheme-safe truncation;
- semantic color plus textual/glyph fallback;
- refreshed observational state separated from interactive state;
- deterministic buffer tests across normal, narrow, minimum, locale, and no-color variants.

HookStat-specific view models must remain runtime- and evidence-source-neutral. Codex discovery, receipt, trust, and instrumentation types must not enter generic UI components.

## Migration risks

### Blocking refresh and stale results

Moving refresh off-thread introduces result ordering and shutdown concerns. Every request needs a monotonically increasing generation; the UI accepts only the newest relevant generation. At most one refresh should run at a time, with repeated requests coalesced. The worker must not own terminal state.

### Semantic drift from v0.1 reporting

The Reliability Center must preserve the existing denominator rules: a failure rate is always paired with its sample count, incomplete/unknown evidence is not healthy, and Blocked/Stopped are not execution failures. View-model tests must assert these invariants independently of layout.

### Identity migration

The present `handler_label` is stored per invocation and is usually an opaque generated value. Human identity must be introduced without changing `handler_key`, `handler_revision`, deduplication, manifest trust, or existing ledger meaning. G02 owns that migration.

### Localization and machine contracts

Moving `label()` methods out of domain presentation risks changing JSON or plain output accidentally. Machine enum values and report fields stay stable; only Human/TUI rendering uses locale catalogs.

### Framework version split

The applications use different Ratatui/Crossterm versions. An external shared crate now would either force a premature upgrade/downgrade or expose backend types across an unstable boundary. The accepted strategy is documented in `docs/adr/0004-terminal-ui-system-strategy.md`.

### Shared-contract versus current TabBeacon behavior

TabBeacon's existing deadline refresh is synchronous and its TUI semantic palette is not yet the full color contract defined for future tools. HookStat must implement the shared target faithfully without asserting byte-for-byte component parity that does not exist. A later extraction can update both consumers.

### ADR numbering collision

The repository already has `0004-opt-in-instrumented-evidence-fallback.md`. This train requires `0004-terminal-ui-system-strategy.md`; both files must be referenced by full slug, not numeric prefix alone. No existing ADR is renamed in this design-only train.

### Roadmap overlap

The older `dev_governance_files/ROADMAP.md` describes v0.2 narrowly as revision/reliability depth and places doctor/probe/repair under `Later`. The newer, dedicated `HOOKSTAT_V02_ROADMAP.md` explicitly includes the Reliability Center and `hookstat doctor`. G04 may implement read-only doctor/diagnostics under the dedicated roadmap, but any mutation or repair remains out of scope and the two roadmap documents should be reconciled in that implementation goal rather than silently broadening authority.

## Recommended implementation order

1. **HS-V02-G00 — TUI Foundation:** introduce the internal shell, theme/typography tokens, typed key commands, terminal guard, locale interfaces, and generation-tagged refresh protocol with compatibility tests. Do not rewrite all views yet.
2. **HS-V02-G01 — Reliability Center TUI:** build Overview, Hooks, and Hook Detail view models and screens over the existing ledger/analytics semantics; move refresh/aggregation off the UI thread.
3. **HS-V02-G02 — Human Identity:** add the safe display-identity resolver and future schema migration while leaving internal keys authoritative.
4. **HS-V02-G03 — i18n:** complete `en-US` and `zh-CN` catalogs, runtime switching, persistent preference, CJK/narrow/no-color matrices.
5. **HS-V02-G04 — Diagnostics:** add the read-only diagnostics projection, `doctor`, and sanitized export without coupling UI to Codex mutation APIs.
6. **HS-V02-G05 — Reliability Intelligence:** add trend, risk, fingerprints, and revision comparison to the view-model layer.
7. **HS-V02-G06 — Release Candidate:** run full regression, Windows/Linux, presentation, privacy, package, and release-candidate gates. Publication remains separately authorized.

## Audit disposition

```text
HOOKSTAT_CURRENT_ARCHITECTURE_AUDITED=true
TABBEACON_REFERENCE_AUDITED=true
UNSUPPORTED_ASYNC_CLAIM=false
TARGET_ARCHITECTURE_DEFINED=true
IMPLEMENTATION_STARTED=false
TUI_ALIGNMENT_AUDIT=PASS
```
