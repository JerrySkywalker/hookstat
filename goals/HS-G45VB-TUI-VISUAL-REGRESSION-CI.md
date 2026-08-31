# HS-G45V-B — TUI Visual Regression CI Foundation

## Objective

Build a deterministic, release-blocking visual-regression gate for HookStat's Ratatui TUI.

This goal exists because G45 Owner dogfood found a duplicated Event catalog row that existing component/render tests did not block.

## Preconditions

```text
G45V_A=PASS
G45_OWNER_VISUAL_CHECK=FAIL_CORRECTION_REQUIRED
G46R=HOLD
```

Read:

- `docs/architecture/TUI_VISUAL_REGRESSION_CI.md`;
- `docs/design/G45V_VISUAL_CORRECTNESS_CHECKLIST.md`;
- accepted G45V-A implementation and receipts.

## Primary mechanism

Use Ratatui `TestBackend` or the exact deterministic equivalent to render the real product TUI into a complete terminal cell grid.

The primary gate is not a Windows Terminal pixel screenshot.

Required:

```text
FULL_FRAME_CELL_GRID=true
PIXEL_SCREENSHOT_PRIMARY_GATE=false
PRODUCT_RENDERER_REUSED=true
```

Do not create a separate simplified renderer whose output can diverge from the production TUI.

## Golden baselines

Introduce committed, explicit golden baselines for a bounded set of canonical frames.

The snapshot representation must be diff-friendly enough for code review and preserve the complete visible text/cell layout required to catch:

- duplicate rows;
- missing rows;
- wrapping changes;
- column drift;
- selection drift;
- footer/help disappearance;
- localization leakage;
- clipping and section-order regression.

Style/color capture is optional if it cannot be made stable without excessive fragility; visible cell text and placement are mandatory.

## Baseline update policy

Normal CI compares actual frames to committed baselines.

CI must never auto-update baselines.

Provide one explicit developer update path, conceptually:

```text
scripts/tui/update-visual-baselines.ps1
```

Exact naming may differ.

The update command must be deterministic and must not read Owner hook/config data.

## Canonical matrix

Establish a bounded initial baseline set covering representative cases across:

### Geometry

```text
140x58
~100x32
~60x30
44x44
```

### Locale

```text
en-US
zh-CN
```

### Screens

```text
Overview
Hooks Events
Hooks Handlers
Hook Detail
Changes
Diagnostics
Settings
```

### States/stress cases

```text
loading
ready
stale accepted data
error
long command
long matcher
long source
installed-unobserved
managed/review/disabled
zero terminal samples
partial coverage
runtime warning/error
Interrupt
future unknown event
```

Do not create an uncontrolled Cartesian product. Target a concise, reviewable set of roughly 20–30 canonical frames unless evidence supports another bounded number.

## Structural invariants

Add invariant checks independent from snapshots.

At minimum where applicable:

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

These invariants must remain capable of failing even if someone intentionally or accidentally regenerates a broken baseline.

## CI integration

Introduce one separately identifiable check, conceptually:

```text
CI / tui-visual
```

It may be a dedicated workflow or dedicated job in the existing workflow, but the final status must be distinguishable from generic Rust build/test.

TUI-sensitive change classification must include at least:

```text
src/tui/**
src/runtime_presentation.rs
presentation-relevant src/codex.rs changes
localization resources
visual fixtures/baselines
terminal-ui-contract dependency changes
```

Unknown presentation-sensitive changes fail safe.

Do not break stable existing required contexts while adding the new gate.

## Failure artifacts

On failure provide bounded privacy-safe diagnostics:

```text
baseline id
geometry
locale
compact expected-vs-actual frame diff
structural invariant failures
```

Never upload a real Owner runtime screen, command, matcher, source path, prompt, tool payload, or other private hook material.

## Windows smoke

A small Windows-specific Unicode/terminal smoke may be added if evidence shows it catches platform-specific failures.

It is supplementary. It does not replace the cross-platform TestBackend golden gate.

## Tests for the visual test system

Test the harness itself where practical:

- intentional frame change fails comparison;
- explicit update rewrites only the intended baseline;
- duplicate-row invariant fails a crafted bad frame/model;
- zh-CN English-leak invariant fails a crafted bad known-event description;
- privacy fixtures contain no Owner-specific values;
- unknown/missing baseline fails closed.

## Cost/CI discipline

Visual CI must remain bounded and reasonably fast. Do not start a real terminal process for every frame when pure TestBackend rendering is sufficient.

Use the existing Fast Lane risk classifier. Visual-sensitive changes should run the visual gate; unrelated docs-only changes should not need the Rust/visual harness unless intentionally overridden for an immutable candidate.

## Acceptance

```text
G45V_B=PASS
TUI_VISUAL_HARNESS=PASS
TUI_GOLDEN_BASELINES=PASS
TUI_STRUCTURAL_INVARIANTS=PASS
TUI_VISUAL_CI_SEPARATELY_VISIBLE=true
BASELINE_AUTO_UPDATE=false
VISUAL_FAILURE_ARTIFACTS=PASS
OWNER_PRIVATE_VISUAL_DATA=0
PIXEL_SCREENSHOT_PRIMARY_GATE=false
CI=PASS
INDEPENDENT_REVIEW=PASS
```

## Next

Begin G45V-C from accepted main.
