# HookStat v0.3.1 — Runtime-Neutral Native & IPC High-Performance Evidence Runtime

## Status

IN PROGRESS after public v0.3.0 baseline `a33a3be56982c6ca00699019883a047a1aca748b`.

G28, G29, G34, G35, G36, and G37 are accepted on `main`. G38A resets the remaining v0.3.1 release contract so HookStat can complete its own architecture, hardening, and release candidate without writing to another product repository.

v0.3.1 remains Codex-first, but a named external cooperative producer is not a release dependency. Runtime-specific producers consume HookStat's HSIP contract and have independent admission lifecycles.

## Problem statement

HookStat measures Hook reliability and performance. Its own instrumentation cannot be allowed to become a material source of Hook latency, timeout, failure, identity drift, false coverage, or cross-product coupling.

The v0.3 instrumentation-centric path exposed two separate concerns:

1. **HookStat substrate correctness:** canonical evidence, correlation, broker/WAL, diagnostics, performance, recovery, privacy, and truthful authority routing.
2. **Runtime integration admission:** whether a concrete runtime-owned Native source or cooperative IPC producer satisfies HookStat's contract for a specific coverage domain.

v0.3.1 owns the first concern completely. The second is integration-specific and may be completed independently.

## Product objective

HookStat has exactly two evidence transports:

1. **Runtime-native evidence — preferred.** Authoritative lifecycle evidence produced and owned by the runtime without HookStat proxying the observed Hook.
2. **Runtime-neutral local IPC.** A versioned bounded local HSIP protocol and HookStat broker. Concrete cooperative producers are runtime/integration-owned and must be admitted independently.

Production authority is selected per coverage domain:

```text
if Native is admitted for the domain:
    authority = Native
else if a concrete IPC integration is admitted for the domain:
    authority = IPC
else:
    authority = NOT_ADMITTED
```

No third production evidence path is admitted in v0.3.1.

## Single-repository release invariant

```text
HOOKSTAT_RELEASE_CAN_COMPLETE_WITH_HOOKSTAT_REPO_ONLY=true
EXTERNAL_REPOSITORY_WRITE_REQUIRED=false
EXTERNAL_INTEGRATION_REQUIRED_FOR_RELEASE=false
EXTERNAL_INTEGRATION_MERGE_REQUIRED_FOR_RELEASE=false
EXTERNAL_INTEGRATION_PACKAGE_REQUIRED_FOR_RELEASE=false
EXTERNAL_INTEGRATION_PUBLICATION_REQUIRED_FOR_RELEASE=false
```

HookStat development may read public integration evidence for compatibility analysis, but an unattended HookStat release train must not modify another product repository or make its progress conditional on doing so.

Historical cross-repository qualification receipts remain historical truth. This scope reset changes their release-critical status, not their factual content.

## Four non-negotiable correctness principles

### 1. Performance correctness

> A measurement tool must not materially perturb the measurement target.

Performance is a correctness property.

### 2. Evidence correctness

> Missing evidence remains missing or incomplete. It is never converted into success.

### 3. Authority correctness

> Each coverage domain has exactly one production evidence authority.

Native and IPC may shadow each other only for qualification; shadow evidence never contributes to the production denominator.

### 4. Integration-boundary correctness

> Supporting or admitting a runtime integration must not require rewriting HookStat Core, broker, ledger, analytics, workbench, or TUI, and HookStat release work must not require writing the integration's repository.

## Production invariants

```text
HOOKSTAT_VERSION_TARGET=0.3.1

CODEX_FIRST=true
RUNTIME_CORE_NEUTRAL=true

EVIDENCE_PATHS=2
NATIVE_FIRST=true
IPC_ADMITTED_INTEGRATION_ONLY_FALLBACK=true
NO_THIRD_EVIDENCE_PATH=true
NOT_ADMITTED_IS_EVIDENCE_PATH=false

NATIVE_MEANS_RUNTIME_OWNED_EVIDENCE=true
NATIVE_TRANSPORT_OPAQUE_TO_CORE=true

IPC_PROTOCOL_RUNTIME_NEUTRAL=true
IPC_BROKER_RUNTIME_NEUTRAL=true
IPC_INTEGRATION_RUNTIME_SPECIFIC=true

HSIP_PROTOCOL_RELEASE_OWNED_BY_HOOKSTAT=true
COOPERATIVE_IPC_INFRASTRUCTURE=PRODUCTION_READY_TARGET
COOPERATIVE_INTEGRATION_ADMISSION=PER_INTEGRATION
BUNDLED_EXTERNAL_COOPERATIVE_PRODUCER=false

ONE_AUTHORITY_PER_COVERAGE_DOMAIN=true
NO_DOUBLE_COUNTING=true
SHADOW_EVIDENCE_IN_DENOMINATOR=false
MISSING_EVIDENCE_NEVER_BECOMES_SUCCESS=true

TRANSPARENT_SHIM_ADMISSION=QUALIFIED_NOT_ADMITTED_PERFORMANCE
TRANSPARENT_SHIM_PRODUCTION_ACTIVATION=false

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
LEGACY_V03_EVIDENCE_PRESERVED=true
```

## Runtime and transport are orthogonal

`Runtime` identifies the coding-agent runtime. `EvidenceTransport` identifies how HookStat receives evidence.

The core must be able to represent:

```text
Codex + Native
Codex + IPC
OpenCode + Native
OpenCode + IPC
DeepSeekHarness + Native
DeepSeekHarness + IPC
```

Core enums and persistence schemas must not encode runtime-specific transport names such as `CodexAppServer`, `OpenCodePlugin`, or `DeepSeekSessionLog` as evidence transports. Those are adapter/internal diagnostics.

## Native evidence and admission

Native evidence is runtime-owned authoritative lifecycle evidence received without HookStat proxying the observed Hook.

A Native adapter may internally use a supported runtime-owned mechanism such as live protocol notifications, event bus callbacks, durable event logs, runtime databases, or official local protocols.

For v0.3.1 only Codex Native has an implemented qualification path. Controlled App Server L1 qualification is retained. Ordinary-CLI Native L2 is admitted only if upstream exposes a supported attach path.

Native admission states remain:

```text
Unavailable
Discovered
Qualified
Admitted
Degraded
Revoked
```

Ordinary Codex Native L2 being upstream-unavailable is truthful state, not a reason to invent a third evidence path and not by itself a v0.3.1 release blocker.

## Coverage-domain routing

Authority operates over a domain such as:

```text
runtime + event family + source class
```

If no source is admitted, diagnostics/report/TUI must display truthful incomplete or `NOT_ADMITTED` coverage. Such a domain does not enter a healthy production denominator.

## Runtime-neutral canonical evidence

Runtime-specific raw events normalize into `CanonicalEvidence` before becoming `HookInvocation`.

Conceptual fields include:

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

`EvidenceCorrelator` owns lifecycle reconciliation for every runtime and transport:

```text
START + COMPLETE -> complete HookInvocation
START only       -> incomplete
COMPLETE only    -> best-effort terminal evidence
duplicate        -> idempotent
out-of-order     -> deterministic reconciliation
```

Adapters must not reimplement these semantics independently.

## Runtime-specific identity resolution

Runtime identity remains adapter-owned. Runtime-specific opaque references must resolve to HookStat stable handler/revision/display metadata before ledger attribution.

Core assumptions must not depend on Codex `hooks.json` indexes, App Server identifiers, or future runtime-specific fields.

## IPC architecture

```text
runtime-specific producer
        ↓
HSIP v1 local transport
        ↓
generic HookStat broker
        ↓
compact append WAL
        ↓
CanonicalEvidence
        ↓
EvidenceCorrelator
```

Platform transport remains:

```text
Windows -> Named Pipe
Unix    -> Unix Domain Socket
```

The broker is per-user, local-only, on-demand, idle-expiring, bounded, and non-networked. It receives, validates, queues, appends WAL, acknowledges, performs group durability/recovery, and expires when idle. It does not own runtime-specific identity inference, analytics, trust decisions, or TUI work.

## IPC producer modes

### Cooperative producer

A runtime/integration under its own control emits START/COMPLETE directly through HSIP. No HookStat wrapper process is introduced.

HookStat v0.3.1 provides the protocol, broker, conformance kit, reference producer, diagnostics, and admission contract. It does not bundle or require a named external cooperative producer.

### Transparent shim

The dedicated minimal `hookstat-hook` implementation remains correctness/security-qualified but performance-not-admitted. Its historical 20/25 and 25/30 failures are retained. It cannot become production authority in v0.3.1.

G36T owns any v0.3.2-or-later rearchitecture.

## HSIP v1 conformance and integration admission

The normative conformance/admission contract is defined by `docs/architecture/HSIP-V1-CONFORMANCE-AND-ADMISSION.md`.

HookStat owns an in-repository reference producer and conformance harness. The reference producer exists only to prove protocol/broker behavior and is not a production runtime integration.

A named external producer becomes eligible for production authority only after independent evidence proves:

```text
PROTOCOL=PASS
CORRELATION=PASS
FAIL_OPEN=PASS
NO_REPLAY_AFTER_UNCERTAIN_WRITE=PASS
PRIVACY=PASS
SECURITY=PASS
PACKAGE_PROVENANCE=PASS
PERFORMANCE=PASS
INDEPENDENT_REVIEW=PASS
```

An external producer's failed admission does not block HookStat v0.3.1.

## Performance contract

G28 budgets remain frozen.

### HookStat release substrate gate

The in-repository reference producer + HookStat broker must prove:

```text
REFERENCE_HSIP_P95_MS<=1
REFERENCE_HSIP_P99_MS<=2
REFERENCE_HSIP_OBSERVATION_GAPS=0
HOOKSTAT_INDUCED_TIMEOUTS_FOR_HEALTHY_HOOK=0
HOOKSTAT_INDUCED_FAILURES_FOR_HEALTHY_HOOK=0
```

If HookStat's own substrate cannot meet this gate, G38B/G38D are blocked.

### External integration admission gate

Each external cooperative producer must independently satisfy:

```text
INTEGRATION_HSIP_P95_MS<=1
INTEGRATION_HSIP_P99_MS<=2
INTEGRATION_OBSERVATION_GAPS=0
```

A failure blocks that integration's admission only.

### Transparent historical result

```text
G28_REFERENCE_TRANSPARENT_WARM_P95_P99_MS=20/25
ONE_TIME_V031_TRANSPARENT_WARM_P95_P99_MS=25/30
TRANSPARENT_SHIM_20_25=FAIL
TRANSPARENT_SHIM_25_30=FAIL
TRANSPARENT_SHIM_PRODUCTION_ADMISSION=false
FURTHER_BUDGET_RELAXATION=false
```

## Persistence policy

v0.3.1 uses compact append WAL and group durability for the production IPC substrate. Per-record `fsync`/`sync_data` is prohibited on the synchronous producer path.

Legacy v0.3 JSON receipts and ledger history remain historical truth and read-only compatible. They may not be silently rewritten or deleted.

## Diagnostics / self-observability

HookStat diagnostics must expose, without raw private content:

```text
runtime
Native capability/admission
IPC protocol/broker state
named IPC integration admission where known
authoritative source per domain
NOT_ADMITTED domains
queue lag
dropped/malformed frames
WAL flush lag
reference/recent IPC latency percentiles
transparent shim status: qualified_not_admitted_performance
```

Diagnostics control frames are not lifecycle evidence and never enter WAL, ledger, replay, or denominators.

## Revised Goal dependency DAG

```text
PUBLIC v0.3.0
  ↓
HS-G28 — Hot Path Performance Baseline                         [accepted]
  ↓
HS-G29 — Runtime-Neutral Evidence Core                         [accepted]
  ↓
HS-G34 — Native Evidence Framework / Codex Native L1           [accepted]
  ↓
HS-G35 — Runtime-Neutral IPC Broker / WAL                      [accepted]
  ↓
HS-G36 — Ultra-Light IPC Clients / Transparent Shim            [accepted]
  ↓
HS-G37 — Authority Routing / Native L2 / Migration             [accepted]
  ↓
HS-G38A — Single-Repo Scope & Admission Contract Reset
  ├───────────────┐
  ▼               ▼
HS-G38B         HS-G38C
Conformance     Windows Hardening
  └───────┬───────┘
          ▼
       HS-G38D
 Acceptance / Closeout
          ↓
       HS-G38R
 Hardening & Release
          ↓
    PUBLIC v0.3.1
```

External producer tracks branch from the HSIP contract and never sit on this critical path.

## G38 acceptance semantics

G38 is decomposed into G38A/B/C/D.

A successful G38 proves HookStat's own protocol, reference producer, broker/WAL, recovery, diagnostics, privacy/security, Windows concurrency/resource behavior, CI, and review.

It does **not** require a real external producer to cover every Codex event family. Event-family coverage is a requirement of a named admitted runtime integration. If no such integration is admitted, production runtime domains remain explicit `NOT_ADMITTED`.

Normal `codex` smoke remains required to prove HookStat preserves ordinary launch semantics and does not require a launcher, trust bypass, or unwanted persistent mutation.

## Release semantics

G38R freezes architecture and may prepare the exact v0.3.1 release candidate.

Fresh install must prove:

```text
report/doctor/TUI work without prior state
HSIP reference/conformance qualification works
Native state is truthful
IPC integration admission state is truthful
uncovered domains are NOT_ADMITTED
normal codex launch remains codex
legacy upgrade/history is preserved
```

If no external producer is admitted, documentation and release notes must say so explicitly. Do not imply live Hook coverage that the release cannot observe.

## Explicit non-goals

v0.3.1 MUST NOT require or silently begin:

- modification of TabBeacon or any other external producer repository;
- a bundled external cooperative producer;
- OpenCode production adapter;
- DeepSeek Harness production adapter;
- Claude Code production adapter;
- Agy production adapter;
- transparent-shim rearchitecture;
- OTel/session-history/polling as a third reliability path;
- Web/remote dashboard;
- cloud/distributed aggregation;
- remote/network broker;
- AI root-cause diagnosis;
- HookStat-as-Codex launcher;
- broad TUI redesign.

## Unattended-train authority

An unattended implementer may autonomously implement, test, benchmark, document, review-repair, reconcile PRs, and merge accepted HookStat G38A/B/C/D and G38R work.

It may not:

```text
WRITE_EXTERNAL_REPOSITORY=true
ADD_THIRD_EVIDENCE_PATH=true
WEAKEN_PRIVACY=true
WEAKEN_COVERAGE_SEMANTICS=true
WEAKEN_FROZEN_PERFORMANCE_BUDGET=true
DELETE_OR_REWRITE_LEGACY_EVIDENCE=true
REQUIRE_GLOBAL_OR_NETWORK_DAEMON=true
PUBLISH_CRATES_IO=true
CREATE_PUBLIC_V031_TAG=true
CREATE_PUBLIC_GITHUB_RELEASE=true
```

## Mandatory stop gates

1. Stop G38B/G38D if the HookStat reference HSIP substrate cannot meet the frozen 1/2 ms Windows budget.
2. Native L2 upstream unavailability is not a release blocker; record it and retain `NOT_ADMITTED` where needed.
3. Any false healthy coverage, persistence corruption, privacy/security defect, or reproducible HookStat-induced timeout/failure blocks G38R.
4. Public crates.io/tag/GitHub Release publication remains an explicit Owner gate after G38R acceptance.
