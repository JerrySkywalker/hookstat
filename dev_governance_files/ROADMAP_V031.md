# HookStat v0.3.1 execution roadmap

## Status

IN PROGRESS from public v0.3.0 baseline
`a33a3be56982c6ca00699019883a047a1aca748b`. G36 release-scope recovery admits
cooperative IPC for v0.3.1 while retaining the transparent shim as
correctness-qualified but not production-admitted after both governed warm
contracts failed.

This file is the compact long-train entrypoint for v0.3.1. Product intent, architecture, invariants, performance correctness, release boundaries, and future-runtime constraints are defined in [`../goals/HS-V031-NATIVE-IPC-HIGH-PERFORMANCE-EVIDENCE-RUNTIME.md`](../goals/HS-V031-NATIVE-IPC-HIGH-PERFORMANCE-EVIDENCE-RUNTIME.md). Each numbered Goal owns its exact implementation and exit gates.

## Product direction

**v0.3.1 — Runtime-Neutral Native & IPC High-Performance Evidence Runtime** keeps Codex as the only production runtime while replacing the v0.3 instrumentation-centric hot path with two and only two evidence paths:

1. **Runtime-native evidence, preferred:** consume authoritative Hook lifecycle evidence produced and owned by the coding-agent runtime without proxying the observed Hook.
2. **Runtime-neutral local IPC, conditional fallback:** when a coverage domain
   lacks admitted native evidence, use IPC only if a concrete IPC integration
   for that domain is itself admitted. Otherwise the domain is `NOT_ADMITTED`.

For v0.3.1 cooperative IPC is admitted. The transparent shim is implemented and
correctness-qualified but is not performance-admitted or production-activated;
its rearchitecture is deferred to G36T for v0.3.2 or later.

The release is Codex-first and Codex-only. The architecture must nevertheless allow future OpenCode, DeepSeek Harness, Claude Code, Agy, or another runtime to add a runtime integration without rewriting the evidence core, broker, ledger, analytics, workbench, or TUI.

## Dependency sequence

```text
PUBLIC v0.3.0
a33a3be56982c6ca00699019883a047a1aca748b
        ↓
HS-G28   Hot Path Performance Baseline
        ↓
HS-G29   Runtime-Neutral Evidence Core
        ↓
HS-G34   Native Evidence Framework + Codex Native L1 Qualification
        ↓
HS-G35   Runtime-Neutral IPC Broker / WAL
        ↓
HS-G36   Ultra-Light IPC Clients / Transparent Shim
        ↓
HS-G37   Codex Evidence Routing / Native L2 / Migration (acceptance candidate)
        ↓
HS-G38   Performance & Windows Dogfood Hardening
        ↓
HS-G38R  v0.3.1 Hardening & Release
        ↓
PUBLIC v0.3.1
```

`HS-G30X` through `HS-G33X` remain reserved future-runtime tracks and are not v0.3.1 dependencies.

Default execution is sequential. A long autonomous train may continue only after the predecessor acceptance conditions are actually satisfied and merged, except for the bounded stacked-development exception below. `HS-G38R` is a mandatory release boundary; public crates.io/tag/GitHub Release publication remains an explicit Owner gate.

## Bounded stacked-development exception

A predecessor Goal whose **implementation is complete** may permit implementation of exactly one successor Goal before predecessor acceptance/merge when the only remaining blockers are external to the implementation itself.

The exception is admitted only when all of the following are true:

```text
PREDECESSOR_IMPLEMENTATION_COMPLETE=true
PREDECESSOR_KNOWN_CODE_BLOCKERS=0
PREDECESSOR_CI=PASS
PREDECESSOR_ARCHITECTURE_STABLE=true
MAX_STACK_DEPTH=1
```

Admitted blocker classes are limited to:

```text
OWNER_ENVIRONMENT
REAL_HARDWARE_OR_HOST_QUALIFICATION
EXTERNAL_SERVICE_AVAILABILITY
INDEPENDENT_REVIEW
OWNER_ONLY_EVIDENCE_OR_APPROVAL
```

The exception is **not** available when the predecessor has an unresolved correctness bug, architecture uncertainty, persistence/data-corruption risk, privacy/security defect, unstable API/format contract, or unresolved evidence/failure semantics.

A stacked successor must:

- branch from the exact predecessor implementation head;
- open a stacked PR against the predecessor branch rather than `main`;
- remain unmerged to `main` until the predecessor is accepted and merged;
- preserve the predecessor's pending acceptance evidence rather than reclassifying it;
- be rebased/retargeted to accepted `main` and re-run exact-head CI before its own merge;
- stop after that one successor Goal if the predecessor still has not merged.

Acceptance work for the predecessor may proceed in parallel with successor implementation. This exception changes execution scheduling only; it does not weaken any Goal acceptance criterion, frozen performance budget, review requirement, or release gate.

For the current G35/G36 sequence, G36 stacked implementation is permitted only while G35 has no known implementation/correctness blocker and its remaining gates are external acceptance evidence/review. G37 remains blocked until both G35 and G36 are accepted into `main`.

## Goal index

| Goal | Scope | Estimated effort |
| --- | --- | ---: |
| G28 | Real Windows hot-path benchmark, current-proxy cost decomposition, provisional-to-frozen performance budget | 5–8 h |
| G29 | `CanonicalEvidence`, runtime-neutral correlator, authority domains, coverage/admission semantics | 8–12 h |
| G34 | Runtime-neutral Native contracts and controlled Codex HookStarted/HookCompleted qualification | 8–12 h |
| G35 | Versioned binary IPC, Windows Named Pipe / Unix Domain Socket, bounded broker, append WAL, group durability | 9–14 h |
| G36 | Production-admitted cooperative IPC plus retained, correctness-qualified, non-admitted transparent shim | 10–16 h |
| G36T | Deferred transparent-shim rearchitecture; v0.3.2 or later, not a v0.3.1 dependency | Deferred |
| G37 | Codex Native/IPC authority routing, ordinary `codex` Native L2 qualification, shadow proof, v0.3 legacy migration | 8–12 h |
| G38 | Real Windows concurrency/dogfood, tail latency, broker recovery, diagnostics/self-observability, privacy/security review | 7–11 h |
| G38R | v0.3.1 release freeze, upgrade/fresh-install proof, public release closure | 4–6 h |
| **Total** | **v0.3.1** | **59–91 h** |

Expected normal execution is approximately **65–75 effective engineering hours**. These are workload estimates, not permissions to bypass Goal gates.

## v0.3.1 admitted outcomes

```text
PRODUCTION_RUNTIME=Codex
CODEX_ONLY_RELEASE_REQUIREMENT=true

RUNTIME_NEUTRAL_EVIDENCE_CORE=true
CANONICAL_EVIDENCE=true
RUNTIME_NEUTRAL_CORRELATOR=true
DOMAIN_AUTHORITY_ROUTING=true

NATIVE_FRAMEWORK=true
CODEX_NATIVE_L1=true
CODEX_NATIVE_L2=PASS_OR_TRUTHFULLY_UPSTREAM_UNAVAILABLE

RUNTIME_NEUTRAL_IPC=true
BINARY_IPC_PROTOCOL=true
EPHEMERAL_BROKER=true
COMPACT_APPEND_WAL=true
COOPERATIVE_IPC=PRODUCTION_ADMITTED
TRANSPARENT_IPC_SHIM=QUALIFIED_NOT_ADMITTED_PERFORMANCE
TRANSPARENT_SHIM_PRODUCTION_ACTIVATION=false

FULL_CLI_HOT_PATH=false
PER_RECORD_FSYNC_HOT_PATH=false
JSON_RECEIPT_HOT_PATH=false
FULL_MANIFEST_PARSE_HOT_PATH=false

ORIGINAL_HANDLER_TIMEOUT_PRESERVED=true
HOOKSTAT_INDUCED_TIMEOUTS=0

LEGACY_V03_EVIDENCE_PRESERVED=true
OPENCODE_PRODUCTION_ADAPTER=false
DEEPSEEK_PRODUCTION_ADAPTER=false
```

## Cross-cutting invariants

```text
NATIVE_FIRST=true
IPC_ADMITTED_INTEGRATION_ONLY_FALLBACK=true
NO_THIRD_EVIDENCE_PATH=true
NOT_ADMITTED_IS_EVIDENCE_PATH=false

NATIVE_MEANS_RUNTIME_OWNED_EVIDENCE=true
NATIVE_TRANSPORT_OPAQUE_TO_CORE=true
IPC_PROTOCOL_RUNTIME_NEUTRAL=true
IPC_BROKER_RUNTIME_NEUTRAL=true
IPC_INTEGRATION_RUNTIME_SPECIFIC=true

ONE_AUTHORITY_PER_COVERAGE_DOMAIN=true
NO_DOUBLE_COUNTING=true
SHADOW_EVIDENCE_IN_DENOMINATOR=false
MISSING_EVIDENCE_NEVER_BECOMES_SUCCESS=true

CANONICAL_EVIDENCE_BEFORE_HOOK_INVOCATION=true
CORRELATION_RUNTIME_NEUTRAL=true
IDENTITY_RESOLUTION_RUNTIME_SPECIFIC=true

NORMAL_CODEX_LAUNCH=codex
HOOKSTAT_AS_CODEX_LAUNCHER=false
GLOBAL_MANDATORY_DAEMON=false
NETWORK_LISTENER=false
REMOTE_TELEMETRY=false

RAW_PROMPT_CONTENT_PERSISTED=false
RAW_ASSISTANT_CONTENT_PERSISTED=false
RAW_TOOL_CONTENT_PERSISTED=false
RAW_STDOUT_PERSISTED=false
RAW_STDERR_PERSISTED=false
RAW_COMMAND_PERSISTED_IN_LEDGER=false

OBSERVABILITY_MUST_NOT_DISTURB_OBSERVED_SYSTEM=true
MISSING_EVIDENCE_NEVER_BECOMES_SUCCESS=true
FUTURE_RUNTIME_ADAPTER_REQUIRES_CORE_REWRITE=false
```

A local on-demand, idle-expiring per-user broker is permitted. It is not a mandatory machine-global daemon, network service, remote telemetry system, or self-update mechanism.

## Performance correctness

G28 is the only Goal allowed to calibrate the provisional numeric targets below. Once G28 records real Owner Windows measurements, the accepted budget becomes release-governing for G36–G38R.

Current governed targets and retained results:

```text
NATIVE_ADDED_SYNCHRONOUS_LATENCY_MS=0
COOPERATIVE_IPC_P95_MS<=1
COOPERATIVE_IPC_P99_MS<=2
G28_REFERENCE_TRANSPARENT_WARM_P95_P99_MS=20/25
ONE_TIME_V031_TRANSPARENT_WARM_P95_P99_MS=25/30
TRANSPARENT_SHIM_20_25=FAIL
TRANSPARENT_SHIM_25_30=FAIL
TRANSPARENT_SHIM_PRODUCTION_ADMISSION=false
TRANSPARENT_SHIM_COLD_P95_MS<=50
HOOKSTAT_INDUCED_TIMEOUTS_FOR_HEALTHY_HOOK=0
FURTHER_BUDGET_RELAXATION=false
```

Hosted CI may detect structural performance regressions but does not replace the real Windows p50/p95/p99 release gate.

## Explicitly deferred

Do not fold these into v0.3.1 merely because time remains:

- OpenCode production adapter;
- DeepSeek Harness production adapter;
- Claude Code production adapter;
- Agy production adapter;
- OTel as reliability evidence;
- rollout/session-history inference as a third evidence path;
- polling as a third evidence path;
- Web UI or remote dashboard;
- cloud/distributed aggregation;
- remote or network broker;
- AI root-cause diagnosis;
- general daemon framework;
- Codex launcher wrapper;
- broad TUI redesign.

Synthetic multi-runtime fixtures in G29 prove the core abstraction only; they do not constitute non-Codex runtime support.

## Suggested 12h train partitions

```text
Train A: G28 + begin G29
Train B: finish G29 + G34
Train C: G35
Train D: G36
Train E: G37
Train F: G38
Train G: G38R -> Owner publication gate
```

Partitions are estimates. Goal acceptance and truthful state remain authoritative. A stacked train created by the bounded exception remains capped at one successor Goal and does not advance the downstream train schedule.

## Mandatory stop gates

### Gate A — after G28

If the chosen local IPC transport cannot satisfy a credible low-latency hot-path budget on the Owner Windows environment, stop before G35 and revise the transport design. Do not proceed on an unmeasured assumption.

### Gate B — during G34/G37 Native qualification

If ordinary `codex` offers no supported external attach path for authoritative
HookStarted/HookCompleted evidence, record Native L2 as upstream-unavailable.
Use IPC only for domains with an admitted cooperative integration; otherwise
route the domain to `NOT_ADMITTED`. Do not introduce a HookStat launcher or the
non-admitted transparent shim to force coverage.

### Gate C — before G38R

Any reproducible HookStat-induced timeout or execution failure in a previously healthy Hook blocks release.
