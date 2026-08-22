# HS-G29 — Runtime-Neutral Evidence Core

## Status

PLANNED after accepted G28 and the frozen v0.3.1 performance budget.

## Objective

Introduce the canonical, runtime-neutral evidence semantics that allow HookStat to consume either runtime-native or IPC evidence without rewriting ledger, analytics, workbench, or TUI logic for each coding-agent runtime.

This is the most important architecture Goal in v0.3.1.

## Core design rule

`Runtime` and `EvidenceTransport` are orthogonal.

Runtime identifies the coding-agent runtime. Evidence transport identifies how evidence reached HookStat.

Production transport values are exactly:

```text
Native
Ipc
```

Do not encode transport/runtime mixtures such as `CodexAppServer`, `OpenCodePlugin`, or `DeepSeekSessionLog` in the core evidence transport model.

## CanonicalEvidence

Add a bounded canonical lifecycle record before `HookInvocation`.

Conceptual contract:

```text
schema_version
runtime
runtime_instance
invocation_key
runtime_handler_ref
event
lifecycle
occurred_at_unix_ms
terminal_status?
duration_ms?
source_scope
revision_ref?
evidence_transport
source_coverage
invocation_coverage
```

Exact Rust representation may differ, but it must preserve these semantics and remain independent of Codex-specific wire types.

## Lifecycle model

At minimum:

```text
Started
Completed
```

A runtime-native adapter or IPC producer emits canonical lifecycle evidence. The correlator, not each adapter, constructs final invocation state.

## EvidenceCorrelator

Centralize lifecycle reconciliation:

```text
START + COMPLETE -> complete HookInvocation
START only       -> Incomplete
COMPLETE only    -> BestEffort terminal evidence
duplicate        -> idempotent
out-of-order     -> deterministic reconciliation
```

Missing terminal evidence must never become success.

Correlation keys must be bounded, deterministic, privacy-safe, and runtime-neutral at the core boundary.

## Coverage model

Separate source-surface qualification from individual-invocation completeness.

Conceptually distinguish:

### SourceCoverage

Examples:

```text
Complete
Partial
EventLimited
IdentityLimited
LiveOnly
Durable
Unknown
```

The exact enum may be refined if mutually exclusive enum values prove inadequate; the essential requirement is that HookStat can truthfully say why a Native/IPC source is limited without collapsing every limitation into one generic `Partial`.

### InvocationCoverage

At minimum preserve:

```text
Complete
Incomplete
BestEffort
Unknown
```

Do not weaken existing v0.3 coverage semantics during migration.

## Evidence authority

Introduce a coverage-domain authority model.

Minimum conceptual domain:

```text
runtime + event family + source class
```

Each production domain has exactly one authority:

```text
Native
or
Ipc
```

Shadow evidence is allowed only for qualification and must carry a state that makes it impossible to enter the production failure-rate denominator.

Do not depend on fuzzy post-ingest deduplication to prevent Native/IPC double counting.

## Native admission state

Add a runtime-neutral admission state suitable for future runtime adapters:

```text
Unavailable
Discovered
Qualified
Admitted
Degraded
Revoked
```

Only `Admitted` Native evidence may become production authority for a domain.

## RuntimeHandlerRef and identity boundary

The canonical layer accepts an opaque runtime handler reference. Runtime-specific identity resolution remains outside the evidence core and resolves to HookStat stable handler identity/revision before ledger attribution.

The core MUST NOT depend on:

- Codex hooks.json paths;
- Codex group/handler array indexes;
- Codex App Server IDs;
- OpenCode plugin IDs;
- DeepSeek handlerId semantics;
- any future runtime-specific definition field.

## Runtime-neutral proof fixtures

Do not implement non-Codex production adapters. Instead create synthetic evidence fixtures modeling at least:

### Synthetic Runtime A — live lifecycle

```text
start notification
completed notification
```

### Synthetic Runtime B — durable lifecycle

```text
persisted invoked record
persisted result record
replay cursor
```

### Synthetic Runtime C — partial Native + IPC fallback

At least one domain is Native authority and another domain is IPC authority within the same synthetic runtime.

These fixtures prove the abstraction only and must never be represented as production runtime support.

## Existing core compatibility

Prefer additive/refactoring changes that keep:

```text
HookInvocation
ledger semantics
failure-rate denominator
analytics
Changes workbench
Hook Catalog
TUI
```

stable after canonical correlation.

If a ledger migration becomes demonstrably necessary, stop and document the exact reason before implementing it. The default acceptance target is no destructive ledger migration.

## Privacy

Canonical evidence and its tests must not require or persist:

```text
prompt
assistant content
tool input/output
stdout
stderr
raw command
credential/secret material
```

## Required tests

At minimum:

- ordered START/COMPLETE;
- completion-before-start;
- duplicate START;
- duplicate COMPLETE;
- START without COMPLETE;
- COMPLETE without START;
- conflicting duplicate terminal evidence handled conservatively;
- Native shadow + IPC authority does not double-count;
- Native authority + IPC shadow does not double-count;
- mixed domains in one runtime route independently;
- synthetic durable replay is idempotent;
- synthetic live delivery is idempotent;
- handler reference remains opaque to core;
- missing evidence never becomes completed/success.

## Risk vector

```text
CODE_CHANGED=true
ARCHITECTURE_CHANGED=true
PERSISTENCE_CHANGED=prefer_no
CODEX_INTEGRATION_CHANGED=false
FAILURE_SEMANTICS_CHANGED=false
DENOMINATOR_SEMANTICS_CHANGED=false
SECURITY_OR_PRIVACY_CHANGED=true
RELEASE_BOUNDARY=false
```

An independent evidence-semantics review is required. Green tests alone are not sufficient acceptance for this Goal.

## Acceptance

```text
CANONICAL_EVIDENCE=PASS
CORRELATOR_RUNTIME_NEUTRAL=true
AUTHORITY_MODEL=PASS
ONE_AUTHORITY_PER_DOMAIN=true
DOUBLE_COUNTING_TESTS=PASS
PARTIAL_NATIVE_PLUS_IPC=PASS
OUT_OF_ORDER_EVIDENCE=PASS
DUPLICATE_EVIDENCE=PASS
INCOMPLETE_TRUTHFUL=true

CODEX_TYPES_IN_EVIDENCE_CORE=0
OPENCODE_TYPES_IN_EVIDENCE_CORE=0
DEEPSEEK_TYPES_IN_EVIDENCE_CORE=0

FAILURE_RATE_DENOMINATOR_CHANGED=false
MISSING_EVIDENCE_AS_SUCCESS=false
LEDGER_DESTRUCTIVE_MIGRATION=false
ANALYTICS_REWRITE_REQUIRED=false
TUI_REWRITE_REQUIRED=false

SYNTHETIC_RUNTIME_A=PASS
SYNTHETIC_RUNTIME_B=PASS
SYNTHETIC_RUNTIME_C=PASS
EVIDENCE_SEMANTICS_REVIEW=PASS
CODE_CI=PASS
```

## Estimated effort

**8–12 effective engineering hours.**

## Next

`HS-G34 — Native Evidence Framework + Codex Native L1 Qualification`.
