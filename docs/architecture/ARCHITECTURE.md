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
