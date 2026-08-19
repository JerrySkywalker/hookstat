# HookStat Architecture

## Shape

HookStat starts as one Rust package with explicit internal modules. The design is multi-runtime-ready, while v0.1 implements only Codex.

```text
Runtime
  └─ EvidenceSource(s)
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

A runtime may expose multiple evidence sources. Historical/durable sources are preferred for the core product because HookStat is intended to work when the user opens it after the original agent sessions ended. Live sources such as App Server are supplementary unless evidence qualification proves otherwise.

## Canonical concepts

`Runtime`: Codex, DeepSeek Harness, OpenCode, future runtimes.

`EvidenceSource`: runtime-specific source with its own cursor, coverage class, and evidence semantics.

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

The canonical record should retain only fields required for reliability: runtime/version, session reference if necessary, handler identity, event, timestamps/duration when available, normalized terminal outcome/fault, sanitized error fingerprint/summary when safe, evidence source, and coverage. Raw prompts and tool payloads are not part of the analytics model.
