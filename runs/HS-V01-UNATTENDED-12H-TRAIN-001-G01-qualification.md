# HS-V01-UNATTENDED-12H-TRAIN-001 — HS-G01 Qualification Receipt

## Result

```text
HS_G01=BLOCKED_DATA_SOURCE_DECISION_REQUIRED
RUNTIME=codex
RUNTIME_VERSION=0.147.0
EVIDENCE_SOURCE=none_admitted
COVERAGE=NOT_ADMITTED
HANDLER_IDENTITY_PROVEN=false
INVOCATION_DENOMINATOR_PROVEN=false
TERMINAL_STATUS_PROVEN=false
CODEX_MUTATED=false
RAW_PRIVATE_SESSION_CONTENT_COMMITTED=false
```

The v0.1 minimum requires a durable per-handler invocation denominator,
terminal status, and timestamp. No inspected source met all four properties.
This is a latched architectural result, not permission to add a wrapper,
daemon, live App Server dependency, or Codex configuration mutation.

## Sanitized qualification matrix

| Candidate surface | Durable after session | Handler identity | Denominator | Terminal status | Timestamp | Duration | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Active Codex session JSONL | Yes, where readable | No | No | No | Generic records only | No | Rejected |
| Archived Codex session JSONL | Yes | No | No | No | Generic records only | No | Rejected |
| Local App Server/state SQLite | Yes | No record present | No record present | No record present | N/A | N/A | Rejected |
| Local Codex logs | Yes | No structured run record | No | No | Log timestamp only | No | Rejected |
| Codex App Server notifications | No, live transport | Yes by protocol | Yes by protocol | Yes by protocol | Yes by protocol | Yes by protocol | Comparison only; rejected as retrospective source |
| Local OTel surface | Absent | N/A | N/A | N/A | N/A | N/A | Rejected |

## Read-only evidence summary

- The installation was `codex-cli 0.147.0`.
- Active-session sampling parsed 59,973 records from 64 readable recent files;
  16 contemporaneous files were exclusively held by Codex and skipped without
  retry or mutation. No structured hook-runtime field or hook-start/completion
  discriminator appeared.
- Archived-session sampling parsed 17,803 records from all 36 available files.
  No hook or handler field/discriminator appeared.
- The local state database had zero `app_server_history_snapshots` rows and zero
  `thread_timeline_ledger` rows. Its log table contained no hook-start or
  hook-completion marker rows in a metadata-only query.
- The locally generated schema for the installed App Server includes
  `HookStarted`, `HookCompleted`, `HookRunSummary`, `handlerType`, `eventName`,
  `durationMs`, and terminal status. The public protocol is a live JSON-RPC
  transport, so it cannot be adopted as a historical source without prohibited
  product-scope expansion.

No session lines, prompts, tool arguments, hook command text, credentials,
filesystem paths, or full transcripts are included in this receipt.

## Public upstream comparison

- The [Codex App Server protocol](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md)
  documents JSON-RPC notifications and hook discovery/configuration, not a
  retrospective hook-run ledger.
- The [hook runtime](https://github.com/openai/codex/blob/main/codex-rs/core/src/hook_runtime.rs)
  emits live hook start/completion events with runtime-native statuses; this
  establishes why the App Server was considered, but does not establish durable
  local retention.

## Safe non-claiming work completed

- Canonical privacy-preserving `HookInvocation`, handler identity, coverage,
  admission, and terminal-status model.
- Deterministic synthetic JSON/report path with same-event multi-handler tests.
- HookStat-owned SQLite ledger with idempotence, incremental, malformed-batch,
  cursor, and no-prompt/no-payload-schema tests. No CLI command opens it yet.
- Frozen-style blocked and synthetic rendering tests that keep sample counts
  with every rate and never render an empty Codex source as healthy.
