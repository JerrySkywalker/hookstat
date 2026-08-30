# HS-G41 — Live Runtime Hook Catalog

## Objective

Implement the v0.4 ephemeral current-runtime hook catalog and conservative join to HookStat reliability history.

This is the foundation for Hooks Control Center. It must not redesign analytics or persist raw runtime presentation metadata.

## Preconditions

```text
G40=PASS
V040_ROADMAP_AUTHORITY=true
CODEX_HOOKS_PARITY_MATRIX_COMPLETE=true
```

## Required architecture

Implement the semantics defined by `docs/architecture/RUNTIME_PRESENTATION_SNAPSHOT.md`.

Required capabilities:

```text
RuntimePresentationSnapshot
RuntimeEventPresentation
RuntimeHandlerPresentation
RuntimeCatalogIssue
RuntimeHandlerKind
ReliabilityJoinState
```

Exact Rust type names may differ.

## Runtime source

Use the official Codex `hooks/list` read surface already available through HookStat's App Server discovery path. Do not add filesystem guessing for plugin/managed layers.

The presentation parser may retain runtime-owned Human fields in memory for the lifetime of the snapshot, including matcher/source/command/MCP/type/trust values, but these values must not enter persistence or exports.

## Event compatibility

Do not constrain live catalog rendering to canonical `HookEvent`.

Required:

```text
ALL_RUNTIME_EVENTS_VISIBLE=true
UNKNOWN_EVENT_DROPPED=false
```

Audit current Codex `Interrupt`.

If reliable invocation/terminal mapping is proven, add canonical support with migrations/tests as required. If not, display it catalog-only with reliability unavailable.

## Join semantics

Current catalog is authoritative for current installation state.

Reliability history is joined conservatively into:

```text
Matched
NoHistory
Ambiguous
Unsupported
```

Never guess an ambiguous identity match.

Installed current hooks remain visible even with no history.

Historical hooks absent from the current catalog remain available only in historical/Changes surfaces.

## Runtime catalog resource lifecycle

Catalog loading/refresh is independent from period analytics.

Required:

```text
PERIOD_SWITCH_REDISCOVERS_RUNTIME=false
EXPLICIT_RUNTIME_REFRESH=true
CATALOG_ERROR_DOES_NOT_ERASE_HISTORICAL_ANALYTICS=true
RELIABILITY_ERROR_DOES_NOT_HIDE_RUNTIME_CATALOG=true
```

## Privacy/security gates

Prove:

```text
RAW_RUNTIME_PRESENTATION_LEDGER_WRITES=0
RAW_RUNTIME_PRESENTATION_RECEIPT_WRITES=0
RAW_RUNTIME_PRESENTATION_DIAGNOSTICS_EXPORT=0
RAW_RUNTIME_PRESENTATION_NETWORK_EGRESS=0
SNAPSHOT_DEBUG_LOG_FULL_CONTENT=false
NO_THIRD_EVIDENCE_PATH=true
```

## Tests

At minimum:

- command handler;
- MCP tool handler;
- Prompt handler;
- Agent handler;
- matcher/source values;
- trusted/untrusted/modified/managed states;
- enabled/disabled;
- runtime warnings/errors;
- unknown event;
- current `Interrupt` disposition;
- Matched/NoHistory/Ambiguous joins;
- failed catalog discovery;
- refresh ownership;
- no persistence leakage.

## Acceptance

```text
LIVE_RUNTIME_HOOK_CATALOG=PASS
CODEX_READ_FIELD_PARITY=PASS
INSTALLED_UNOBSERVED_HOOKS_VISIBLE=true
UNKNOWN_RUNTIME_EVENTS_VISIBLE=true
AMBIGUOUS_JOIN_FALSE_ATTRIBUTION=0
HISTORICAL_NOT_INSTALLED_DISTINCT=true
RAW_RUNTIME_PRESENTATION_PERSISTED=false
NO_THIRD_EVIDENCE_PATH=true
CI=PASS
INDEPENDENT_REVIEW=PASS
```

## Next

G42 and G43 may proceed after the snapshot/view-model contracts settle.
