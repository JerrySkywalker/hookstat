# HS-G34 — Native Evidence Framework + Codex Native L1 Qualification

## Status

PLANNED after accepted G29.

## Objective

Implement the runtime-neutral Native evidence contracts and prove them with Codex Hook lifecycle evidence in a controlled App Server environment, without requiring HookStat to become the Codex launcher.

## Native definition

Native evidence is lifecycle evidence produced and owned by the coding-agent runtime without HookStat proxying the observed Hook.

The framework must permit future Native implementations backed by live protocol notifications, event buses, plugin callbacks, durable event logs, or runtime-owned local databases/protocols.

## Required narrow contracts

Implement narrow composition rather than one large runtime adapter:

```text
NativeCapabilityProbe
NativeEvidenceReader
NativeNormalizer
RuntimeIdentityResolver
```

A small runtime integration object may compose them.

### NativeCapabilityProbe

Report capability facts rather than a single boolean. At minimum assess:

```text
invocation_start
terminal_result
stable_handler_attribution
duration
source_scope
revision_attribution
ordering/correlation
replay_or_delivery_characteristics
event_surface_completeness
privacy_boundary
version_compatibility
```

### NativeEvidenceReader

The reader must support adapter-owned opaque cursor/session state. Core code must not assume every Native source is a live subscription.

### NativeNormalizer

Normalize runtime-owned records to `CanonicalEvidence`. No Codex wire type may cross into the evidence core.

## Codex implementation

Implement:

```text
CodexNativeCapabilityProbe
CodexNativeReader
CodexNativeNormalizer
CodexNativeIdentityResolver
```

Qualify current Codex HookStarted/HookCompleted lifecycle evidence using a controlled App Server session. Pin the source/version evidence used for qualification and record future compatibility assumptions explicitly.

## Codex L1 acceptance path

Required controlled proof:

```text
controlled Codex App Server
  ↓
real HookStarted / HookCompleted
  ↓
Codex Native reader
  ↓
CanonicalEvidence
  ↓
EvidenceCorrelator
  ↓
HookInvocation
```

Use real protocol evidence, not only hand-authored JSON fixtures.

## Identity qualification

Do not assume an upstream handler/run ID is automatically a long-term HookStat stable handler key.

Prove or derive:

```text
stable handler attribution
revision attribution
event attribution
source-scope attribution
restart/config-change behavior
```

Raw paths/commands remain ephemeral inputs and must not enter the ledger.

If Codex Native cannot prove a required identity property, report the relevant source/domain coverage limitation and do not upgrade Native to `Admitted` for that domain.

## Ordinary Codex attachment boundary

G34 does **not** require an external HookStat process to attach to a user-launched ordinary `codex` CLI session.

That is G37 Native L2 qualification.

Do not introduce:

```text
hookstat codex launcher
PATH shadow
PTY host
wrapper around codex
```

to manufacture Native availability.

## Required tests

- capability report is deterministic and version-aware;
- controlled HookStarted maps to canonical Started evidence;
- controlled HookCompleted maps to canonical Completed evidence;
- start/completion correlation reaches a correct HookInvocation;
- terminal statuses remain truthful;
- duration semantics are preserved;
- multiple handlers on one event remain distinguishable where upstream evidence permits;
- missing identity proof lowers admission/coverage rather than guessing;
- raw private payload fields are rejected/ignored by persistence boundaries;
- reader cursor/state remains adapter-internal.

## Risk vector

```text
CODE_CHANGED=true
ARCHITECTURE_CHANGED=true
CODEX_INTEGRATION_CHANGED=true
PRODUCTION_AUTHORITY_CHANGED=false
USER_LIVE_CONFIG_MUTATION=false
SECURITY_OR_PRIVACY_CHANGED=true
RELEASE_BOUNDARY=false
```

## Acceptance

```text
NATIVE_FRAMEWORK_RUNTIME_NEUTRAL=true
NATIVE_CAPABILITY_MATRIX=PASS
NATIVE_READER_TRANSPORT_AGNOSTIC=true

CODEX_NATIVE_PROBE=PASS
CODEX_NATIVE_NORMALIZER=PASS
CODEX_NATIVE_IDENTITY=PASS
CODEX_CONTROLLED_HOOK_STARTED=PASS
CODEX_CONTROLLED_HOOK_COMPLETED=PASS
CODEX_NATIVE_TO_HOOK_INVOCATION=PASS

ORDINARY_CODEX_ATTACH_NOT_REQUIRED_AT_G34=true
HOOKSTAT_AS_CODEX_LAUNCHER=false
RAW_NATIVE_PRIVATE_CONTENT_PERSISTED=false
CODE_CI=PASS
```

## Estimated effort

**8–12 effective engineering hours.**

## Next

`HS-G35 — Runtime-Neutral IPC Broker / WAL`.
