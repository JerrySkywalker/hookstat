# HookStat v0.4 execution roadmap

## Status

IN_PROGRESS from public HookStat v0.3.1.

```text
G40=ACCEPTED
G41=ACCEPTED
G42=ACCEPTED
G43=ACCEPTED
G44=QUALIFIED_UPSTREAM_UNAVAILABLE
G45_AUTOMATED_PREPARATION=ACCEPTED
G45_OWNER_FIRST_PASS=FAIL_HISTORICAL
G45V_A=ACCEPTED
G45V_B=ACCEPTED
G45V_C=ACCEPTED
G45_OWNER_REDOGFOOD=ACCEPTED
G46R=ACTIVE_RELEASE_CLOSEOUT
PUBLICATION=OWNER_GATE
```

```text
PUBLIC_BASELINE=v0.3.1
PUBLIC_MAIN=651620cbc9f204f312fc31efee424c747895927a
PUBLIC_TAG=v0.3.1
V040_PRODUCT_THEME=Hooks Control Center / Human Usability
PRODUCTION_RUNTIME=Codex
EXPERIMENTAL_RUNTIME_REQUIRED_FOR_V040=false
HISTORICAL_CORRECTION_BASELINE=c24139842e35f83368db00dbf56d9025817d4a9e
CURRENT_EXECUTION_BASELINE=6125734fdbc3edbe33712929abcd4cd1e0e07e1b
```

v0.4 is a product-usability release, not the productionization of a second runtime. DeepSeek Harness, OpenCode, Claude Code, Agy, and other runtime work proceeds in independent `exp/*` tracks and cannot block v0.4.

## Product thesis

**Runtime Truth First, Reliability Second.**

For Codex hooks, HookStat must first answer the same human questions as Codex `/hooks`:

- which hook events exist;
- how many handlers are installed and active;
- which handlers need review;
- what each handler is, where it came from, and how it is configured;
- whether it is enabled, managed, or trusted;
- which runtime warnings/errors affect the hook catalog.

Only after complete runtime truth is visible should HookStat add its reliability layer: observed runs, terminal samples, failures, latency, coverage, revisions, trends, fingerprints, history, and health explanations.

Normative rules:

```text
RUNTIME_TRUTH_FIRST=true
CODEX_HOOKS_INFORMATION_PARITY_IS_FLOOR=true
CODEX_HOOKS_HUMAN_INFORMATION_PARITY=MANDATORY
HOOKSTAT_RELIABILITY_OVERLAY=ADDITIVE
RUNTIME_TRUTH_MAY_NOT_BE_REPLACED_BY_ANALYTICS=true
```

## Accepted product foundation

The following v0.4 nodes remain accepted historical milestones:

- **G40** — v0.3.1 closeout and Codex `/hooks` parity baseline;
- **G41** — ephemeral current-runtime hook catalog and conservative reliability join;
- **G42** — Events → Handlers → Detail Hooks Control Center;
- **G43** — human-readable reliability presentation;
- **G44** — safe-management qualification with `WRITE_PARITY=UPSTREAM_UNAVAILABLE`;
- **G45 automated preparation** — sanitized fixtures and Owner dogfood packet.

The first G45 Owner visual check was the first Human acceptance gate after those nodes. It discovered a release-blocking presentation defect. This does not rewrite G42 history; it created the required correction train before release.

## Owner visual finding — G45-OV-001

Historical Owner dogfood against then-current main `c24139842e35f83368db00dbf56d9025817d4a9e` found that the Hook Event catalog can display the same semantic Codex event twice.

The current parser seeds pinned event names using PascalCase strings such as `PreToolUse`, while the qualified Codex v0.151.0 v2 protocol serializes `HookEventName` with camelCase values such as `preToolUse`. The runtime presentation map keys by raw `(runtime_context, runtime_event_name)`, so the synthetic zero-handler row and the real `hooks/list` row become distinct entries and later localize to the same Human label.

The same pass also exposed a localization-layer defect: known event descriptions seeded as English strings can leak unchanged into the zh-CN TUI, and known runtime events such as `Interrupt` cannot rely on reliability `HookEvent` support to obtain localized presentation semantics.

Historical first-pass disposition (immutable):

```text
G45_OWNER_VISUAL_CHECK=FAIL
EVENT_CATALOG_SEMANTIC_DUPLICATES=true
ZH_CN_KNOWN_EVENT_DESCRIPTION_LEAK=true
G46R_ALLOWED=false
CORRECTION_TRAIN_REQUIRED=G45V
```

See `docs/qualification/G45_OWNER_VISUAL_FINDING_001.md`.

## Why existing automated TUI coverage was insufficient

HookStat already has deterministic Ratatui rendering and interaction tests, including wide/narrow layouts, bilingual rendering, long runtime fields, runtime error states, and the Hooks Control Center navigation model.

The gap is not simply a lack of TUI tests. It is the absence of an explicit **Visual Regression CI** that combines:

```text
official-shaped runtime wire fixture
        ↓
RuntimePresentationSnapshot parser
        ↓
App state
        ↓
full Ratatui terminal frame
        ↓
golden cell-buffer snapshot + structural invariants
```

Existing tests can construct already-normalized presentation objects and assert selected text fragments. That does not prove the actual Codex wire representation produces a correct final screen.

Therefore G45V establishes both a product fix and a durable quality gate.

## Runtime event identity contract after G45V

Runtime event presentation must separate three concepts:

```text
RAW_RUNTIME_WIRE_IDENTITY
KNOWN_RUNTIME_PRESENTATION_IDENTITY
OPTIONAL_RELIABILITY_IDENTITY
```

A known runtime event such as `Interrupt` may be:

```text
known_runtime_event=true
localized_human_name=true
localized_description=true
reliability_event=None
reliability_state=UNAVAILABLE_OR_NOT_ADMITTED
```

Reliability support may not determine whether a known runtime event can be localized.

For the pinned Codex v0.151.0 baseline, semantic wire names are the exact camelCase v2 values:

```text
preToolUse
permissionRequest
postToolUse
preCompact
postCompact
sessionStart
sessionEnd
userPromptSubmit
subagentStart
subagentStop
stop
interrupt
```

The implementation should prefer a typed known-runtime-event mapping over duplicated bare-string tables.

## Runtime context contract

`runtime_context` remains meaningful and must not be discarded merely to make duplicate rows disappear.

Production discovery requests one exact cwd. If multiple contexts are returned unexpectedly, HookStat must either select the exact requested context or visibly disambiguate contexts. It must not silently merge handlers from distinct contexts.

Required:

```text
SAME_CONTEXT_SEMANTIC_EVENT_DUPLICATES=0
CROSS_CONTEXT_SILENT_MERGE=false
CURRENT_CONTEXT_EXPLICIT=true
```

## Human localization contract

Known event names and descriptions are semantic resources, not English transport strings.

Required:

```text
KNOWN_EVENT_NAME_LOCALIZED=true
KNOWN_EVENT_DESCRIPTION_LOCALIZED=true
ZH_CN_KNOWN_EVENT_ENGLISH_LEAK=false
UNKNOWN_EVENT_RAW_NAME_PRESERVED=true
UNKNOWN_EVENT_DESCRIPTION_GUESSED=false
```

Unknown future runtime event names remain visible verbatim rather than being dropped or guessed.

## Visual Regression CI architecture

The primary visual gate is a Ratatui **cell-buffer golden snapshot**, not an operating-system pixel screenshot.

Pixel-level Windows Terminal screenshots are too sensitive to font, DPI, antialiasing, terminal version, transparency, and compositor behavior to serve as the primary deterministic gate.

Canonical visual pipeline:

```text
sanitized fixture / official-shaped wire fixture
                 ↓
          deterministic App state
                 ↓
          ratatui::TestBackend
                 ↓
         complete terminal cell grid
                 ↓
 golden snapshot + structural invariants
                 ↓
              CI gate
```

See `docs/architecture/TUI_VISUAL_REGRESSION_CI.md`.

## Visual snapshot baseline matrix

The exact matrix may be refined by implementation evidence, but G45V-B must cover representative canonical frames across:

```text
Widths/heights:
- wide: 140x58
- standard: about 100x32
- narrow: about 60x30
- very narrow/tall: 44x44

Locales:
- en-US
- zh-CN

Primary views:
- Overview
- Hooks / Events
- Hooks / Handlers
- Hook Detail
- Changes
- Diagnostics
- Settings

Resource states:
- loading
- ready
- stale accepted data
- error

Content stress:
- short values
- long command
- long matcher
- long source
```

The implementation should select a bounded canonical matrix rather than an uncontrolled Cartesian product. Target approximately 20–30 stable frames unless evidence justifies a different number.

## Structural visual invariants

Golden snapshots alone are insufficient because an accidental baseline update could bless a broken screen.

At minimum, deterministic checks must include applicable invariants such as:

```text
EVENT_DISPLAY_IDENTITY_DUPLICATES=0
SELECTED_ROW_COUNT<=1
FOOTER_VISIBLE=true
RAW_UNIX_MS_VISIBLE=false
KNOWN_EVENT_ENGLISH_LEAK_IN_ZH_CN=false
CURRENT_RUNTIME_SECTION_PRECEDES_RELIABILITY=true
OUT_OF_BOUNDS_RENDERING=false
```

Snapshot updates must be explicit, reviewed changes. CI must never auto-accept new visual baselines.

## Dedicated CI gate

G45V-B introduces a stable dedicated visual gate, conceptually:

```text
CI / tui-visual
```

TUI-sensitive changes include at minimum:

```text
src/tui/**
src/runtime_presentation.rs
presentation-relevant src/codex.rs changes
localization resources
terminal-ui-contract dependency changes
visual fixture and baseline changes
```

A workflow implementation may live in the existing CI file or a dedicated workflow, but the result must be separately identifiable from generic `cargo test`.

The visual gate must emit useful failure diagnostics/artifacts without exposing Owner-private runtime values. All committed visual fixtures are sanitized.

## Real-wire end-to-end contract

G45V-C must add an official-shaped, sanitized Codex v0.151.0 `hooks/list` fixture using exact wire names and field casing.

The test path must be:

```text
v0.151-shaped hooks/list JSON
        ↓
from_codex_hooks_list
        ↓
RuntimePresentationSnapshot
        ↓
App runtime catalog
        ↓
Events / Handlers / Detail render
        ↓
golden frame
```

Required proof includes:

```text
preToolUse appears exactly once
installed count comes from real fixture handlers
known description is localized
interrupt appears exactly once
unknown future event appears exactly once
unknown future event remains visible without fabricated reliability
```

## Revised v0.4 dependency DAG

```text
PUBLIC v0.3.1
      │
      ▼
G40 ✅ Rebaseline & parity
      │
      ▼
G41 ✅ Runtime catalog
      │
      ├──────────────────────┐
      ▼                      ▼
G42 ✅ Hooks Center       G43 ✅ Human Reliability
      └──────────┬───────────┘
                 ▼
G44 ✅ Safe Management Qualification
    WRITE_PARITY=UPSTREAM_UNAVAILABLE
                 │
                 ▼
G45 Automated Preparation ✅
                 │
                 ▼
G45 Owner Visual Check ❌
                 │
                 ▼
G45V-A — Runtime Event Identity & Localization Repair
                 │
                 ▼
G45V-B — TUI Visual Regression CI Foundation
                 │
                 ▼
G45V-C — Real-Wire End-to-End Visual Matrix
                 │
                 ▼
G45R — Owner Re-Dogfood
                 │
                 ▼
G46R — v0.4 Hardening & Release
                 │
                 ▼
            PUBLIC v0.4
```

G46R is in active release closeout after the accepted G45R Owner re-dogfood.

## Historical correction estimate

| Goal | Scope | Estimated effort |
| --- | --- | ---: |
| G45V-A | semantic event identity, wire-name correction, localization, context semantics | 2–4 h |
| G45V-B | golden cell-buffer visual harness, structural invariants, dedicated CI gate | 4–7 h |
| G45V-C | official-shaped real-wire fixture and parser→frame E2E matrix | 2–4 h |
| G45R | Owner visual A/B re-dogfood | 0.5–1 h Owner time |
| G46R | release hardening, version/upgrade/package candidate | 2–4 h |
| **Remaining** | **from failed first Owner visual pass** | **10.5–20 h** |

## G45V-A — Runtime Event Identity & Localization Repair

Implement a typed known-runtime-event presentation identity or an equally explicit mechanism that:

- maps exact upstream wire values to known semantic events;
- keeps raw unknown wire names intact;
- localizes known names/descriptions independent of reliability support;
- prevents same-context semantic duplicates;
- preserves runtime context semantics;
- adds deterministic parser regression tests for the Owner-observed duplicate case.

Acceptance is defined in `goals/HS-G45VA-RUNTIME-EVENT-IDENTITY-LOCALIZATION.md`.

## G45V-B — TUI Visual Regression CI Foundation

Implement a bounded, deterministic visual test framework using Ratatui `TestBackend` full-frame cell buffers, explicit golden baselines, structural invariants, and a dedicated CI result.

This goal must not depend on real Owner hook data or pixel screenshots.

Acceptance is defined in `goals/HS-G45VB-TUI-VISUAL-REGRESSION-CI.md`.

## G45V-C — Real-Wire End-to-End Visual Matrix

Add an upstream-shaped sanitized `hooks/list` fixture pinned to Codex v0.151.0 and test the full parser→App→frame path at representative widths/locales/states.

Acceptance is defined in `goals/HS-G45VC-REAL-WIRE-E2E-VISUAL-MATRIX.md`.

## G45R — Owner Re-Dogfood

After G45V-A/B/C are accepted, repeat the Owner Windows Terminal / Codex `/hooks` A/B check from exact accepted main.

The first-pass failure receipt remains historical evidence. Do not overwrite it.

The accepted Owner re-dogfood receipt is bound to accepted main
`6125734fdbc3edbe33712929abcd4cd1e0e07e1b`:

```text
G45_OWNER_REDOGFOOD=PASS
TESTED_MAIN=6125734fdbc3edbe33712929abcd4cd1e0e07e1b
NO_HISTORY_PRESENTATION=PASS
LIVE_RELIABILITY_SMOKE=BOUNDED_UNAVAILABLE_ACCEPTED
FINDINGS=NONE
```

This accepts the NoHistory presentation without claiming populated live
reliability observations.

Required answer to the primary question remains:

> If the user only opens HookStat, do they still need Codex `/hooks` to understand the current hook?

Required answer: **NO**.

## G46R — v0.4 release

G46R is in active release closeout because:

```text
G45V_A=ACCEPTED
G45V_B=ACCEPTED
G45V_C=ACCEPTED
G45_OWNER_REDOGFOOD=ACCEPTED
```

Then use the Fast Lane candidate process:

```text
settle code + docs
→ freeze exact candidate SHA
→ hosted CI / visual CI / independent review / owner dogfood evidence
→ release gate
→ merge
→ separately Owner-authorized publication
```

## Explicit non-goals for G45V

- production DeepSeek Harness adapter;
- production OpenCode adapter;
- broad Web UI/dashboard work;
- pixel-perfect OS screenshot gating as the primary CI mechanism;
- mutation of Owner Codex hook configuration;
- remote telemetry;
- AI-generated visual acceptance;
- weakening privacy or reliability admission contracts.

## v0.4 completion definition

```text
PUBLIC_BASELINE=v0.3.1
PRODUCTION_RUNTIME=Codex

CODEX_HOOKS_HUMAN_INFORMATION_PARITY=PASS
LIVE_RUNTIME_HOOK_CATALOG=PASS
EVENT_DISPLAY_IDENTITY_DUPLICATES=0
KNOWN_EVENT_LOCALIZATION=PASS
REAL_WIRE_TO_FRAME_E2E=PASS
TUI_VISUAL_REGRESSION_CI=PASS
TUI_GOLDEN_BASELINES=PASS
TUI_STRUCTURAL_INVARIANTS=PASS

INSTALLED_UNOBSERVED_HOOKS_VISIBLE=true
HISTORICAL_NOT_INSTALLED_DISTINCT=true
RUNTIME_ISSUES_VISIBLE=true
UNKNOWN_RUNTIME_EVENTS_VISIBLE=true

RAW_UNIX_MILLISECONDS_IN_NORMAL_TUI=false
ZERO_SAMPLE_HEALTHY_PERCENT=false
METRIC_SCOPE_EXPLICIT=true
METRIC_SCOPE_CONSISTENCY=PASS
HUMAN_RISK_EXPLANATION=PASS
HUMAN_COVERAGE_EXPLANATION=PASS

RUNTIME_PRESENTATION_IN_MEMORY_ONLY=true
RAW_RUNTIME_PRESENTATION_PERSISTED=false
SAFE_WRITE_PARITY=UPSTREAM_UNAVAILABLE
MANAGED_HOOK_MUTATION=false

WINDOWS=PASS
UBUNTU=PASS
OWNER_CODEX_HOOKS_AB_DOGFOOD=PASS
PACKAGE=PASS
PUBLICATION=OWNER_GATE
```
