# HookStat v0.3.1 — Runtime-Neutral Native & IPC High-Performance Evidence Runtime

## Status

PLANNED after public v0.3.0 baseline `a33a3be56982c6ca00699019883a047a1aca748b`.

v0.3.1 remains **Codex-only in production**. Future OpenCode, DeepSeek Harness, Claude Code, Agy, or other runtime integrations are not release dependencies. The purpose of this release is to establish the runtime-neutral evidence architecture those future integrations can use without rewriting HookStat Core.

## Problem statement

HookStat measures Hook reliability and performance. Its own instrumentation therefore cannot be allowed to become a material source of Hook latency, timeout, failure, or identity drift.

Owner Windows dogfood of v0.3.0 exposed the current architectural limit:

- the v0.3 proxy path launches the full HookStat executable for individual handlers;
- it loads a private manifest, writes start/completion receipt files, appends a journal, and performs synchronous durability work;
- it invokes the original Windows command through an additional shell layer;
- high-frequency PreToolUse/PostToolUse paths can therefore amplify instrumentation overhead thousands of times;
- observed one-second Hook declarations timed out when enabled under the current instrumentation path, while old evidence contained a large start-only/incomplete population.

v0.3.1 treats this as an architecture and performance-correctness problem, not as a reason to blindly increase Hook timeout values.

## Product objective

Replace the instrumentation-centric evidence model with exactly two production evidence paths:

1. **Runtime-native evidence — preferred.** Consume authoritative Hook lifecycle evidence produced and owned by the coding-agent runtime. HookStat does not proxy the observed Hook and adds zero synchronous Hook latency.
2. **Runtime-neutral local IPC — fallback.** For a coverage domain without admitted native evidence, obtain start/result evidence using a minimal local IPC producer and broker. Cooperative producers are preferred when the observed Hook can emit HookStat evidence itself; a tiny transparent shim remains the universal third-party fallback within the same IPC path.

No third production evidence path is admitted in v0.3.1.

## Four non-negotiable correctness principles

### 1. Performance correctness

> A measurement tool must not materially perturb the measurement target.

Performance is a release property, not an optional optimization.

### 2. Evidence correctness

> Missing evidence remains missing or incomplete. It is never converted into success.

### 3. Authority correctness

> Each coverage domain has exactly one production evidence authority. Native and IPC may shadow each other for qualification, but shadow evidence never contributes to a production denominator.

### 4. Future-runtime correctness

> Supporting a future runtime must require adding runtime integration code, not rewriting HookStat Core, broker, ledger, analytics, workbench, or TUI.

## Production invariants

```text
HOOKSTAT_VERSION_TARGET=0.3.1

PRODUCTION_RUNTIME=Codex
CODEX_FIRST=true
CODEX_ONLY_RELEASE_REQUIREMENT=true
NON_CODEX_RUNTIME_REQUIRED_FOR_RELEASE=false

RUNTIME_CORE_NEUTRAL=true

EVIDENCE_PATHS=2
NATIVE_FIRST=true
IPC_ONLY_FALLBACK=true
NO_THIRD_EVIDENCE_PATH=true

NATIVE_MEANS_RUNTIME_OWNED_EVIDENCE=true
NATIVE_TRANSPORT_OPAQUE_TO_CORE=true

IPC_PROTOCOL_RUNTIME_NEUTRAL=true
IPC_BROKER_RUNTIME_NEUTRAL=true
IPC_INTEGRATION_RUNTIME_SPECIFIC=true

ONE_AUTHORITY_PER_COVERAGE_DOMAIN=true
NO_DOUBLE_COUNTING=true
SHADOW_EVIDENCE_IN_DENOMINATOR=false

CANONICAL_EVIDENCE_BEFORE_HOOK_INVOCATION=true
CORRELATION_RUNTIME_NEUTRAL=true
IDENTITY_RESOLUTION_RUNTIME_SPECIFIC=true

NORMAL_CODEX_LAUNCH=codex
HOOKSTAT_AS_CODEX_LAUNCHER=false

GLOBAL_MANDATORY_DAEMON=false
NETWORK_LISTENER=false
REMOTE_TELEMETRY=false
SELF_UPDATE=false

RAW_PRIVATE_CONTENT_PERSISTED=false
COVERAGE_TRUTHFUL=true
FAILURE_RATE_WITH_SAMPLE_COUNT=true
```

## Runtime and transport are orthogonal

`Runtime` identifies the coding-agent runtime. `EvidenceTransport` identifies how HookStat receives evidence.

The architecture must be able to represent:

```text
Codex + Native
Codex + IPC

OpenCode + Native
OpenCode + IPC

DeepSeekHarness + Native
DeepSeekHarness + IPC
```

Core enums and persistence schemas must not encode runtime-specific transport names such as `CodexAppServer`, `OpenCodePlugin`, or `DeepSeekSessionLog` as evidence transports. Those remain adapter-internal diagnostics.

## What Native means

Native evidence is authoritative lifecycle evidence produced and owned by the runtime without HookStat proxying the observed Hook.

A Native adapter may internally use any runtime-owned mechanism, including:

- live protocol notifications;
- an event bus;
- a plugin callback;
- a durable event log;
- a runtime database or official local protocol.

Therefore the Native abstraction must not assume that every runtime supports a live `subscribe()` API.

For v0.3.1 only Codex Native is implemented. The current target is the runtime-owned HookStarted/HookCompleted lifecycle evidence exposed by the Codex App Server protocol. Controlled App Server qualification is mandatory; production ordinary-CLI attachment is admitted only if upstream provides a supported attach path.

## Native admission

A runtime-native source becomes production authority only after evidence qualification proves the required semantics for the relevant coverage domain.

Native admission states:

```text
Unavailable
Discovered
Qualified
Admitted
Degraded
Revoked
```

Capability qualification must explicitly cover at least:

```text
invocation start
terminal result
handler attribution
duration semantics
source scope
revision attribution
ordering/correlation
replay or delivery characteristics
event-surface completeness
privacy boundary
version compatibility
```

`Qualified` is not automatically `Admitted`.

## Coverage-domain routing

Native availability may differ by event or source class. Authority selection therefore operates on a coverage domain rather than on a runtime-wide boolean.

Minimum conceptual domain:

```text
runtime + event family + source class
```

Example future state:

```text
DeepSeekHarness / PreToolUse / bridge-owned -> Native
DeepSeekHarness / SessionStart / bridge-owned -> IPC
```

The production router chooses Native only where that domain is admitted; otherwise it chooses IPC.

## Runtime-neutral canonical evidence

Runtime-specific raw events normalize first into `CanonicalEvidence`, before they become `HookInvocation`.

Conceptual fields:

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

The canonical layer must not persist raw prompt, assistant, tool, stdout, stderr, secret, or command content.

## Runtime-neutral correlation

`EvidenceCorrelator` owns lifecycle pairing and reconciliation for every runtime and transport:

```text
START + COMPLETE -> complete HookInvocation
START only       -> incomplete
COMPLETE only    -> best-effort terminal evidence
duplicate        -> idempotent
out-of-order     -> deterministic reconciliation
```

These semantics must not be reimplemented independently in each runtime adapter.

## Runtime-specific identity resolution

Runtime identity semantics differ and remain adapter-owned. A runtime adapter may use its own opaque `RuntimeHandlerRef` and runtime-specific definition evidence, but it must resolve to HookStat's stable handler key/revision/display metadata before ledger attribution.

No core assumption may depend on Codex `hooks.json` path, group index, handler index, App Server identifiers, or any future OpenCode/DeepSeek-specific field.

## Native adapter composition

Do not create one giant `RuntimeAdapter` trait. Prefer narrow contracts:

```text
NativeCapabilityProbe
NativeEvidenceReader
NativeNormalizer
RuntimeIdentityResolver
IpcIntegrationAdapter
```

A small `RuntimeIntegration` object may compose these components and report the runtime it serves.

`NativeEvidenceReader` must allow adapter-owned opaque cursor/state so a future runtime can implement live notifications, event-sequence replay, or durable-log offsets without changing core interfaces.

## IPC architecture

IPC is the only fallback evidence path and is runtime-neutral after integration.

```text
runtime-specific integration
        ↓
generic local IPC producer
        ↓
generic HookStat broker
        ↓
compact append WAL
        ↓
CanonicalEvidence
        ↓
EvidenceCorrelator
```

### Platform transport

```text
Windows -> Named Pipe
Unix    -> Unix Domain Socket
```

Both implement one local transport abstraction.

### IPC producer modes

Both are the same evidence path:

1. **Cooperative IPC:** a Hook under our control emits START/COMPLETE directly to HookStat IPC. No HookStat wrapper process is introduced.
2. **Transparent shim:** a dedicated minimal HookStat shim emits START, executes the original third-party Hook, emits COMPLETE, and forwards the original terminal semantics.

Cooperative IPC is preferred where practical. TabBeacon is the first intended real cooperative dogfood consumer.

### Dedicated minimal shim

The v0.3 full `hookstat.exe codex proxy` must leave the production hot path.

A dedicated minimal executable conceptually named `hookstat-hook` owns only:

```text
IPC protocol
handler capsule reader
clock
process spawn
Windows process containment
exit/timeout propagation
```

It must not link product-level TUI, analytics, workbench, report, localization, or SQLite functionality.

### Handler capsule

Instrumentation precompiles per-handler bounded execution metadata rather than loading the full private manifest for each invocation.

Conceptual capsule fields:

```text
schema
stable key
revision
bounded execution plan
original timeout semantics
definition fingerprint
```

A provably simple executable/argv command may use a direct process fast path. Shell-dependent commands fall back to the platform shell; HookStat must not invent an incomplete Windows shell parser.

## IPC broker

The broker is per-user, local-only, on-demand, and idle-expiring. It is not a mandatory machine-global daemon.

Broker responsibilities are deliberately narrow:

```text
receive
validate
enqueue
append WAL
acknowledge
batch/group durability
recover
idle-expire
```

The broker does not perform runtime-specific identity inference, analytics, trust decisions, TUI work, or source-authority decisions.

The broker must use bounded queues and bounded frame sizes. The network stack is not part of this subsystem.

## Persistence policy

The v0.3 hot path creates per-invocation JSON files and synchronously appends the receipt journal. v0.3.1 replaces this production hot path with a compact append WAL and group durability.

Per-record `fsync`/`sync_data` is prohibited on the synchronous Hook path.

The exact group-durability interval/size is calibrated by G28/G35 performance and crash-recovery evidence. Losing a very small final power-loss window is preferable to making every high-frequency observed Hook pay synchronous disk-flush latency, provided crash/restart semantics remain explicit and truthful.

## Timeout correctness

HookStat instrumentation overhead must not silently consume the original Hook's business timeout budget.

The implementation distinguishes:

```text
OriginalHandlerBudget
InstrumentationEnvelope
```

The original handler receives no more execution time than its original semantics allow. A bounded outer envelope may exist solely to permit HookStat startup/finalization overhead; its size must be derived from the frozen G28 performance budget, not guessed.

Blindly increasing all Hook timeouts is not an accepted fix.

## Performance contract

G28 records the real Owner Windows baseline and is the only Goal permitted to calibrate provisional numeric targets. After G28, the budget is frozen and release-governing.

Provisional targets:

```text
NATIVE_ADDED_SYNCHRONOUS_LATENCY_MS=0
COOPERATIVE_IPC_P95_MS<=1
COOPERATIVE_IPC_P99_MS<=2
TRANSPARENT_SHIM_WARM_P95_MS<=15
TRANSPARENT_SHIM_WARM_P99_MS<=25
TRANSPARENT_SHIM_COLD_P95_MS<=50
HOOKSTAT_INDUCED_TIMEOUTS_FOR_HEALTHY_HOOK=0
```

## Legacy compatibility

Existing v0.3 evidence is historical truth and must not be rewritten or deleted merely because v0.3.1 changes transport.

Required model:

```text
legacy v1 JSON receipts -> read-only compatibility
legacy ledger history   -> preserved
new Native evidence     -> new evidence generation/source
new IPC evidence        -> new evidence generation/source
```

Historical incomplete rows remain incomplete unless later real evidence can truthfully upgrade the same invocation under existing idempotent semantics.

## Diagnostics / self-observability

HookStat must make its own evidence path and overhead inspectable without introducing meaningful hot-path overhead.

Target diagnostics include:

```text
runtime
authoritative evidence source per domain
Native admission state
IPC mode: cooperative/shim
broker health
queue lag
dropped frames
WAL flush lag
IPC p50/p95/p99
shim incremental overhead p50/p95/p99
```

Hosted CI checks structural regressions; real Windows performance evidence remains the release gate.

## Goal dependency DAG

```text
PUBLIC v0.3.0
  ↓
HS-G28 — Hot Path Performance Baseline
  ↓
HS-G29 — Runtime-Neutral Evidence Core
  ↓
HS-G34 — Native Evidence Framework + Codex Native L1 Qualification
  ↓
HS-G35 — Runtime-Neutral IPC Broker / WAL
  ↓
HS-G36 — Ultra-Light IPC Clients / Transparent Shim
  ↓
HS-G37 — Codex Evidence Routing / Native L2 / Migration
  ↓
HS-G38 — Performance & Windows Dogfood Hardening
  ↓
HS-G38R — v0.3.1 Hardening & Release
```

`HS-G30X`–`HS-G33X` remain future-runtime tracks and are not changed by this version.

## Explicit non-goals

v0.3.1 MUST NOT ship or require:

- OpenCode production adapter;
- DeepSeek Harness production adapter;
- Claude Code production adapter;
- Agy production adapter;
- OTel as Hook reliability evidence;
- rollout/session-history inference as a third evidence path;
- polling as a third evidence path;
- Web/remote dashboard;
- cloud/distributed aggregation;
- remote/network broker;
- AI root-cause diagnosis;
- general machine daemon framework;
- HookStat-as-Codex launcher;
- broad TUI redesign.

Synthetic future-runtime fixtures prove abstraction quality only and must never be described as production support.

## Unattended-train authority

An implementer may autonomously perform routine implementation, tests, bounded refactors inside this accepted design, fixtures, benchmarks, documentation, CI repair, PR creation/review repair, merge after acceptance, and continuation into the next adjacent Goal after predecessor acceptance.

The implementer may not autonomously:

- add a third evidence path;
- weaken privacy or coverage semantics;
- change failure-rate denominator semantics;
- delete or rewrite legacy evidence;
- promote a non-Codex runtime;
- convert HookStat into a Codex launcher;
- require a global/network daemon;
- publish crates.io, create the public v0.3.1 tag, or create the public GitHub Release.

## Bounded stacked-development exception

A predecessor Goal with complete/stable implementation may permit implementation of **one and only one** successor Goal before predecessor acceptance/merge when its remaining blockers are external acceptance conditions rather than unresolved implementation defects.

Required predecessor state:

```text
PREDECESSOR_IMPLEMENTATION_COMPLETE=true
PREDECESSOR_KNOWN_CODE_BLOCKERS=0
PREDECESSOR_CI=PASS
PREDECESSOR_ARCHITECTURE_STABLE=true
MAX_STACK_DEPTH=1
```

Admitted blocker classes are limited to Owner/environment qualification, real-hardware/host qualification, external-service availability, independent review, or another Owner-only evidence/approval gate.

This exception MUST NOT be used when the predecessor has an unresolved correctness defect, architecture uncertainty, persistence/data-corruption risk, privacy/security defect, unstable protocol/API/format contract, or unresolved evidence/failure semantics.

The successor must branch from the exact predecessor implementation head and target the predecessor branch as a stacked PR. It may be implemented, tested, reviewed, and kept CI-green, but it MUST NOT merge to `main` until the predecessor is accepted and merged. After predecessor merge, the successor must be rebased/retargeted to accepted `main` and exact-head CI must be repeated before successor merge.

Acceptance work on the predecessor may proceed in parallel. If the predecessor remains unmerged after the one successor Goal is implemented, downstream development stops; stacking may not continue to a second successor.

This exception changes scheduling only. It does not weaken Goal acceptance criteria, performance budgets, review requirements, evidence truthfulness, or release gates.

For the current G35/G36 sequence, G36 stacked implementation is permitted only while G35 has no known implementation/correctness blocker and its remaining blockers are external performance qualification and/or independent review. G37 remains blocked until both G35 and G36 are accepted into `main`.

## Mandatory version stop gates

1. **After G28:** stop if the selected IPC transport cannot satisfy a credible low-latency Windows budget.
2. **Native L2:** ordinary-Codex external attach being upstream-unavailable is not a version stop. Record it truthfully and use IPC authority.
3. **Before G38R:** any reproducible HookStat-induced timeout/failure in a previously healthy Hook blocks release.

## Release boundary

`HS-G38R` freezes new architecture and feature work. Public publication remains an explicit Owner authorization after exact-head CI, upgrade/fresh-install proof, real Windows Codex dogfood, and performance acceptance.
