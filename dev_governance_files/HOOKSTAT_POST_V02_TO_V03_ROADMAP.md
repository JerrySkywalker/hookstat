# HookStat Post-v0.2 to v0.3 Roadmap

Roadmap IDs are stable governance identifiers. `X` suffixes denote experimental work that does not block the adjacent Codex production release unless a later Owner decision explicitly promotes it.

## Current production baseline

HookStat v0.2.0 is publicly released and is the historical production baseline for this roadmap.

```text
PUBLIC_VERSION=0.2.0
PUBLIC_RELEASE_SHA=ba1d456d310982d816cd0e7fbfd7b67423c34311
TAG=v0.2.0
PRODUCTION_RUNTIME=Codex
NORMAL_CODEX_LAUNCH=codex
DAEMON_REQUIRED=false
```

The v0.2 line established the Reliability Center, Human hook identities, `en-US`/`zh-CN`, read-only diagnostics, sample-aware risk, trend projections, bounded failure fingerprints, revision comparison, and preserved v0.1 instrumentation/trust/privacy invariants.

The released v0.2 roadmap remains historical and MUST NOT be rewritten to pretend later dogfood findings were part of the original release scope.

## Dogfood evidence that admitted this roadmap

Owned `zenbookduo` dogfood after installing crates.io `hookstat 0.2.0` produced the following planning baseline:

```text
HOOKSTAT_VERSION=0.2.0
REPORT_RW_MS=788.6
REPORT_RO_MS=580.4
DOCTOR_MS=1026.3
LEDGER_BYTES=2568192
RECEIPT_FILES=6769
DOCTOR_CODEX_BINARY=FAIL
INSTRUMENTATION=PASS
RECEIPT_SPOOL=PASS
LEDGER=PASS
EVIDENCE_FRESHNESS=PASS
```

These timings are command-level measurements, not TUI time-to-first-frame measurements. Shell-reported TUI process lifetime is not startup latency and MUST NOT be used as such.

Source inspection confirms the following accepted dogfood findings:

1. the initial TUI snapshot is built synchronously before the interactive event loop starts;
2. receipt startup work enumerates and parses the receipt record directory;
3. ledger invocation loading currently reads full historical invocation rows before Rust-side window filtering;
4. diagnostics performs work that is too expensive to couple to every normal reliability refresh;
5. Windows `CodexBinary=Fail` can be a false negative when Codex is available through a PATH-resolved Windows shim such as `codex.cmd` rather than a directly spawnable executable;
6. v0.2 exposes 24h/7d/30d/All but has no local-calendar `Today` period;
7. real Owner use found HookStat and TabBeacon visually related but behaviorally different in navigation, screen switching, selection grammar, settings interaction, header geometry, color policy, and footer hints.

## Product strategy

HookStat remains **Codex-only in production through v0.3**.

The architecture MUST continue to be runtime-neutral so future adapters can be added without rewriting the ledger, analytics, or Reliability Center. However, future Claude Code, OpenCode, Agy, and DeepSeek Harness support is not a production requirement for v0.2.x or v0.3.

```text
PRODUCTION_RUNTIME=Codex
CORE_RUNTIME_NEUTRAL=true
LEDGER_RUNTIME_NEUTRAL=true
ANALYTICS_RUNTIME_NEUTRAL=true
TUI_RUNTIME_NEUTRAL=true
NON_CODEX_RUNTIME_REQUIRED_FOR_RELEASE=false

CLAUDE_PRODUCTION_ADAPTER=false
OPENCODE_PRODUCTION_ADAPTER=false
AGY_PRODUCTION_ADAPTER=false
DEEPSEEK_HARNESS_PRODUCTION_ADAPTER=false
```

Long-term conceptual boundary:

```text
Codex (production today) ───────┐
Claude (future X-track) ────────┤
OpenCode (future X-track) ──────┤
Agy (future X-track) ───────────┤ -> Runtime Adapter -> Canonical HookInvocation
DeepSeek Harness (future X) ────┘                        |
                                                         v
                                                   SQLite ledger
                                                         |
                                                         v
                                                     Analytics
                                                         |
                                                         v
                                                Reliability Center
```

## Product invariants

The following remain mandatory through v0.3:

```text
PRODUCTION_RUNTIME=Codex
NORMAL_CODEX_LAUNCH=codex
FAIL_OPEN=true
GLOBAL_DAEMON_BASELINE=false
HOOK_TRUST_BYPASS=false
OPT_IN_INSTRUMENTATION=true
RAW_PROMPT_TOOL_MODEL_CONTENT_PERSISTED=false
RAW_COMMAND_PERSISTED=false
COVERAGE_TRUTHFUL=true
FAILURE_RATE_WITH_SAMPLE_COUNT=true
UNKNOWN_IS_NOT_HEALTHY=true
RUNTIME_NEUTRAL_CORE=true
```

A UI improvement, performance optimization, or future-platform abstraction MUST NOT weaken those boundaries.

# Production sequence — v0.2.1 Startup & Period Reliability

## Track objective

Make daily HookStat startup and period switching feel immediate while preserving exact evidence semantics.

The target lifecycle is:

```text
hookstat
   |
   v
enter terminal + render shell immediately
   |
   +--> loading / last accepted presentation state
   |
   v
background data pipeline
   +--> incremental receipt ingest
   +--> bounded ledger query / aggregate
   +--> reliability intelligence
   +--> independent diagnostics refresh
   |
   v
latest-request-wins immutable snapshot
   |
   v
atomic UI update
```

The product-level period selector becomes:

```text
Today | 24h | 7d | 30d | All
```

Semantics:

- `Today`: local calendar day from local midnight to now;
- `24h`: rolling 24 hours ending at now;
- `7d`: rolling 7 days ending at now;
- `30d`: rolling 30 days ending at now;
- `All`: all admitted history.

`Today` and `24h` MUST remain distinct types/semantics, including around timezone offset and daylight-saving transitions.

## Dependency DAG

```text
HS-G07 — Startup Observatory & Performance Contract
   |
   v
HS-G08 — Async First Frame & Data-Pipeline Separation
   |
   v
HS-G09 — Period UX & Today Semantics
   |
   v
HS-G10 — Bounded Ledger Reads & Incremental Receipt Ingestion
   |
   v
HS-G11 — Diagnostics Correctness & Lazy Refresh
   |
   v
HS-G12R — v0.2.1 Hardening & Release
```

Detailed plan: `../goals/HS-V021-STARTUP-PERIOD-RELIABILITY.md`.

## v0.2.1 train status

HS-G07 through HS-G10 merged on `b6480d835f33af58836e1722d27d5d22c5a9416f`.
The current HS-G11 candidate resolves Windows native/CMD/BAT/PowerShell Codex
command forms through a bounded read-only probe and keeps diagnostics refresh
independent from period changes. It is awaiting its own exact-head CI/merge
evidence. HS-G12R remains `NOT_STARTED`; it will version the accepted G11 main
as the unpublished v0.2.1 release candidate and will not begin v0.3 work.

## v0.2.1 confirmed requirements

### P0 — startup and correctness

- render a real interactive TUI shell before initial reliability aggregation completes;
- no SQLite query, receipt scan, diagnostics discovery, or analytics computation on the UI event-loop thread;
- add measured time-to-first-frame (TTFF), first-data-ready, warm-refresh, and period-switch latency instrumentation/benchmarks;
- replace process-lifetime anecdotes with repeatable startup measurements;
- fix Windows Codex binary detection for supported PATH-resolved executable and command-shim forms;
- add first-class `Today` period;
- make Today/24h/7d/30d/All a visible primary dashboard control, not a hidden Settings option;
- preserve latest-request-wins so a stale background result cannot overwrite a newer period request.

### P1 — bounded data work

- remove duplicate report construction on the normal TUI startup path;
- push bounded time selection down toward SQLite rather than loading all invocation rows for every finite window;
- preserve previous-period, revision, risk, and fingerprint correctness while bounding queries;
- make receipt ingestion incremental on the hot path while retaining crash safety and start-to-completion upgrade semantics;
- keep a bounded integrity sweep/reconciliation path so performance work cannot permanently hide malformed/orphaned evidence;
- decouple diagnostics refresh from ordinary period changes;
- avoid respawning/rediscovering Codex simply because the user changes 7d to 30d;
- preserve current period selection across normal refreshes and optionally across restarts when the persistence contract is safe.

### P2 / experimental

`HS-G13X` may investigate stale-while-revalidate snapshots and adjacent-period prefetch after v0.2.1. It is not a v0.2.1 release blocker unless later promoted.

# Optional maintenance lane — v0.2.2

v0.2.2 is need-based, not a mandatory predecessor to v0.3.

Candidates include:

- distinguish active/recent incomplete receipts from stale orphan starts;
- improve diagnostic explanations for Warning/Unknown/Unsupported;
- refine Human hook-name parsing from additional safe metadata;
- address narrow/CJK layout defects found after v0.2.1;
- add safe snapshot caching if `HS-G13X` proves useful;
- address real SQLite/receipt growth behavior after longer dogfood.

If no material maintenance need appears, proceed directly from accepted v0.2.1 to v0.3 planning/implementation.

# Production sequence — v0.3 Codex Reliability Workbench & Unified Human Interface

## Track objective

v0.3 remains Codex-only in production and turns the Reliability Center into a long-term Codex Hook reliability workbench while making its Human interface **contractually identical to TabBeacon where the interaction is not domain-specific**.

The phrase `TabBeacon-aligned` is retired for v0.3. The normative requirement becomes:

> **TabBeacon-compatible by contract and shared implementation.**

TabBeacon is the UI/UX reference implementation. HookStat may differ in reliability-domain content, but it MUST NOT independently invent a different application shell, navigation state machine, footer grammar, settings editing model, help-overlay behavior, key-event policy, or Human chrome style.

Detailed parity contract: `../docs/design/TABBEACON_UI_UX_PARITY_CONTRACT.md`.

Detailed v0.3 plan: `../goals/HS-V03-CODEX-RELIABILITY-WORKBENCH-UNIFIED-HUMAN-INTERFACE.md`.

## v0.3 dependency DAG

```text
accepted v0.2.1
   |
   v
HS-G20 — TabBeacon UI/UX Parity Audit
   |
   v
HS-G21 — Shared Human Interface Contract / Reuse Boundary
   |
   v
HS-G22 — Navigation, Footer, Settings & Overlay Convergence
   |
   v
HS-G23 — Shell, Header, Theme & Typography Convergence
   |
   v
HS-G24 — Codex Changes & History Workbench
   |
   v
HS-G25 — Hook Catalog & Failure Exploration
   |
   v
HS-G26 — Human Interface / Accessibility / Dogfood Hardening
   |
   v
HS-G27R — v0.3 Hardening & Release
```

## v0.3 P0 — exact TabBeacon UI/UX convergence

The following are release requirements, not suggestions:

- one current-screen model for top-level navigation;
- `Up/Down` and `j/k` switch top-level pages immediately when not in an editing/list-detail mode;
- remove HookStat's global Navigation/Content focus toggle as a top-level navigation requirement;
- remove selected-vs-active route divergence for the top-level sidebar;
- use one `>` current-screen marker and TabBeacon-compatible sidebar grammar;
- match normal shell geometry, header structure, sidebar width policy, and footer height to the reference contract;
- match TabBeacon footer sentence grammar and state replacement behavior instead of composing dot-separated shortcut tokens;
- adopt TabBeacon settings edit state: Enter edit/done, Up/Down field selection, Left/Right value change, `a` Apply, `r` Revert;
- require dirty-quit discard confirmation rather than silently losing staged settings;
- add the TabBeacon-style `?` Help overlay with the same overlay ownership/dismissal model;
- match press-only key handling and ignore Repeat/Release events;
- align Human locale resolution behavior, including actual OS locale lookup on Windows;
- align Human color/chrome policy and typography instead of maintaining an independent HookStat chrome palette;
- use parity render fixtures and an owned Windows Terminal A/B dogfood pack against the same reference baseline.

## v0.3 P0/P1 — Codex Reliability Workbench

Add a new `Changes` workbench surface and mature historical exploration:

- recent regression detection;
- recovery detection;
- new Hook detection;
- inactive/disappeared historical Hook classification where evidence supports it;
- revision timeline rather than only current-vs-previous;
- first-seen / last-seen / latest-evidence time;
- Hook catalog with current revision and historical status;
- failure-cluster browser using only bounded admitted metadata;
- compact trend visualization/sparklines where terminal width permits;
- explicit sample-confidence presentation;
- in-TUI safe Human alias editing;
- data freshness/currentness visible in the Human interface;
- preserve base counts, denominators, coverage, and insufficient-evidence states under every higher-level interpretation.

Recommended v0.3 top-level screens after parity convergence:

```text
Overview
Hooks
Changes
Diagnostics
Interface
```

Hook Detail remains a drill-down state rather than a separate top-level screen.

## Runtime-neutral architecture requirement

v0.3 may improve adapter/capability contracts and add fake-runtime contract tests, but MUST ship no new production runtime adapter.

Acceptance principle:

```text
ADDING_A_FUTURE_RUNTIME_SHOULD_NOT_REQUIRE_REWRITING:
- reliability ledger semantics
- failure denominator semantics
- risk scoring core
- trend semantics
- revision comparison core
- Reliability Center application shell
```

If a proposed future runtime exposes different capabilities, the capability/coverage model absorbs the difference rather than contaminating Codex production logic with unproven generic behavior.

# Future experimental runtime tracks — after v0.3

The following are future `X` tracks only:

```text
HS-G30X — Claude Code Adapter Qualification
HS-G31X — OpenCode Adapter Qualification
HS-G32X — Agy Adapter Qualification
HS-G33X — DeepSeek Harness Adapter Qualification
```

An X-track may research, prototype, build fixtures, and prove capability mappings. It MUST NOT become a production release dependency merely because code exists.

Promotion rule:

```text
experimental qualification
   -> real Owner usage
   -> evidence semantics proven
   -> explicit Owner promotion decision
   -> production adapter goal
```

Until promotion, Codex remains the only production runtime.

# Explicit non-goals through v0.3

- Claude Code production adapter;
- OpenCode production adapter;
- Agy production adapter;
- DeepSeek Harness production adapter;
- Web dashboard;
- cloud synchronization;
- global resident daemon;
- PTY/launcher/PATH interception for normal HookStat use;
- AI-generated diagnosis;
- remote telemetry service;
- distributed multi-machine HookStat;
- automatic ntfy alerting as a v0.3 release requirement.

# Performance and Human-interface acceptance philosophy

Each production Goal:

1. starts from one accepted predecessor;
2. declares the changed-risk vector;
3. uses focused tests while iterating;
4. settles one candidate before broad final gates;
5. reuses accepted unchanged-risk evidence;
6. runs one representative proof per material risk family rather than manufacturing traceability work;
7. records real dogfood separately from fixture-only evidence;
8. treats `UNKNOWN`, `UNSUPPORTED`, `PARTIAL`, and `INSUFFICIENT_HISTORY` as first-class truthful states;
9. never claims a visual or performance improvement without a reproducible measurement or owned terminal proof;
10. merges only after normative exit gates pass.

Long unattended trains MAY cover multiple adjacent Goals when dependency order is preserved and each Goal's acceptance state is recorded. They MUST NOT skip a blocker merely to consume the requested run time.

# v0.2.1 completion definition

```text
INITIAL_TUI_SHELL_BEFORE_DATA=true
UI_THREAD_BLOCKING_DATA_IO=false
PERIODS=Today,24h,7d,30d,All
TODAY_IS_LOCAL_CALENDAR_DAY=true
ROLLING_24H_DISTINCT_FROM_TODAY=true
LATEST_REQUEST_WINS=true
FINITE_WINDOW_FULL_HISTORY_LOAD=false
RECEIPT_HOT_PATH_INCREMENTAL=true
DIAGNOSTICS_PERIOD_SWITCH_COUPLED=false
WINDOWS_CODEX_SHIM_DETECTION=PASS
STARTUP_PERFORMANCE_CONTRACT=PASS
V02_BEHAVIOR_REGRESSION=PASS
WINDOWS=PASS
LINUX=PASS
PACKAGE=PASS
PUBLICATION=PASS
```

# v0.3 completion definition

```text
PRODUCTION_RUNTIME=Codex
NON_CODEX_RUNTIME_REQUIRED=false

TABBEACON_UI_REFERENCE=normative
TABBEACON_UI_UX_PARITY=PASS
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
OWNER_WINDOWS_TERMINAL_A_B_DOGFOOD=PASS
PACKAGE=PASS
PUBLICATION=PASS
```
