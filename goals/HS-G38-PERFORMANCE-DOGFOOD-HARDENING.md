# HS-G38 — Performance, Conformance & Windows Hardening

## Status

REPLANNED after accepted G37.

G38 is now an umbrella Goal decomposed into:

```text
G38A — Single-Repo Scope & Admission Contract Reset
G38B — HSIP v1 Conformance Kit
G38C — HookStat Windows Hardening
G38D — Acceptance / PR Closeout
```

The old G38 requirement that HookStat release acceptance depend on a named external cooperative producer is superseded by G38A. Historical receipts remain factual history but no external product repository is a mandatory release dependency.

The preserved G38 safe-activation receipts describe historical, unadmitted
external-integration experiments. They remain evidence of what did and did
not happen at that time; they do not require external repair or block this
HookStat-only G38C path.

## Objective

Prove HookStat's v0.3.1 evidence substrate under deterministic HSIP conformance, Owner Windows concurrency/recovery/performance testing, diagnostics/self-observability, privacy/security review, and exact-head CI/review.

Production authority remains:

```text
Native where admitted
else IPC where a named integration is admitted
else NOT_ADMITTED
```

The transparent shim remains `QUALIFIED_NOT_ADMITTED_PERFORMANCE` and inactive.

## Single-repository boundary

```text
EXTERNAL_REPOSITORY_WRITE_REQUIRED=false
EXTERNAL_INTEGRATION_REQUIRED_FOR_G38=false
EXTERNAL_INTEGRATION_REQUIRED_FOR_G38R=false
```

G38 may use HookStat's in-repository reference producer to qualify HSIP. That reference producer is test-only and never production authority.

## DAG

```text
accepted G37
    │
    ▼
  G38A
   ├───────────────┐
   ▼               ▼
 G38B             G38C
 conformance      Windows hardening
   └───────┬───────┘
           ▼
          G38D
           │
           ▼
          G38R
```

## G38A — contract reset

G38A must align roadmap, master goal, this Goal, G38R, and the HSIP admission architecture around a single-repository HookStat release contract.

Acceptance:

```text
EXTERNAL_INTEGRATION_REQUIRED_FOR_RELEASE=false
HSIP_PROTOCOL_RELEASE_OWNED_BY_HOOKSTAT=true
INTEGRATION_ADMISSION_SEPARATE=true
DOMAIN_WITHOUT_ADMITTED_SOURCE=NOT_ADMITTED
HISTORICAL_RECEIPTS_PRESERVED=true
```

## G38B — HSIP conformance

G38B must provide an in-repository reference producer and deterministic conformance/performance harness.

Required matrix includes:

```text
START/COMPLETE
start-only / complete-only
out-of-order
duplicate/replay
uncertain ACK / no replay
broker unavailable/restart
malformed/oversized frames and identifiers
concurrent producers
WAL valid-prefix / partial-tail recovery
privacy field exclusions
identity stability
p50/p95/p99
```

Frozen substrate performance gate:

```text
REFERENCE_HSIP_P95_MS<=1
REFERENCE_HSIP_P99_MS<=2
REFERENCE_HSIP_OBSERVATION_GAPS=0
```

## G38C — HookStat Windows hardening

Qualify HookStat itself on an Owner-controlled Windows 11 environment with:

```text
PowerShell 7
Windows Terminal where relevant
HookStat v0.3.1 candidate
HookStat reference HSIP producer/clients
accepted/current Codex CLI for normal-launch smoke only
```

A named external producer is not required.

Required controlled workload families:

```text
1 client
5 concurrent clients
10 concurrent clients
10,000+ controlled evidence frames
```

Required recovery/resource tests:

- normal idle expiry and restart;
- broker killed while clients exist;
- bounded producer behavior while broker is unavailable;
- concurrent reconnect/startup race;
- WAL valid-prefix and partial-tail recovery;
- duplicate/replay resistance;
- dropped/withheld evidence becomes visible coverage degradation;
- no unwanted permanent process/resource leak;
- concurrent identities remain isolated;
- diagnostics remain bounded and truthful.

Normal `codex` smoke proves only that HookStat preserves ordinary launch semantics and does not require a wrapper, trust bypass, or unwanted configuration mutation. If no admitted Native or external IPC integration exists for a domain, the expected result is explicit `NOT_ADMITTED` rather than fabricated event coverage.

## Diagnostics / self-observability

Expose sufficient read-only diagnostics to answer, without raw private content:

```text
runtime
Native capability/admission
authoritative source per domain
named IPC integration admission where known
NOT_ADMITTED domains
broker state
queue lag
dropped/rejected/malformed frame count
WAL flush lag
recent/reference IPC latency percentiles
transparent shim status: qualified_not_admitted_performance
```

Diagnostics control frames are not evidence and must never enter WAL, ledger, replay, or denominators.

The Draft foundation uses a numeric diagnostics request/response on the
existing local HSIP control plane. These control frames are not lifecycle
evidence, never enter the WAL/ledger, cannot contribute to a denominator, and
do not create a third evidence transport. The fixed recent-latency window is
bounded to 128 in-memory samples and resets with the broker process.

## Structural regression guards

Prevent regressions such as:

```text
per-record fsync on producer path
JSON receipt creation on producer hot path
full HookStat CLI returning as production shim
unbounded frame/payload
unbounded queue/connections
shadow evidence entering denominator
NOT_ADMITTED entering denominator
transparent shim becoming production authority
network listener/global mandatory daemon
```

## Privacy and security review

Re-audit:

- IPC endpoint scope and permissions;
- broker/state path containment;
- WAL privacy;
- diagnostics privacy;
- malformed/oversized local client behavior;
- process containment and cleanup;
- no prompt/tool/stdout/stderr/raw command content;
- no trust/config bypass;
- retained transparent shim lockout.

## Existing G38 draft implementation

The pre-reset G38 draft PR contains substantial useful diagnostics, boundedness, broker recovery, concurrency, and structural-guard work.

After G38A is accepted:

1. preserve valid implementation/tests/evidence;
2. reconcile the branch with accepted `main`;
3. remove stale wording that names an external product as a mandatory release blocker;
4. map existing work into G38C and add only the missing G38B/G38D requirements;
5. do not discard working code merely because the acceptance contract changed.

## G38D acceptance

G38D can close only when:

```text
G38A=PASS
G38B=PASS
G38C=PASS

REFERENCE_HSIP_PERFORMANCE=PASS
HSIP_CONFORMANCE=PASS
BROKER_RECOVERY=PASS
PROCESS_LEAK=0
DIAGNOSTICS=PASS
COVERAGE_TRUTHFUL=PASS

HOOKSTAT_INDUCED_TIMEOUTS=0
HOOKSTAT_INDUCED_FAILURES=0

PRIVACY_REVIEW=PASS
SECURITY_REVIEW=PASS
WINDOWS_CI=PASS
UBUNTU_CI=PASS
FRESH_INDEPENDENT_REVIEW=PASS

TRANSPARENT_SHIM_PRODUCTION_ACTIVATION=false
NO_THIRD_EVIDENCE_PATH=true
EXTERNAL_INTEGRATION_REQUIRED=false
```

An external producer's conformance/admission result is not part of this acceptance block.

## Stop gates

Any of the following blocks G38D/G38R:

- HookStat reference HSIP substrate fails frozen performance budget;
- reproducible HookStat-induced timeout/failure;
- persistence/recovery corruption;
- false healthy coverage or denominator contamination;
- privacy/security defect;
- unbounded hot-path/resource behavior.

An external producer failing admission does **not** block HookStat G38.

## Estimated effort

```text
G38A  2–3 h
G38B  4–7 h
G38C  3–5 h
G38D  2–4 h
```

Existing G38 draft work is expected to reduce effective G38C effort.

## Next

`HS-G38R — v0.3.1 Hardening & Release` after G38D acceptance and merge.
