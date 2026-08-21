# HookStat v0.3 — Codex Reliability Workbench & Unified Human Interface

## Status

PLANNED after accepted v0.2.1 Startup & Period Reliability.

v0.3 remains **Codex-only in production**. Future Claude Code, OpenCode, Agy, and DeepSeek Harness adapters remain experimental/future work and are not release dependencies.

## Product objective

Turn HookStat from a reliability dashboard into a long-term Codex Hook reliability workbench while eliminating Human-interface drift from TabBeacon.

Two outcomes are equally important:

1. **Reliability workbench depth:** recent changes, recovery, revision history, Hook catalog, and failure exploration make long-running Codex Hook behavior understandable without opening every Hook manually.
2. **Unified Human interface:** HookStat uses the same application-shell and interaction grammar as TabBeacon. Product-specific reliability content differs, but shared Human chrome and control behavior do not.

The old phrase `TabBeacon-aligned` is too weak and is retired for v0.3.

Normative rule:

> **TabBeacon-compatible by contract and shared implementation.**

Reference contract: `../docs/design/TABBEACON_UI_UX_PARITY_CONTRACT.md`.

## Production invariants

```text
PRODUCTION_RUNTIME=Codex
NON_CODEX_RUNTIME_REQUIRED_FOR_RELEASE=false
NORMAL_CODEX_LAUNCH=codex
GLOBAL_DAEMON_BASELINE=false
OPT_IN_INSTRUMENTATION=true
HOOK_TRUST_BYPASS=false
RAW_PRIVATE_CONTENT_PERSISTED=false
COVERAGE_TRUTHFUL=true
FAILURE_RATE_WITH_SAMPLE_COUNT=true
RUNTIME_NEUTRAL_CORE=true
```

## Explicit non-goals

v0.3 MUST NOT ship:

- Claude Code production adapter;
- OpenCode production adapter;
- Agy production adapter;
- DeepSeek Harness production adapter;
- Web dashboard;
- cloud synchronization;
- global daemon;
- AI-generated root-cause diagnosis;
- remote telemetry service;
- distributed multi-machine aggregation;
- notification/ntfy automation as a release requirement.

Those may be investigated later without blocking Codex production.

# Goal dependency DAG

```text
accepted v0.2.1
  ↓
HS-G20 — TabBeacon UI/UX Parity Audit
  ↓
HS-G21 — Shared Human Interface Contract / Reuse Boundary
  ↓
HS-G22 — Navigation, Footer, Settings & Overlay Convergence
  ↓
HS-G23 — Shell, Header, Theme & Typography Convergence
  ↓
HS-G24 — Codex Changes & History Workbench
  ↓
HS-G25 — Hook Catalog & Failure Exploration
  ↓
HS-G26 — Human Interface / Accessibility / Dogfood Hardening
  ↓
HS-G27R — v0.3 Hardening & Release
```

Goals remain sequential by default because the UI-state refactor in G22/G23 directly constrains G24/G25 presentation. Long unattended trains may combine adjacent Goals only after the earlier Goal's acceptance conditions are actually met and recorded.

# HS-G20 — TabBeacon UI/UX Parity Audit

## Objective

Build an exact source-level parity matrix between current accepted TabBeacon Control Center behavior and HookStat before changing UI code.

## Required reference evidence

Audit the current accepted TabBeacon implementations of at least:

- Control Center shell and screen model;
- Human presentation/localization;
- settings/edit mode;
- footer state machine;
- help/repair overlays;
- terminal session lifecycle;
- press-only input handling;
- narrow/minimum-size behavior.

Record a concrete TabBeacon baseline commit. Later TabBeacon changes do not silently change HookStat acceptance; a baseline move requires an explicit roadmap/contract update.

## Required parity matrix

At minimum classify each item as `MATCH`, `DRIFT`, `DOMAIN_EXCEPTION`, or `NOT_APPLICABLE`:

```text
header geometry
header grammar
sidebar width
sidebar title
screen marker grammar
page switching
selected vs active state
navigation/content focus
settings edit entry
settings field navigation
settings value change
apply key
revert key
dirty quit confirmation
help overlay
footer navigation text
footer editing text
footer conflict/discard text
footer spacing grammar
color/chrome policy
typography
locale source precedence
Windows OS locale lookup
minimum size
resize behavior
key repeat/release filtering
terminal cleanup
```

`DOMAIN_EXCEPTION` requires written justification. “HookStat was already implemented differently” is not sufficient justification.

## Acceptance

```text
TABBEACON_BASELINE_PINNED=true
PARITY_MATRIX_COMPLETE=true
UNJUSTIFIED_DRIFT=0
DOMAIN_EXCEPTIONS_DOCUMENTED=true
```

# HS-G21 — Shared Human Interface Contract / Reuse Boundary

## Objective

Stop future drift by converting parity into a shared implementation boundary wherever practical.

HookStat's earlier TUI ADR intentionally kept its UI implementation internal until a second conforming application proved a stable shared boundary. TabBeacon + HookStat now satisfy that condition, and real dogfood has demonstrated the cost of duplicated interaction state machines.

## Required design decision

Evaluate and select one reuse strategy:

### Option A — dedicated shared crate/repository

Conceptually:

```text
jerry-terminal-ui
  ├ shell/layout
  ├ Human chrome style
  ├ navigation
  ├ footer grammar
  ├ help overlay
  ├ edit mode primitives
  ├ press-only key policy
  ├ terminal guard
  ├ locale/display-width primitives
  └ parity fixtures
```

### Option B — another cross-repository shared implementation mechanism

Allowed only if it produces an equally enforceable single source of truth and does not require copy/paste synchronization.

### Rejected baseline

Continuing two independent UI state-machine implementations that merely promise to stay visually similar is not an accepted v0.3 end state.

## Shared vs domain-owned boundary

Shared Human-interface infrastructure should own:

- shell geometry;
- header and sidebar chrome;
- top-level page navigation behavior;
- footer state grammar;
- generic help overlay shell;
- edit/draft interaction primitives;
- press-only key policy;
- terminal lifecycle/min-size/resize primitives;
- display-cell-safe truncation;
- Human chrome color/typography policy.

HookStat remains responsible for:

- period selector semantics;
- Hook rows and detail models;
- risk/trend/revision/failure content;
- Changes workbench;
- diagnostics domain facts;
- safe alias operations.

## Acceptance

```text
SHARED_UI_BOUNDARY_DECIDED=true
COPY_PASTE_STATE_MACHINE_BASELINE=false
TABBEACON_REMAINS_REFERENCE=true
HOOKSTAT_DOMAIN_BOUNDARY_EXPLICIT=true
```

# HS-G22 — Navigation, Footer, Settings & Overlay Convergence

## Objective

Make HookStat interaction behavior feel like the same application family as TabBeacon, not a separately designed TUI.

## Top-level navigation contract

When not in a local edit/list/detail interaction:

```text
Up / Down / j / k
        ↓
current top-level screen changes immediately
```

Required changes from v0.2 behavior:

- remove top-level `Navigation` vs `Content` focus as a prerequisite to page switching;
- remove selected-route vs active-route divergence;
- top-level sidebar has one current screen;
- current screen uses one `>` marker;
- no secondary `• active` marker;
- `Tab` MUST NOT be required merely to navigate to another top-level page.

Local list/detail interaction remains allowed. For example, Hooks may enter a local row-selection/drill-down context, but that context must use the shared interaction grammar and footer state rather than inventing a global second focus system.

## Footer grammar

HookStat footer must use TabBeacon sentence-style Human grammar:

```text
<Key> action  <Key> action  <Key> action
```

Use spacing/state replacement rather than a generated ` · ` token chain.

Footer is a state machine, not a static complete command inventory. At minimum support:

- normal navigation;
- local list/detail interaction;
- settings editing;
- dirty/unsaved state;
- discard confirmation;
- overlay dismissal;
- conflict/warning state where applicable.

Longer command documentation belongs in `?` Help rather than forcing the footer to truncate every possible shortcut.

## Settings interaction contract

Match TabBeacon:

```text
Enter     enter/finish field-edit mode
Up/Down   select setting while editing
Left/Right change draft value
a         Apply
r         Revert
q         quit; confirm before discarding dirty draft
```

HookStat may use `r` as Refresh on non-settings reliability pages. Context-specific meaning is acceptable when footer/help make it explicit.

## Help overlay

Add `?` Help using the shared overlay model.

Required behavior:

- overlay owns normal keys while open;
- `Esc`, `?`, or `q` dismiss Help consistently with the reference;
- Help explains navigation, period switching, Hook search/filter/sort, detail exploration, and settings;
- Help uses the active locale;
- footer switches to overlay-dismiss guidance while overlay is open.

## Press policy

Keep the accepted press-only rule:

```text
KeyEventKind::Press   -> admitted
Repeat                -> ignored
Release               -> ignored
```

## Acceptance

```text
TOP_LEVEL_GLOBAL_FOCUS_MODEL=false
TOP_LEVEL_SELECTED_ACTIVE_DIVERGENCE=false
DIRECT_PAGE_NAVIGATION=true
CURRENT_MARKER_GRAMMAR=tabbeacon
FOOTER_GRAMMAR_PARITY=true
FOOTER_DOT_TOKEN_CHAIN=false
SETTINGS_EDIT_MODEL_PARITY=true
DIRTY_QUIT_CONFIRMATION=true
HELP_OVERLAY_PARITY=true
PRESS_ONLY_INPUT_PARITY=true
```

# HS-G23 — Shell, Header, Theme & Typography Convergence

## Objective

Make a screenshot of HookStat and TabBeacon immediately recognizable as the same Human-interface system before reading product-specific content.

## Normal shell geometry

Use the reference shell contract for normal terminals:

```text
header:  2 rows
body:    remaining
footer:  2 rows
sidebar: 21 columns
content: remaining
minimum: 24x10
```

Narrow fallback may adapt, but the adaptation must come from shared primitives/parity rules rather than independent HookStat breakpoints.

## Header grammar

Reference grammar:

```text
<Product Human Title> — <overall Human status>
```

For example:

```text
HookStat Reliability Center — Coverage limited
HookStat 可靠性中心 — 覆盖受限
```

Product wording may differ, but structure, emphasis, spacing, and chrome style match TabBeacon.

## Sidebar

- same block/border style;
- same normal width;
- same Human section-title grammar (`Sections` / `分区` or its shared equivalent);
- one `>` current-screen marker;
- no separate dark selected background just because HookStat previously had a focus model.

## Color/chrome policy

TabBeacon Human chrome is normative.

HookStat must not maintain a separate global chrome palette for header/sidebar/footer/normal controls. If the shared Human color setting enables color, shared chrome follows the TabBeacon policy; if color is disabled/NO_COLOR, both applications fall back consistently.

Reliability-domain status content may use additional semantic glyphs/text. Additional domain colors are allowed only when documented as a content-level exception and MUST NOT alter the shared chrome language or become the primary means of conveying state.

## Typography

Match shared TabBeacon hierarchy. Avoid HookStat-only bold/dim/color hierarchy in shared chrome. Domain tables may use restrained formatting only after shared roles are satisfied.

## Locale policy

Match TabBeacon locale-source precedence and OS-locale behavior:

```text
explicit CLI
  ↓
environment
  ↓
persisted preference
  ↓
operating-system locale
  ↓
en-US fallback
```

On Windows, use actual Windows user locale behavior rather than assuming `LANG` exists.

## Acceptance

```text
HEADER_HEIGHT_PARITY=true
FOOTER_HEIGHT_PARITY=true
NORMAL_SIDEBAR_WIDTH_PARITY=true
MINIMUM_SIZE_PARITY=true
HEADER_GRAMMAR_PARITY=true
SIDEBAR_GRAMMAR_PARITY=true
HUMAN_CHROME_STYLE_PARITY=true
TYPOGRAPHY_PARITY=true
LOCALE_POLICY_PARITY=true
WINDOWS_OS_LOCALE_PARITY=true
```

# HS-G24 — Codex Changes & History Workbench

## Objective

Add a first-class `Changes` page so long-running Codex Hook evolution is visible without opening every Hook detail page.

## Top-level screen target

```text
Overview
Hooks
Changes
Diagnostics
Interface
```

Hook Detail remains drill-down, not a permanent top-level screen.

## Changes event model

Derive only from admitted ledger/history evidence. Candidate Human events:

### Regression

A previously stable/more reliable Hook becomes materially worse with sufficient samples and coverage.

### Recovery

A previously degraded Hook becomes materially better/stable with sufficient evidence.

### Revision change

Stable handler key moves to a new proven revision epoch.

### New Hook

First admitted observation of a stable handler identity.

### Historical/inactive Hook

A Hook was observed historically but is no longer present/active only when the runtime/discovery evidence is sufficient to make that statement. Absence of evidence is not disappearance.

## Example Human surface

```text
Recent Changes · 7d

REGRESSION
HAPI Stop Hook
2.1% -> 8.7% · samples 239

REVISION
TabBeacon Notification
hr_a82... -> hr_f19...

NEW
Workspace Cleanup Hook
first seen 3h ago

RECOVERED
ntfy Notification
6.4% -> 0.8%
```

All items retain drill-down to base evidence/counts.

## History model

Add/derive:

- first seen;
- last seen;
- current revision epoch;
- ordered revision timeline;
- period-specific status/classification;
- latest admitted evidence timestamp.

Do not infer exact “removed at” times from missing data unless discovery coverage proves it.

## Acceptance

```text
CHANGES_PAGE=PASS
REGRESSION_EVENTS=PASS
RECOVERY_EVENTS=PASS
NEW_HOOK_EVENTS=PASS
REVISION_CHANGE_EVENTS=PASS
INACTIVE_HOOK_CLAIM_COVERAGE_AWARE=true
FIRST_SEEN_LAST_SEEN=PASS
REVISION_TIMELINE=PASS
FABRICATED_HISTORY=false
```

# HS-G25 — Hook Catalog & Failure Exploration

## Objective

Make the entire known Codex Hook population and recurring failure classes browsable as a long-term workbench.

## Hook catalog

For each stable Hook where evidence exists, show or make accessible:

```text
display name
stable internal key
source/event
first seen
last seen
current revision
historical revision count
selected-period runs
failure samples
failure rate
coverage
risk/confidence
latest evidence time
current/historical status
```

Internal `hk_*` remains metadata, never primary Human identity.

## Safe Human alias editing

Allow local TUI alias editing using existing sanitized alias storage semantics.

Requirements:

- explicit user action;
- edit only HookStat-owned presentation metadata;
- no effect on handler stable key, instrumentation, trust, or Codex config;
- conflict-safe persistence;
- revert/cancel behavior follows shared edit UX;
- privacy-safe validation.

## Failure exploration

Promote bounded failure fingerprints into a browsable surface or drill-down:

- fingerprint kind;
- occurrence count;
- first/latest occurrence where safely derivable;
- affected Hook(s);
- selected period;
- coverage;
- no raw stderr/stdout/prompt/tool payload.

## Trend visualization

Add compact trend rendering where width permits. Sparklines/buckets are preferred over verbose prose if they remain deterministic and bilingual labels are preserved.

Any visual trend must retain access to sample counts and availability semantics.

## Acceptance

```text
HOOK_CATALOG=PASS
HUMAN_ALIAS_EDITING=PASS
ALIAS_MUTATES_CODEX=false
FAILURE_EXPLORATION=PASS
RAW_ERROR_STREAM_INSPECTED=false
COMPACT_TREND_VISUALIZATION=PASS
SAMPLE_CONFIDENCE_VISIBLE=true
DATA_FRESHNESS_VISIBLE=true
```

# HS-G26 — Human Interface / Accessibility / Dogfood Hardening

## Objective

Prove the unified Human interface under realistic Windows Terminal use and make parity durable.

## Required verification families

### Source parity

Automated contract tests assert shared chrome/navigation/footer/settings/overlay semantics rather than comparing screenshots only.

### Render parity

Deterministic TestBackend fixtures for HookStat and shared primitives cover:

- en-US;
- zh-CN;
- normal width;
- narrow width;
- minimum height;
- long Human Hook names;
- empty/loading/error states;
- Changes populated/empty;
- Help overlay;
- dirty Settings/discard confirmation;
- color/no-color.

### Owned Windows Terminal A/B dogfood

Open TabBeacon and HookStat on the same owned terminal environment and verify:

- page switching feels identical;
- sidebar marker behavior is identical;
- footer spacing/state grammar is identical;
- Settings editing/apply/revert/quit behavior is identical;
- `?` Help behaves identically;
- header/sidebar/footer chrome is visually the same family;
- terminal cleanup/resize behavior is consistent.

The proof should focus on interaction/chrome parity, not identical domain content.

### Accessibility

Preserve:

- no-color path;
- CJK display-cell correctness;
- text/glyph semantics not color-only;
- minimum-size fallback;
- press-only input to avoid accidental repeat storms.

## Acceptance

```text
TABBEACON_UI_UX_PARITY=PASS
SOURCE_CONTRACT_PARITY=PASS
RENDER_PARITY=PASS
OWNER_WINDOWS_TERMINAL_A_B_DOGFOOD=PASS
EN_US=PASS
ZH_CN=PASS
NO_COLOR=PASS
CJK_WIDTH=PASS
MINIMUM_SIZE=PASS
TERMINAL_RESTORATION=PASS
```

# HS-G27R — v0.3 Hardening & Release

## Objective

Settle one exact-head v0.3 candidate proving both the Codex Reliability Workbench and unified TabBeacon-compatible Human interface.

## Required gates

- G20-G26 complete;
- public-production runtime remains Codex only;
- v0.2.1 performance contract remains green;
- no new runtime adapter accidentally promoted;
- exact-head Windows/Linux CI;
- locked fmt/clippy/tests/build/package/publish dry-run;
- privacy and coverage semantics regression proof;
- owned Windows Terminal A/B dogfood accepted;
- fresh package/install smoke;
- separate Owner authorization for publication.

## Completion definition

```text
PRODUCTION_RUNTIME=Codex
NON_CODEX_RUNTIME_REQUIRED=false
TABBEACON_UI_REFERENCE=normative
TABBEACON_UI_UX_PARITY=PASS
SHARED_UI_BOUNDARY=PASS
TOP_LEVEL_GLOBAL_FOCUS_MODEL=false
TOP_LEVEL_SELECTED_ACTIVE_DIVERGENCE=false
DIRECT_PAGE_NAVIGATION=true
FOOTER_GRAMMAR_PARITY=true
SETTINGS_EDIT_MODEL_PARITY=true
DIRTY_QUIT_CONFIRMATION=true
HELP_OVERLAY_PARITY=true
PRESS_ONLY_INPUT_PARITY=true
HUMAN_CHROME_STYLE_PARITY=true
LOCALE_POLICY_PARITY=true

CHANGES_WORKBENCH=PASS
REVISION_TIMELINE=PASS
HOOK_CATALOG=PASS
FAILURE_EXPLORATION=PASS
HUMAN_ALIAS_EDITING=PASS
DATA_FRESHNESS_VISIBLE=true
COVERAGE_TRUTHFUL=true
FAILURE_RATE_WITH_SAMPLE_COUNT=true

NORMAL_CODEX_LAUNCH=codex
GLOBAL_DAEMON_BASELINE=false
HOOK_TRUST_BYPASS=false
RAW_PRIVATE_CONTENT_PERSISTED=false

WINDOWS=PASS
LINUX=PASS
CARGO_PACKAGE=PASS
CARGO_PUBLISH_DRY_RUN=PASS
PUBLICATION_AUTHORIZED=false
```

# Future runtime X-tracks

After v0.3, future qualification may use:

```text
HS-G30X Claude Code Adapter Qualification
HS-G31X OpenCode Adapter Qualification
HS-G32X Agy Adapter Qualification
HS-G33X DeepSeek Harness Adapter Qualification
```

These are deliberately excluded from the v0.3 production DAG. They become production work only after real Owner use plus an explicit promotion decision.
