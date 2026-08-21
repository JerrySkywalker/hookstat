# HookStat Architecture

## Shape

HookStat starts as one Rust package with explicit internal modules. The design is multi-runtime-ready, while v0.1 implements only Codex.

```text
Runtime
  └─ EvidenceSource(s)
       ├─ PassiveEvidenceSource (preferred durable receipts/logs)
       └─ InstrumentedEvidenceSource (opt-in proxy receipt spool)
         ↓
      RawEvidence
         ↓
   Runtime normalizer
         ↓
    HookInvocation
         ↓
  local SQLite ledger
         ↓
     analytics
         ↓
   CLI / JSON / TUI
```

A runtime may expose multiple evidence sources. `PassiveEvidenceSource` is
preferred: it recovers runtime-owned durable evidence without changing hooks.
`InstrumentedEvidenceSource` is an opt-in fallback when a runtime exposes no
sufficient passive per-handler ledger. Both normalize into exactly the same
`HookInvocation` and neither changes analytics/storage/TUI contracts. Live
sources such as App Server are supplementary unless qualification proves them
durable.

For Codex v0.1, the read-only App Server `hooks/list` surface is a discovery
and coverage-reconciliation plane, not a historical evidence source. It can
classify effective user/project/plugin/managed handlers and exposed
enabled/trust state without putting command/path/matcher text in HookStat
output. The receipt spool remains the admitted durable invocation plane after
explicit opt-in instrumentation.

## Canonical concepts

`Runtime`: Codex, DeepSeek Harness, OpenCode, future runtimes.

`EvidenceSource`: runtime-specific source with its own cursor, coverage class, and evidence semantics.

`PassiveEvidenceSource`: read-only recovery from a runtime-owned durable
surface. Future DeepSeek Harness support should prefer this class when it has
durable receipts.

`InstrumentedEvidenceSource`: an explicitly enabled, runtime-native handler
proxy that forwards streams/exit status without inspecting their contents and
writes atomic HookStat-owned metadata receipts. It is not a launcher wrapper or
daemon.

`HandlerIdentity`: stable identity for a specific hook handler, not merely an event name.

`HookInvocation`: one normalized handler execution. Runtime-native terminal status is preserved and mapped to execution/fault semantics without assuming all runtimes use identical control-flow conventions.

`Coverage`: explicit statement of the evidence denominator. Candidate classes include COMPLETE, PARTIAL, SYNC_ONLY, BEST_EFFORT, and UNKNOWN.

## v0.1 module target

```text
src/
  cli.rs
  domain/
  runtime/
    codex/
  evidence/
  ingest/
  store/
  analytics/
  tui/
```

These are modules, not separate crates. A workspace split requires evidence from a real second runtime.

## Privacy

The canonical record retains only reliability metadata: runtime, handler
key/revision/source kind/event/matcher fingerprint/structural identity/execution
mode, timestamps/duration when available, normalized terminal outcome/fault,
evidence source, and coverage. Raw prompts, tool payloads, hook command text,
and stdin/stdout/stderr are not part of the analytics model.

## v0.2.1 startup and bounded-reconciliation model

The Reliability Center starts with a terminal-independent `Loading` model.
Terminal entry and the first draw occur before receipt reconciliation, SQLite
queries, reliability aggregation, runtime discovery, or diagnostics checks.
The UI event-loop thread only renders immutable view models and handles input.

```text
terminal guard -> empty application shell -> first frame
                                      |             |
                                      |             +-> input stays responsive
                                      v
                         reliability coordinator (background)
                         - incremental receipt reconciliation
                         - bounded SQLite working-set query
                         - analytics and immutable snapshot

                         diagnostics coordinator (independent background path)
                         - read-only diagnostics snapshot
```

Each request has a monotonically increasing generation. The refresh transport
coalesces pending work to the newest generation, and the application also
rejects a completed snapshot whose period no longer equals the visible
requested period. Failed refreshes preserve the last accepted reliability or
diagnostics view rather than clearing it.

The normal finite-period query materializes only the most recent 60 days—the
largest current-plus-previous range needed for the 30-day trend. `All` is the
explicit full-history mode. All-time trend metrics and current/previous
revision epochs are calculated by specialized SQLite aggregates/timeline
boundary queries, so finite requests retain the released semantics without
materializing all historical invocation rows.

Instrumented receipt records remain the canonical durable files. Existing
v0.2 spools receive one safe full reconciliation on migration. Later proxy
writes append a compact HookStat-owned journal record after publishing the
atomic receipt; the SQLite reconciliation cursor and accepted ledger rows
commit together. An unchanged warm start reads no historical receipt files.
An explicit full reconciliation remains available to validate canonical files
after an interrupted or damaged journal; duplicate replay and a later
completion retain the existing guarded idempotence/upgrade semantics.

The local-only observatory records sanitized phase timing and work counts:
process/terminal/first-frame/receipt/query/reliability/diagnostics phases,
manual-refresh and period-request latency, receipt files inspected/parsed,
bounded ledger rows materialized, query range, and request/accepted
generations. It transports no telemetry and retains no private runtime
content.
