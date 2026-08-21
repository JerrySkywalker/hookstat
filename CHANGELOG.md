# Changelog

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
