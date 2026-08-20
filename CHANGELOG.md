# Changelog

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
