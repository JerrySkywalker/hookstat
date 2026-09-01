# HookStat TUI Visual Regression CI

## Purpose

HookStat already has deterministic TUI unit/render tests, but the first G45 Owner visual pass demonstrated that component-level assertions can still admit a broken final screen. A semantic event was rendered twice because the parser combined a synthetic PascalCase event descriptor with a real camelCase Codex wire event. Existing tests did not exercise the complete upstream-shaped wire → parser → App → terminal frame path.

This document defines the durable visual-regression architecture required before HookStat v0.4 release.

## Design goals

```text
DETERMINISTIC=true
OWNER_PRIVATE_DATA_REQUIRED=false
PIXEL_SCREENSHOT_PRIMARY_GATE=false
FULL_TERMINAL_FRAME_ASSERTED=true
STRUCTURAL_INVARIANTS_ASSERTED=true
UPSTREAM_WIRE_FIXTURE_INCLUDED=true
CI_RESULT_SEPARATELY_VISIBLE=true
BASELINE_UPDATE_EXPLICIT=true
```

The visual gate protects information hierarchy, duplication, wrapping, selection, localization, footer/navigation visibility, and state presentation. It does not attempt to prove font rendering or operating-system compositor fidelity.

## Why cell-buffer snapshots are primary

HookStat uses Ratatui. The most stable visual artifact is therefore the terminal cell grid produced by `ratatui::backend::TestBackend`.

A pixel screenshot from Windows Terminal is not suitable as the primary CI oracle because it varies with:

- font installation and font version;
- DPI and display scaling;
- antialiasing and subpixel behavior;
- Windows Terminal version;
- transparency/background configuration;
- window decorations and compositor state.

Pixel screenshots may later be used as supplementary Owner evidence. They are not the deterministic release gate.

## Canonical pipeline

```text
sanitized fixture
or official-shaped wire fixture
          │
          ▼
application/parser path
          │
          ▼
accepted App state
          │
          ▼
Ratatui TestBackend
          │
          ▼
complete terminal cell buffer
          │
          ├──────────────► structural invariants
          │
          ▼
golden textual/cell snapshot
          │
          ▼
CI / tui-visual
```

The same product rendering functions used by the normal TUI must render the test frame. Do not create a separate simplified visual renderer only for tests.

## Golden representation

The implementation may use a lightweight repository-native snapshot format or an established Rust snapshot crate if dependency cost and maintenance are justified.

Whatever representation is chosen must preserve enough information to detect:

- duplicated rows;
- unexpected missing rows;
- line wrapping changes;
- column movement;
- footer/help disappearance;
- selected-row movement;
- known localization leakage;
- section-order regression;
- unexpected clipping.

At minimum the baseline must represent the complete visible character grid. Style/color roles may be included where stable and valuable, but text/cell position correctness is mandatory.

## Explicit baseline updates

CI must never update or bless snapshots automatically.

Normal CI mode:

```text
render canonical frame
compare with committed baseline
fail on difference
```

Intentional UI-change mode must require an explicit developer command, conceptually:

```powershell
pwsh scripts/tui/update-visual-baselines.ps1
```

Exact script naming may differ.

A baseline update is a normal code-review surface. The diff should be inspectable by a fresh Supervisor together with the code that caused it.

## Canonical frame matrix

Do not run an uncontrolled Cartesian product. Maintain a bounded set of representative frames.

Initial required dimensions:

### Geometry

```text
wide          = 140x58
standard      ≈ 100x32
narrow        ≈ 60x30
very-narrow   = 44x44
```

### Locale

```text
en-US
zh-CN
```

### Primary screens

```text
Overview
Hooks / Events
Hooks / Handlers
Hook Detail
Changes
Diagnostics
Settings
```

### State classes

```text
loading
ready
stale accepted data
error
```

### Stress content

```text
short values
long command
long matcher
long source
installed-but-unobserved
managed/review-needed/disabled
zero terminal samples
partial coverage
runtime warning/error
Interrupt
future unknown event
```

The initial implementation should target approximately 20–30 canonical frames. More frames are justified only by distinct layout/state semantics.

## Structural invariants

Golden snapshots are necessary but not sufficient. Structural assertions must independently reject obvious product errors even if someone accidentally updates a baseline to match them.

Required invariants where applicable:

```text
EVENT_DISPLAY_IDENTITY_DUPLICATES=0
SELECTED_ROW_COUNT<=1
FOOTER_VISIBLE=true
KNOWN_EVENT_ENGLISH_LEAK_IN_ZH_CN=false
RAW_UNIX_MS_VISIBLE=false
ZERO_SAMPLE_HEALTHY_PERCENT_VISIBLE=false
CURRENT_RUNTIME_SECTION_PRECEDES_RELIABILITY=true
OUT_OF_BOUNDS_RENDERING=false
```

Additional implementation-specific invariants are encouraged when they protect stable Human contracts.

## Event duplicate invariant

For one displayed current runtime context, a known semantic runtime event may appear at most once.

This is not a renderer string-dedup rule. It must be derived from semantic runtime-event identity.

Forbidden workaround:

```text
deduplicate rows by localized display label
```

That would incorrectly merge different contexts or future runtime events with colliding Human labels.

## Localization invariant

Known event names and descriptions are localized resources.

For zh-CN canonical frames:

```text
known event name -> Chinese Human label
known event description -> Chinese Human description
```

Raw unknown runtime event names remain visible verbatim. Their descriptions are not fabricated.

## Official-shaped wire fixtures

At least one visual path must begin with a sanitized fixture matching the exact qualified Codex v0.151.0 `hooks/list` representation.

The fixture must use real wire casing such as:

```json
{"eventName":"preToolUse"}
```

not presentation/helper casing such as:

```json
{"eventName":"PreToolUse"}
```

The fixture should include representative handler types and at least:

- one known event with handlers;
- one known event with zero handlers after completion of the catalog surface;
- `interrupt`;
- one synthetic future unknown event;
- warning/error context where useful.

The fixture contains no Owner command, matcher, source path, prompt, or tool payload.

## End-to-end visual path

G45V-C must prove:

```text
Codex-shaped JSON
  -> RuntimePresentationSnapshot::from_codex_hooks_list
  -> App runtime catalog apply
  -> Events render
  -> Handlers render
  -> Detail render
  -> golden snapshots
```

At minimum:

```text
preToolUse_display_count=1
interrupt_display_count=1
unknown_event_display_count=1
installed_count_matches_fixture=true
known_description_localized=true
unknown_event_not_dropped=true
```

## CI routing

Introduce a separately identifiable gate, conceptually:

```text
CI / tui-visual
```

It may be implemented as a dedicated workflow or as a dedicated job in the existing workflow. The check must be visible independently from generic Rust compilation/tests.

TUI-sensitive paths include at minimum:

```text
src/tui/**
src/runtime_presentation.rs
presentation-relevant src/codex.rs changes
visual fixtures/baselines
localization resources
terminal-ui-contract dependency changes
```

The risk classifier should fail safe for unknown presentation-sensitive changes.

## CI artifact behavior

On visual failure, CI should provide bounded, privacy-safe diagnostics such as:

- baseline name;
- expected versus actual text frame;
- compact line/cell diff;
- geometry/locale fixture identity.

Do not upload real Owner runtime frames or private hook values.

## Windows-specific smoke

The deterministic visual gate is cross-platform because it uses `TestBackend`.

A smaller Windows-specific smoke may additionally check:

- Unicode width assumptions;
- terminal-size plumbing;
- Windows-only runtime catalog wiring where relevant.

This smoke is supplementary and does not replace the golden frame gate.

## Ownership and review

Product rendering changes remain `agent/*` product work.

A final visual candidate follows the normal immutable-candidate contract:

```text
settle code + baselines
freeze SHA
run CI / tui-visual + ordinary CI
fresh read-only Supervisor review
no post-freeze receipt commits
```

The Supervisor should review both code and baseline diffs. A baseline-only change without a product rationale is a material review concern.

## Acceptance

```text
TUI_VISUAL_HARNESS=PASS
TUI_GOLDEN_BASELINES=PASS
TUI_STRUCTURAL_INVARIANTS=PASS
REAL_WIRE_VISUAL_PATH=PASS
TUI_VISUAL_CI_SEPARATELY_VISIBLE=true
BASELINE_AUTO_UPDATE=false
OWNER_PRIVATE_VISUAL_DATA=0
PIXEL_SCREENSHOT_PRIMARY_GATE=false
```
