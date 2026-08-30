# HookStat v0.3.1 execution roadmap

## Status

CLOSED / PUBLICLY RELEASED. This document is the immutable historical
execution record for v0.3.1; the current product authority is
[`ROADMAP_V040.md`](ROADMAP_V040.md).

```text
PUBLIC_VERSION=0.3.1
PUBLIC_MAIN=651620cbc9f204f312fc31efee424c747895927a
PUBLIC_TAG=v0.3.1
PUBLIC_RELEASE=true
```

The v0.3.1 train began from public v0.3.0 baseline
`a33a3be56982c6ca00699019883a047a1aca748b`. The original checkpoints and
receipts below remain historical evidence and are not rewritten by this
closeout.

Accepted `main` has completed G28, G29, G34, G35, G36, and G37. The current accepted main fingerprint at this scope reset is:

```text
G37_ACCEPTED_MAIN=ac9683a151741a28341b357dc11ae6fd3b701dfd
```

The previous G38 plan coupled HookStat release acceptance to a concrete external cooperative producer. That coupling is removed by G38A. HookStat v0.3.1 must be completable from the HookStat repository alone.

Historical external-integration receipts remain valid historical evidence and must not be deleted or rewritten, but they are no longer mandatory predecessors for HookStat v0.3.1 release acceptance.

## Product direction

**v0.3.1 — Runtime-Neutral Native & IPC High-Performance Evidence Runtime** keeps Codex as the first runtime while establishing a runtime-neutral evidence platform.

HookStat has exactly two evidence transports:

1. **Runtime-native evidence, preferred.** Runtime-owned authoritative lifecycle evidence when a concrete Native integration is admitted for the coverage domain.
2. **Runtime-neutral local IPC.** A bounded local HSIP v1 protocol and broker. A runtime-specific cooperative producer may use it only after that producer independently passes the integration admission contract.

If neither source is admitted for a domain:

```text
authority = NOT_ADMITTED
```

`NOT_ADMITTED` is truthful coverage state, not a failure, not a transport, and never enters a healthy denominator.

## Single-repository release boundary

The v0.3.1 critical path obeys:

```text
HOOKSTAT_RELEASE_CAN_COMPLETE_WITH_HOOKSTAT_REPO_ONLY=true
EXTERNAL_REPOSITORY_WRITE_REQUIRED=false
EXTERNAL_PRODUCER_REQUIRED_FOR_RELEASE=false
EXTERNAL_PRODUCER_MERGE_REQUIRED_FOR_RELEASE=false
EXTERNAL_PRODUCER_PACKAGE_REQUIRED_FOR_RELEASE=false
EXTERNAL_PRODUCER_PUBLICATION_REQUIRED_FOR_RELEASE=false
```

A HookStat unattended development train MUST NOT make correctness or release progress conditional on modifying another product repository.

External producers are consumers of the HSIP contract. They own their implementation lifecycle and may be qualified/admitted independently after HookStat publishes or while it is under development.

## Admission layers

Do not collapse these states:

```text
HSIP_PROTOCOL_QUALIFIED
HOOKSTAT_IPC_INFRASTRUCTURE_READY
INTEGRATION_CONFORMANT
INTEGRATION_ADMITTED
DOMAIN_AUTHORITY_SELECTED
PUBLICLY_RELEASED
```

The first two are HookStat repository responsibilities.

`INTEGRATION_CONFORMANT` and `INTEGRATION_ADMITTED` are properties of a named runtime/integration candidate and are not HookStat release prerequisites unless that integration is bundled in HookStat itself.

No external cooperative producer is bundled in v0.3.1.

## Performance correctness

The G28 budgets remain frozen; this scope reset does not weaken them.

HookStat must prove its own HSIP substrate with an in-repository reference producer/conformance harness:

```text
REFERENCE_HSIP_P95_MS<=1
REFERENCE_HSIP_P99_MS<=2
REFERENCE_HSIP_OBSERVATION_GAPS=0
HOOKSTAT_INDUCED_TIMEOUTS_FOR_HEALTHY_HOOK=0
```

Each external cooperative producer must separately satisfy the same producer-admission contract before it can become production authority:

```text
EXTERNAL_INTEGRATION_P95_MS<=1
EXTERNAL_INTEGRATION_P99_MS<=2
EXTERNAL_INTEGRATION_OBSERVATION_GAPS=0
```

Failure by one external producer blocks that producer's admission only. It does not retroactively make HookStat's protocol, broker, ledger, diagnostics, or release candidate invalid when the HookStat reference substrate passes.

The historical transparent-shim failures remain frozen truth:

```text
G28_REFERENCE_TRANSPARENT_WARM_P95_P99_MS=20/25
ONE_TIME_V031_TRANSPARENT_WARM_P95_P99_MS=25/30
TRANSPARENT_SHIM_20_25=FAIL
TRANSPARENT_SHIM_25_30=FAIL
TRANSPARENT_SHIM_PRODUCTION_ADMISSION=false
TRANSPARENT_SHIM_PRODUCTION_ACTIVATION=false
FURTHER_BUDGET_RELAXATION=false
```

## Core invariants

```text
RUNTIME_NEUTRAL_EVIDENCE_CORE=true
CANONICAL_EVIDENCE=true
RUNTIME_NEUTRAL_CORRELATOR=true
DOMAIN_AUTHORITY_ROUTING=true

NATIVE_FIRST=true
IPC_ADMITTED_INTEGRATION_ONLY_FALLBACK=true
NO_THIRD_EVIDENCE_PATH=true
NOT_ADMITTED_IS_EVIDENCE_PATH=false

ONE_AUTHORITY_PER_COVERAGE_DOMAIN=true
NO_DOUBLE_COUNTING=true
SHADOW_EVIDENCE_IN_DENOMINATOR=false
MISSING_EVIDENCE_NEVER_BECOMES_SUCCESS=true

IPC_PROTOCOL_RUNTIME_NEUTRAL=true
IPC_BROKER_RUNTIME_NEUTRAL=true
IPC_INTEGRATION_RUNTIME_SPECIFIC=true

BINARY_IPC_PROTOCOL=true
EPHEMERAL_BROKER=true
COMPACT_APPEND_WAL=true
GLOBAL_MANDATORY_DAEMON=false
NETWORK_LISTENER=false
REMOTE_TELEMETRY=false

NORMAL_CODEX_LAUNCH=codex
HOOKSTAT_AS_CODEX_LAUNCHER=false

RAW_PROMPT_CONTENT_PERSISTED=false
RAW_ASSISTANT_CONTENT_PERSISTED=false
RAW_TOOL_CONTENT_PERSISTED=false
RAW_STDOUT_PERSISTED=false
RAW_STDERR_PERSISTED=false
RAW_COMMAND_PERSISTED_IN_LEDGER=false

LEGACY_V03_EVIDENCE_PRESERVED=true
```

## Revised dependency DAG

```text
PUBLIC v0.3.0
      │
      ▼
HS-G28 — Hot Path Performance Baseline                 [ACCEPTED]
      │
      ▼
HS-G29 — Runtime-Neutral Evidence Core                 [ACCEPTED]
      │
      ▼
HS-G34 — Native Framework / Codex Native L1            [ACCEPTED]
      │
      ▼
HS-G35 — Runtime-Neutral IPC Broker / WAL              [ACCEPTED]
      │
      ▼
HS-G36 — Ultra-Light IPC Clients / Transparent Shim    [ACCEPTED]
      │
      ▼
HS-G37 — Authority Routing / Native L2 / Migration     [ACCEPTED]
      │
      ▼
HS-G38A — Single-Repo Scope & Admission Contract Reset
      │
      ├──────────────────────────┐
      ▼                          ▼
HS-G38B                      HS-G38C
HSIP v1 Conformance Kit      HookStat Windows Hardening
      │                          │
      └────────────┬─────────────┘
                   ▼
              HS-G38D
         Acceptance / PR Closeout
                   │
                   ▼
              HS-G38R
       v0.3.1 Hardening & Release
                   │
                   ▼
            PUBLIC v0.3.1
```

G38B and G38C are independent after G38A. A single unattended writer should normally execute G38B then G38C sequentially. Separate isolated writers may execute them in parallel only when they do not share a branch/worktree and repository governance permits it. G38D is the convergence gate and cannot pass until both are accepted.

## External integration DAG

External integrations consume the contract but are not on the release critical path:

```text
                 HSIP v1 conformance contract
                         /      |      \
                        /       |       \
                       ▼        ▼        ▼
                 Integration A  B        C
                       │        │        │
                       ▼        ▼        ▼
                  conformance conformance conformance
                       │        │        │
                       ▼        ▼        ▼
                   admission admission admission
```

For every such track:

```text
BLOCKS_HOOKSTAT_V031_RELEASE=false
CROSS_REPO_WRITE_FROM_HOOKSTAT_TRAIN=false
```

A historical or future TabBeacon integration is one possible external integration; it has no special authority in HookStat Core.

## Goal index

| Goal | Scope | Estimated effort |
| --- | --- | ---: |
| G28 | Windows hot-path baseline and frozen budgets | accepted |
| G29 | CanonicalEvidence, neutral correlator, authority/coverage | accepted |
| G34 | Native contracts and Codex Native L1 | accepted |
| G35 | Versioned IPC, Named Pipe/UDS, bounded broker, WAL | accepted |
| G36 | IPC clients and retained non-admitted transparent shim | accepted |
| G37 | Codex authority routing, Native L2 state, legacy migration | accepted |
| G38A | Single-repo release boundary and admission-contract reset | 2–3 h |
| G38B | HSIP v1 reference producer + conformance/performance/privacy kit | 4–7 h |
| G38C | Diagnostics, broker/WAL recovery, Windows concurrency/resource hardening | 3–5 h |
| G38D | Exact-head qualification, independent review, reconcile/merge G38 | 2–4 h |
| G38R | Release freeze, package/dry-run, upgrade/fresh-install, docs | 4–6 h |
| **Remaining** | **from G38A** | **15–25 h** |

Existing draft G38 implementation may satisfy substantial G38C work. Preserve valid tests and evidence; do not rewrite working code simply because the execution contract changed.

## G38A — scope reset

G38A updates governance only. It must establish:

```text
EXTERNAL_INTEGRATION_REQUIRED_FOR_RELEASE=false
HSIP_PROTOCOL_RELEASE_OWNED_BY_HOOKSTAT=true
INTEGRATION_ADMISSION_SEPARATE=true
DOMAIN_WITHOUT_ADMITTED_SOURCE=NOT_ADMITTED
```

It must not delete historical receipts or hide previous external-producer performance findings.

## G38B — HSIP v1 conformance kit

HookStat must provide an in-repository reference producer and deterministic conformance harness covering at minimum:

```text
START / COMPLETE
start-only / complete-only
out-of-order
duplicate / replay
uncertain ACK / no replay
malformed frame
oversized frame / identifier
broker unavailable / restart
concurrent producers
privacy/data minimization
identity stability
WAL recovery
performance p50/p95/p99
```

The reference producer is a test/conformance instrument, not a production runtime adapter and not a third evidence path.

## G38C — HookStat-only Windows hardening

Qualify HookStat itself on Owner-controlled Windows using the reference producer and controlled clients:

```text
1 / 5 / 10 concurrent clients
10,000+ controlled evidence frames
broker idle/restart/reconnect
WAL valid-prefix and partial-tail recovery
bounded diagnostics
process/resource leak checks
report/doctor/TUI consistency
privacy/security review
```

Also run ordinary `codex` smoke to prove HookStat does not require a launcher, trust bypass, or unwanted configuration mutation. If no admitted Native or external IPC producer exists, the corresponding runtime domains must remain `NOT_ADMITTED`; that is an allowed truthful state.

## G38D — convergence and closeout

G38D requires:

```text
G38A=PASS
G38B=PASS
G38C=PASS
REFERENCE_HSIP_PERFORMANCE=PASS
DIAGNOSTICS=PASS
RECOVERY=PASS
PRIVACY_REVIEW=PASS
SECURITY_REVIEW=PASS
WINDOWS_CI=PASS
UBUNTU_CI=PASS
FRESH_INDEPENDENT_REVIEW=PASS
EXTERNAL_INTEGRATION_REQUIRED=false
```

The existing G38 draft PR should be reconciled with accepted main after G38A and with G38B/G38C results before merge. Stale text that names an external product as a mandatory G38 blocker must be removed, while historical receipts remain intact.

## G38R release semantics

A fresh v0.3.1 install must be able to:

```text
run report / doctor / TUI with no prior state
run HSIP conformance/reference qualification
report Native capability truthfully
report IPC integration admission truthfully
report uncovered domains as NOT_ADMITTED
launch ordinary codex without HookStat wrapper semantics
```

An admitted external cooperative producer is optional. If none is installed/admitted, release notes must say so explicitly rather than imply live evidence coverage.

## Explicitly deferred

Do not fold these into v0.3.1:

- any external producer implementation as a HookStat release dependency;
- OpenCode production adapter;
- DeepSeek Harness production adapter;
- Claude Code production adapter;
- Agy production adapter;
- transparent-shim rearchitecture (`G36T`, v0.3.2 or later);
- OTel as reliability evidence;
- session-history inference or polling as a third evidence path;
- Web UI / remote dashboard;
- cloud/distributed aggregation;
- remote/network broker;
- AI root-cause diagnosis;
- HookStat-as-Codex launcher;
- broad TUI redesign.

## Unattended-train authority

After this roadmap is accepted, an unattended HookStat train may autonomously:

- implement G38B/G38C/G38D and then G38R in dependency order;
- add/refine in-repository reference producer and test fixtures;
- run Windows/Ubuntu/local tests and benchmarks;
- repair HookStat-only CI and correctness defects;
- update HookStat docs and sanitized receipts;
- create/reconcile HookStat branches and PRs;
- merge a Goal after its exact acceptance gates and review pass.

It may not:

```text
WRITE_EXTERNAL_REPOSITORY=true
WEAKEN_COVERAGE_TRUTHFULNESS=true
ADD_THIRD_EVIDENCE_PATH=true
WEAKEN_FROZEN_PERFORMANCE_BUDGET=true
DELETE_LEGACY_EVIDENCE=true
PUBLISH_CRATES_IO=true
CREATE_PUBLIC_V031_TAG=true
CREATE_PUBLIC_GITHUB_RELEASE=true
```

Public publication remains an explicit Owner gate.

## Mandatory stop gates

1. **HSIP substrate:** if the HookStat reference producer + broker cannot satisfy the frozen 1/2 ms cooperative transport budget on the admitted Owner Windows test methodology, stop G38B/G38D and fix HookStat substrate; do not blame or modify an external producer.
2. **Native:** ordinary Codex Native L2 may remain upstream-unavailable. Record it truthfully; it is not a release blocker by itself.
3. **Coverage:** a domain with no admitted source remains `NOT_ADMITTED`; never convert it into success.
4. **Before G38R:** any reproducible HookStat-induced timeout/failure, persistence corruption, privacy/security defect, or false coverage blocks release.
