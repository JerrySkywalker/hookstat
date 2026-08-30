# Changelog

## 0.3.1 – HSIP Infrastructure & Windows Hardening

HookStat v0.3.1 is the current public release. This entry records the
single-package HookStat substrate released without bundling, admitting, or
modifying an external cooperative producer.

### HSIP and recovery hardening

- adds the in-repository HSIP v1 reference producer and conformance kit for
  real bounded broker transport, START/COMPLETE correlation, duplicate and
  out-of-order semantics, malformed/oversized ingress, broker absence/restart,
  uncertain-write/ACK no-replay, privacy, and runtime-neutral authority checks;
- preserves the frozen HookStat substrate performance gate of P95 <= 1 ms,
  P99 <= 2 ms, and zero observation gaps; accepted G38 qualification retained
  a worst accepted series of P50 0.1048 ms, P95 0.3254 ms, P99 0.7124 ms, and
  diagnostic maximum 2.8944 ms;
- hardens bounded, read-only broker diagnostics, reconnect/startup-race and WAL
  valid-prefix/partial-tail recovery, duplicate protection, and process cleanup
  without adding a third evidence path or activating the transparent shim;
- retains a bounded 50 ms broker idle-read window after the 25 ms producer
  reuse cutoff, preserving short START/COMPLETE reuse while reclaiming stale
  Windows pipe slots before a later COMPLETE reconnects at the connection cap;
  ambiguous lifecycle delivery remains fail-open and no-replay.

### Availability, migration, and privacy

- ships HSIP infrastructure and conformance only: a named external producer is
  independently qualified and admitted before it can become authority; domains
  without an admitted Native source or named IPC integration remain
  `NOT_ADMITTED`;
- preserves v0.3 ledgers, receipt history, historical failed/incomplete states,
  aliases, revision history, and interface preferences through additive,
  idempotent migration coverage;
- keeps normal `codex` launch unchanged, keeps instrumentation/trust explicit,
  and retains local-first privacy: no raw prompts, tool payloads, commands,
  standard streams, credentials, or network telemetry are persisted.

### Publication boundary

- records the package and publish-dry-run validation that preceded the
  Owner-authorized v0.3.1 publication, public tag, and GitHub Release; future
  publication, tag, and release actions remain explicit Owner gates.

## 0.3.0 — Codex Reliability Workbench

HookStat v0.3.0 is the public release for the Codex Reliability Workbench and
the shared TabBeacon-compatible Human interface. Its immutable crate, tag, and
GitHub Release bind to the exact release commit.

### Reliability workbench

- adds a coverage-aware Changes workbench with first/last/latest admitted
  evidence, conservative regression/recovery classifications, and ordered
  revision timelines;
- adds a Hook Catalog with safe local Human aliases, selected-period confidence,
  visible data freshness, compact trends, and bounded failure-cluster drill-down;
- preserves Today, 24h, 7d, 30d, and All period semantics, latest-request-wins
  refreshes, asynchronous first frame, and independent diagnostics.

### Unified Human interface and boundaries

- uses the registry-published `terminal-ui-contract` 0.1.0 shared boundary for
  TabBeacon-compatible shell, navigation, footer, editing, Help, locale, and
  terminal primitives;
- preserves ordinary `codex` launch, Codex-only production scope, opt-in
  instrumentation and explicit trust, truthful coverage, sample-counted failure
  rates, and local-first privacy with no raw private content or telemetry.

## 0.2.1 — Startup & Period Reliability

HookStat v0.2.1 is the public release focused on making the
Reliability Center responsive at real ledger/receipt scale without weakening
evidence semantics.

### Startup, periods, and evidence pipeline

- draws an interactive loading shell before receipt reconciliation, SQLite
  queries, analytics, or diagnostics work; worker responses use
  latest-request-wins ownership and preserve an accepted view on refresh
  failure;
- adds first-class `Today`, `24h`, `7d`, `30d`, and `All` periods. `Today` is
  the local civil day and is never an alias of rolling 24 hours;
- bounds finite-window SQLite materialization while preserving released
  period, risk, failure-fingerprint, revision, coverage, and comparison
  semantics; and changes the common receipt warm path to incremental durable
  journal reconciliation;
- adds local-only sanitized startup/work observability with no remote
  telemetry or private runtime content.

### Diagnostics correctness

- resolves Windows native, CMD/BAT, and PowerShell Codex command forms with
  bounded literal version probes, avoiding the prior PATH-shim false fail;
- keeps diagnostics generation-owned and read-only. Initial diagnostics load
  independently, while reliability period changes do not rerun Codex probing
  or runtime discovery.

### Compatibility and privacy

- preserves normal `codex` launch, opt-in instrumentation, trust boundaries,
  privacy guarantees, and truthful partial/unknown evidence states;

## 0.2.0

HookStat v0.2 turns the v0.1 report into the Reliability Center: a
TabBeacon-aligned, local-first terminal UI for interpreting admitted hook
evidence.

### Reliability Center and Human interface

- adds the stable application shell, semantic theme, four-region navigation,
  localized footer, responsive layouts, asynchronous refresh, and terminal
  restoration guard;
- adds Overview, Hooks, Hook Detail, read-only Diagnostics, and Settings
  views with `en-US` and `zh-CN` runtime switching and persisted `auto`/
  locale/color preferences;
- resolves primary hook names from safe aliases and admitted metadata before
  falling back to sanitized basenames or localized event names. Internal
  `hk_*` keys remain metadata in Detail/Diagnostics only, and same-event
  hooks receive stable human disambiguators;
- adds selected-row viewport following plus `PgUp`/`PgDn` page navigation for
  long hook lists and scrollable detail intelligence on short terminals.

### Diagnostics and intelligence

- adds read-only `hookstat doctor`, `hookstat doctor --json`, and sanitized
  `hookstat diagnostics export`; diagnostics distinguish pass, warning, fail,
  unknown, and unsupported without repairing configuration or changing trust;
- adds deterministic 24h/7d/30d/All projections, previous-period comparisons,
  coverage/insufficient-history states, regression classification, bounded
  failure fingerprints, and real revision comparisons only when the ledger
  supplies prior admitted evidence;
- replaces percentage-only ranking with an interpretable risk score that
  considers failure rate, sample confidence, coverage, recency/trend, and
  impact. Failure rates remain adjacent to their terminal sample denominator.

### Compatibility and privacy

- retains stable handler keys, receipt format, ledger schema compatibility,
  analytics boundaries, normal `codex` launch, opt-in instrumentation, exact
  restore semantics, and separate scoped trust behavior;
- continues to exclude prompts, tool payloads, credentials, raw hook commands,
  stdout, stderr, and private session content from the ledger, report,
  diagnostics export, and committed evidence.

## 0.1.0

First public HookStat release: Codex-first, local-first hook reliability
analytics with opt-in transparent instrumentation. Normal daily launch remains
`codex`; HookStat reports per-handler runs, failures, and failure rates with
24h/7d/30d/All views and a Ratatui TUI. Plugin/managed sources that v0.1 cannot
instrument remain visible as explicit unsupported coverage. Receipts and
reports retain metadata only, never prompts, tool payloads, or stream content.
Instrumentation is backed by exact restore and does not bypass Codex trust.
The separate explicit scoped-trust action uses Codex's official App Server
mechanism only for current HookStat-generated supported user hooks. Windows
proxies use Job Object containment to clean up active handler trees on forced
proxy termination while preserving legitimate descendants after normal handler
root completion. Unix does not claim this Windows-specific containment guarantee
in v0.1.

### Windows Codex 0.147 quote compatibility recovery

- writes a Windows-only proxy command using a canonical URL-safe manifest token
  and strict safe handler key, avoiding embedded quotes under Codex's
  `cmd.exe /C "<command>"` launch form;
- preserves the portable quoted command and exact byte-for-byte restore while
  adding `commandWindows` for every wrapped command handler;
- adds a local Codex-spawn-form regression covering spaces, non-ASCII paths,
  receipt start/completion, original exit propagation, malformed tokens, and
  handler-key injection rejection.

### Pre-activation hardening

- adds read-only Codex App Server effective-hook discovery and static/effective
  coverage reconciliation without exposing commands or paths;
- makes plugin/managed runtime handlers explicit unsupported coverage;
- hardens instrumented receipt atomicity, duplicate handling, incomplete
  coverage, Windows shell invocation, full exit-status propagation, and TUI
  refresh/terminal restoration;
- adds Owner activation and rollback runbook.

## 0.1.0 — instrumented evidence recovery

- adds opt-in transparent Codex command-handler receipts, source-neutral SQLite
  analytics/reporting, fixture apply/restore, and the normative Ratatui TUI.
