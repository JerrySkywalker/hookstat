# Runtime Presentation Snapshot Architecture

## Purpose

HookStat v0.4 needs to display the current runtime hook configuration with the same human usefulness as the runtime's own hook browser while preserving HookStat's established privacy contract.

The solution is an **ephemeral runtime presentation snapshot**: a local, in-memory view of runtime-owned presentation metadata that is joined to HookStat's persisted, privacy-preserving reliability model only at presentation time.

## Why a separate snapshot is required

HookStat v0.3.1 intentionally reduces runtime metadata before persistence. Commands, raw matcher expressions, source paths, plugin identifiers, prompt/tool content, and similar values do not enter the ledger or receipts.

That design is correct for analytics but insufficient for a current Hook control surface. A user needs to see values such as source, matcher, command, handler type, timeout, and trust to understand the current runtime configuration.

The v0.4 architecture therefore separates:

```text
Persistence-safe reliability facts
from
Presentation-sensitive current runtime facts
```

## Data flow

```text
Codex hooks/list (or future runtime equivalent)
        │
        ▼
Runtime adapter presentation parser
        │
        ▼
RuntimePresentationSnapshot
        │
        ├──────────────────────────────┐
        │                              │
        ▼                              ▼
Current catalog projection      Reliability join resolver
                                       │
                                       ▼
                               Ledger / analytics facts
        │                              │
        └───────────────┬──────────────┘
                        ▼
              HooksControlCenterViewModel
                        │
                        ▼
                       TUI
```

## Persistence contract

Normative:

```text
SNAPSHOT_IN_MEMORY_ONLY=true
SNAPSHOT_SERIALIZE_DURABLY=false
SNAPSHOT_LEDGER_WRITE=false
SNAPSHOT_RECEIPT_WRITE=false
SNAPSHOT_DIAGNOSTICS_EXPORT=false
SNAPSHOT_PERFORMANCE_RECEIPT=false
SNAPSHOT_NETWORK_EGRESS=false
```

The snapshot may be recreated on demand from the runtime's official read surface.

It is not a new reliability evidence transport and cannot become denominator authority merely because a value is visible in the TUI.

```text
RUNTIME_PRESENTATION_IS_EVIDENCE_PATH=false
NO_THIRD_EVIDENCE_PATH=true
```

## Suggested model

Names are non-normative, semantics are normative.

```text
RuntimePresentationSnapshot
  runtime
  captured_at
  context
  events[]
  warnings[]
  errors[]

RuntimeEventPresentation
  runtime_event_name
  canonical_event: Option<HookEvent>
  description
  installed
  active
  needs_review
  handlers[]

RuntimeHandlerPresentation
  runtime_catalog_id
  canonical_join_hint
  enabled
  managed
  needs_review
  trust
  matcher
  source
  handler
  mode
  timeout
  additional_context_limit

RuntimeHandlerKind
  Command { command }
  McpTool { server, tool }
  Prompt
  Agent
  Unknown { label }
```

Fields containing presentation-sensitive runtime strings must not implement accidental durable serialization merely for convenience. Prefer private/internal types and explicit projection into the TUI model.

## Event model

The runtime presentation layer must not be constrained to the current canonical reliability enum.

Use two concepts:

```text
RuntimeEventName = runtime-owned current event identity
CanonicalHookEvent = HookStat reliability taxonomy when proven
```

Mapping examples:

```text
PreToolUse -> Some(HookEvent::PreToolUse)
SessionStart -> Some(HookEvent::SessionStart)
new/unknown runtime event -> None
```

An unmapped runtime event remains visible in the catalog. Its reliability overlay is unavailable or explicitly not admitted.

This prevents UI information loss when a runtime adds an event before HookStat has reliability semantics for it.

## Interrupt

The current Codex hook surface includes `Interrupt`, while HookStat v0.3.1's canonical event taxonomy does not.

G41 must determine:

1. whether Codex exposes `Interrupt` in `hooks/list` in the pinned baseline;
2. whether HookStat's admitted evidence sources can observe its invocation and terminal semantics reliably;
3. whether it should be added to canonical `HookEvent` or remain presentation-only initially.

Do not add it to reliability denominators solely because it appears in configuration discovery.

## Runtime issues

Runtime discovery warnings/errors are current configuration facts, not reliability failures.

Represent them separately:

```text
RuntimeCatalogIssue
  severity
  human_message
  optional sanitized/source presentation context
```

They may be displayed in the local TUI. They must not automatically become historical HookInvocation failures.

## Join architecture

The current runtime catalog and historical ledger have different identity guarantees.

Define a conservative join outcome:

```text
ReliabilityJoinState
  Matched(handler_key)
  NoHistory
  Ambiguous
  Unsupported
```

The join may use runtime-owned stable keys, HookStat fingerprints, current revision hash, event, source class, or other proven bounded identity material. The exact algorithm is runtime-adapter-owned.

Rules:

- never guess when multiple historical handlers match;
- never attribute one handler's history to another because display labels are similar;
- `NoHistory` is a first-class state;
- `Ambiguous` keeps runtime configuration visible and suppresses misleading reliability attribution;
- raw source/command text is not persisted merely to improve joining.

## Current versus historical truth

Current installation status is derived only from the runtime presentation snapshot.

Historical state is derived only from admitted HookStat data.

Do not infer:

```text
ledger row exists -> hook is installed now
no ledger row -> hook is not installed
```

The joined view explicitly models both axes.

## Refresh lifecycle

The TUI may refresh the snapshot:

- at Hooks Control Center startup/load;
- when the user presses the explicit refresh key;
- after an admitted runtime mutation succeeds;
- when changing working context if runtime discovery is context-sensitive.

Ordinary period switching must not unnecessarily re-run runtime discovery.

The runtime catalog should have its own resource/loading/error state, independent from reliability-period refresh.

## Failure behavior

If runtime catalog discovery fails:

- retain last accepted snapshot only if clearly marked stale and policy permits;
- otherwise show runtime catalog unavailable;
- historical HookStat reliability remains usable;
- do not fabricate installed/active/trust values from history.

If reliability loading fails:

- current runtime catalog remains usable;
- reliability overlay is unavailable;
- runtime truth is not hidden.

## Write operations

Runtime mutations are separate from the read snapshot.

A write controller, if admitted, must operate on exact runtime-owned identity/precondition data from the current snapshot. After success it refreshes the official catalog.

Do not mutate the snapshot optimistically without a runtime confirmation unless the official API contract explicitly guarantees that behavior and the product still verifies the result.

Managed hooks are read-only.

## Privacy threat model

Presentation-sensitive fields may contain:

- filesystem paths;
- command-line arguments;
- matcher expressions;
- plugin/source identifiers;
- MCP names.

Therefore:

- no debug formatting of full snapshots into logs by default;
- no panic/report path that serializes snapshot contents;
- no test fixture containing owner values;
- no diagnostics export of snapshot raw strings;
- no telemetry.

Synthetic fixtures use obvious fake values.

## Test strategy

Required deterministic tests:

- command handler projection;
- MCP handler projection;
- Prompt/Agent projection;
- matcher/source wrapping;
- managed/trust/review states;
- unknown event remains visible;
- failed discovery does not fabricate truth;
- Matched/NoHistory/Ambiguous join outcomes;
- snapshot types do not enter ledger/receipt paths;
- period switching does not trigger unnecessary catalog discovery;
- explicit refresh does.

## Acceptance

```text
LIVE_RUNTIME_PRESENTATION=PASS
UNKNOWN_RUNTIME_EVENT_VISIBLE=true
INSTALLED_UNOBSERVED_VISIBLE=true
AMBIGUOUS_JOIN_NO_FALSE_ATTRIBUTION=true
CURRENT_STATE_FROM_RUNTIME_ONLY=true
HISTORICAL_STATE_FROM_LEDGER_ONLY=true
RAW_RUNTIME_PRESENTATION_PERSISTED=false
NO_THIRD_EVIDENCE_PATH=true
```
