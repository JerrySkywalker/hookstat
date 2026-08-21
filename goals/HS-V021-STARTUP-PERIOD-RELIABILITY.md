# HookStat v0.2.1 — Startup & Period Reliability

## Status

PLANNED after public v0.2.0 (`ba1d456d310982d816cd0e7fbfd7b67423c34311`).

## Product objective

Make HookStat feel immediate in daily Codex use while preserving the exact reliability semantics already accepted in v0.2.

The user must be able to enter the Reliability Center before expensive data work completes, switch among useful time periods without blocking the UI, and trust that every displayed result comes from the latest requested period and admitted evidence.

v0.2.1 is a performance/correctness release. It MUST NOT add another production runtime or redesign the Reliability Center into a different product.

## Planning baseline

Owned post-release dogfood:

```text
HOOKSTAT_VERSION=0.2.0
REPORT_RW_MS=788.6
REPORT_RO_MS=580.4
DOCTOR_MS=1026.3
LEDGER_BYTES=2568192
RECEIPT_FILES=6769
DOCTOR_CODEX_BINARY=FAIL
```

The existing TUI already has an asynchronous refresh worker after startup, but its initial snapshot is built synchronously before the terminal loop starts. The ledger currently supports full invocation loading, and the receipt spool can rescan all record files. Those behaviors are acceptable as historical implementation facts, not as v0.2.1 target behavior.

## Invariants

```text
PRODUCTION_RUNTIME=Codex
NORMAL_CODEX_LAUNCH=codex
GLOBAL_DAEMON_BASELINE=false
OPT_IN_INSTRUMENTATION=true
HOOK_TRUST_BYPASS=false
RAW_PRIVATE_CONTENT_PERSISTED=false
COVERAGE_TRUTHFUL=true
FAILURE_RATE_WITH_SAMPLE_COUNT=true
UNKNOWN_IS_NOT_HEALTHY=true
```

Performance work MUST NOT delete, ignore, or reclassify evidence merely to become faster.

## Period contract

v0.2.1 supports five first-class periods:

```text
Today
24h
7d
30d
All
```

Semantics:

### Today

Local calendar interval from the current local day's midnight to `now`.

- based on the user's local civil time, not UTC midnight;
- distinct from rolling 24 hours;
- must behave deterministically across timezone-offset and DST transitions;
- machine JSON must identify the period with a stable locale-neutral value.

### 24h

Rolling `[now - 24h, now]` interval.

### 7d

Rolling `[now - 7d, now]` interval.

### 30d

Rolling `[now - 30d, now]` interval.

### All

All admitted history. No fabricated previous period is allowed for All.

## Target hot-path architecture

```text
main
  |
  +--> parse bounded CLI/interface preferences
  +--> enter terminal guard
  +--> create application shell in Loading / accepted cached state
  +--> first draw
  |
  v
background data coordinator
  +--> incremental receipt ingest
  +--> bounded ledger query/aggregate
  +--> reliability intelligence
  +--> publish immutable reliability snapshot

independent diagnostics worker
  +--> binary/runtime/discovery checks on its own cadence or explicit refresh
  +--> publish immutable diagnostics snapshot
```

The TUI thread renders and handles input. It MUST NOT perform receipt filesystem scans, SQLite data queries, runtime discovery, Codex process probes, or reliability aggregation.

# Goal DAG

```text
HS-G07 — Startup Observatory & Performance Contract
  ↓
HS-G08 — Async First Frame & Data-Pipeline Separation
  ↓
HS-G09 — Period UX & Today Semantics
  ↓
HS-G10 — Bounded Ledger Reads & Incremental Receipt Ingestion
  ↓
HS-G11 — Diagnostics Correctness & Lazy Refresh
  ↓
HS-G12R — v0.2.1 Hardening & Release
```

# HS-G07 — Startup Observatory & Performance Contract

## Objective

Measure the real startup pipeline before optimizing it, establish reproducible timing phases, and freeze performance gates based on owned hardware rather than intuition.

## Deliverables

Add a development/diagnostic timing surface or benchmark harness that can distinguish at least:

```text
process start
terminal guard entered
first frame drawn
receipt ingest ready
ledger/query ready
reliability snapshot ready
diagnostics ready
```

Required metrics:

- TTFF: process start -> first frame;
- first reliability data ready;
- warm manual refresh;
- period-switch request -> accepted snapshot;
- diagnostics refresh latency;
- receipt files inspected/parsed on the hot path;
- invocation rows loaded/materialized for each finite period.

Do not write telemetry to a remote service. Local benchmark output and deterministic test counters are sufficient.

## Acceptance

```text
STARTUP_PHASES_MEASURABLE=true
TTFF_MEASURABLE=true
DATA_READY_MEASURABLE=true
PERIOD_SWITCH_LATENCY_MEASURABLE=true
RECEIPT_HOT_PATH_WORK_MEASURABLE=true
LEDGER_ROWS_MATERIALIZED_MEASURABLE=true
REMOTE_TELEMETRY=false
```

At G07 close, record baseline and freeze realistic G12R thresholds. The planning target is approximately <=250 ms p50 TTFF on the owned Windows workstation, but G07 evidence is authoritative; do not fake a pass by relaxing measurement semantics.

# HS-G08 — Async First Frame & Data-Pipeline Separation

## Objective

Make first interactive render independent from the initial reliability snapshot.

## Required behavior

- terminal guard and application shell start without waiting for receipt/ledger/diagnostics work;
- initial state may be `Loading`, or a previously accepted local presentation cache if a later accepted design supplies one;
- input remains responsive while initial reliability data is pending;
- refresh worker starts from an empty/loading application model rather than requiring a precomputed `RefreshSnapshot`;
- reliability and diagnostics may complete independently;
- a reliability refresh failure does not erase the last accepted snapshot;
- stale worker responses cannot overwrite a newer generation/request.

Use generation/request IDs or equivalent deterministic ownership.

## Non-goals

- no daemon;
- no remote cache;
- no new runtime;
- no data deletion;
- no hiding partial coverage.

## Acceptance

```text
FIRST_FRAME_REQUIRES_RELIABILITY_SNAPSHOT=false
FIRST_FRAME_REQUIRES_DIAGNOSTICS=false
UI_THREAD_BLOCKING_DATA_IO=false
INPUT_RESPONSIVE_DURING_INITIAL_LOAD=true
LAST_ACCEPTED_VIEW_PRESERVED_ON_REFRESH_FAILURE=true
LATEST_REQUEST_WINS=true
GLOBAL_DAEMON=false
```

# HS-G09 — Period UX & Today Semantics

## Objective

Make period selection a primary Reliability Center interaction inspired by fast analytics dashboards rather than a hidden configuration detail.

## Required presentation

All primary reliability pages must expose the current period clearly:

```text
Today | 24h | 7d | 30d | All
```

The exact visual grammar must later remain compatible with the TabBeacon Human Interface parity contract where shared UI chrome is concerned.

## Keyboard contract

At minimum:

```text
t     Today
1     24h
7     7d
3     30d
a     All
```

Left/Right may cycle adjacent periods when focus/edit semantics do not conflict with the accepted TabBeacon-compatible interaction model. If G09 is implemented before v0.3 UI convergence, preserve compatibility so v0.3 can migrate without changing period semantics.

## State behavior

- period change gives immediate visible input feedback;
- data result may arrive later from the background worker;
- old results remain visually distinguishable as stale/loading until replacement;
- switching quickly `7d -> 30d -> Today` cannot let an old 7d/30d result replace Today;
- period selection survives normal data refresh;
- optional period persistence across process restarts is allowed only if it does not complicate locale/interface preference ownership.

## Acceptance

```text
TODAY_PERIOD=PASS
ROLLING_24H_PERIOD=PASS
TODAY_NE_24H=true
PERIOD_SELECTOR_PRIMARY=true
PERIOD_SWITCH_NONBLOCKING=true
LATEST_REQUEST_WINS=true
PERIOD_STATE_PRESERVED_ON_REFRESH=true
I18N_EN_US=PASS
I18N_ZH_CN=PASS
```

# HS-G10 — Bounded Ledger Reads & Incremental Receipt Ingestion

## Objective

Remove full-history/full-spool work from finite-period startup and refresh hot paths without changing accepted reliability results.

## Ledger requirements

Finite windows MUST avoid materializing all historical invocation rows merely to discard most of them in Rust.

Use SQLite indexes, range predicates, aggregate queries, specialized history queries, or equivalent evidence-preserving approaches.

Correctness requirements:

- current period counts/rates match v0.2 semantics;
- previous-period comparison remains exact;
- risk score remains exact;
- failure fingerprint clustering remains bounded and exact for the selected window;
- revision comparison/timeline queries may use a separate bounded/specialized query and MUST NOT invent a previous revision;
- `All` is allowed to use all history, but should still prefer database aggregation over unnecessary row materialization where practical.

Add/adjust indexes only with migration safety and measured justification.

## Receipt ingestion requirements

The hot path MUST stop reparsing every historical receipt file on every startup once records have already been durably reconciled.

The design must preserve:

- crash safety;
- duplicate idempotence;
- start-only `incomplete` semantics;
- later completion upgrade for the same invocation;
- malformed receipt visibility;
- best-effort completion-without-start handling;
- ability to perform a bounded/full integrity reconciliation outside the startup hot path.

Possible implementation strategies include a durable spool cursor/index/catalog, file metadata journal, ledger-backed processed-record state, or another proven design. Do not assume filename lexical order is a safe cursor without proving it.

## Acceptance

```text
FINITE_WINDOW_FULL_HISTORY_LOAD=false
FINITE_WINDOW_DATABASE_BOUNDED=true
PREVIOUS_PERIOD_SEMANTICS_UNCHANGED=true
REVISION_SEMANTICS_UNCHANGED=true
RISK_SEMANTICS_UNCHANGED=true
RECEIPT_HOT_PATH_INCREMENTAL=true
RECEIPT_START_COMPLETION_UPGRADE=PASS
MALFORMED_EVIDENCE_REMAINS_VISIBLE=true
CRASH_SAFETY=PASS
IDEMPOTENCE=PASS
```

Include a scale fixture at or above the owned baseline of ~6,769 receipt files and enough historical rows to prove startup work does not scale linearly with all old files for the common warm path.

# HS-G11 — Diagnostics Correctness & Lazy Refresh

## Objective

Fix known correctness problems and make diagnostics independent from ordinary period changes.

## Windows Codex binary detection

Owned dogfood observed:

```text
CodexBinary=Fail
```

while Codex remained usable from the user's PowerShell environment.

The current direct spawn assumption must be audited against Windows PATH forms including at least:

- native executable;
- `.cmd` command shim commonly created by npm/global tooling;
- `.bat` where relevant;
- missing binary;
- resolved but failing command;
- timeout.

The fix MUST be deterministic and safe. No user-controlled shell command string may be interpolated unsafely merely to support `.cmd` resolution.

## Diagnostics scheduling

Normal period changes (`Today`, `7d`, etc.) MUST NOT automatically rerun expensive runtime discovery or Codex version probes when the underlying diagnostic state is independent from the time window.

Diagnostics may refresh:

- on entry to Diagnostics when stale;
- on explicit refresh;
- on a bounded independent cadence;
- after known local state mutation by HookStat.

The exact policy should minimize repeated work while keeping freshness visible.

## Receipt integrity refinement

If time permits without expanding scope, distinguish recent/possibly-active incomplete starts from stale orphan starts. This is secondary to the Codex detection bug and decoupling.

## Acceptance

```text
WINDOWS_CODEX_EXE_DETECTION=PASS
WINDOWS_CODEX_CMD_SHIM_DETECTION=PASS
MISSING_CODEX_CLASSIFICATION=PASS
CODEX_DETECTION_FALSE_FAIL_CLOSED=true
DIAGNOSTICS_PERIOD_SWITCH_COUPLED=false
DIAGNOSTICS_READ_ONLY=true
TRUST_MUTATED=false
INSTRUMENTATION_MUTATED=false
```

# HS-G12R — v0.2.1 Hardening & Release

## Objective

Settle one exact-head v0.2.1 candidate and prove that performance improvements did not corrupt evidence or regress the released v0.2 Human interface.

## Required gates

- G07-G11 acceptance complete;
- exact-head Windows and Linux CI;
- locked format/clippy/tests/build/package/publish dry-run;
- owned Windows Terminal startup/period smoke;
- owned scale test using a copy or safe read-only view of the real HookStat data root;
- compare old/new results for representative Today/24h/7d/30d/All fixtures and real data where available;
- package/install smoke;
- separate Owner authorization for public publication.

Performance threshold values are frozen from G07 and measured again on the same class of owned environment.

## Release acceptance

```text
INITIAL_TUI_SHELL_BEFORE_DATA=true
UI_THREAD_BLOCKING_DATA_IO=false
TODAY_PERIOD=PASS
PERIODS=Today,24h,7d,30d,All
LATEST_REQUEST_WINS=true
FINITE_WINDOW_FULL_HISTORY_LOAD=false
RECEIPT_HOT_PATH_INCREMENTAL=true
DIAGNOSTICS_PERIOD_SWITCH_COUPLED=false
WINDOWS_CODEX_SHIM_DETECTION=PASS
STARTUP_PERFORMANCE_CONTRACT=PASS
RELIABILITY_RESULT_EQUIVALENCE=PASS
COVERAGE_TRUTHFUL=true
V02_HUMAN_INTERFACE_REGRESSION=PASS
WINDOWS=PASS
LINUX=PASS
CARGO_PACKAGE=PASS
CARGO_PUBLISH_DRY_RUN=PASS
PUBLICATION_AUTHORIZED=false
```

## Experimental follow-up — HS-G13X

After v0.2.1, an experimental track may evaluate:

- stale-while-revalidate accepted presentation snapshots;
- adjacent-period prefetch;
- lightweight prepared aggregates;
- receipt archive/compaction strategies.

`HS-G13X` MUST NOT block v0.3 unless explicitly promoted by the Owner.
