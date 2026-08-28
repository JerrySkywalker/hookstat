# HSIP v1 Conformance and Integration Admission

## Purpose

This document defines the boundary between HookStat's runtime-neutral local IPC substrate and runtime-specific cooperative producers.

HookStat owns:

```text
HSIP v1 protocol
local transport abstraction
broker
WAL/recovery
canonical evidence ingestion
reference producer
conformance harness
diagnostics
admission criteria
```

A runtime/integration repository owns its concrete producer implementation.

HookStat v0.3.1 release work must not require modifying that repository.

## Release versus integration admission

These states are deliberately separate:

```text
HSIP_PROTOCOL_QUALIFIED
HOOKSTAT_IPC_INFRASTRUCTURE_READY
INTEGRATION_CONFORMANT
INTEGRATION_ADMITTED
DOMAIN_AUTHORITY_SELECTED
```

HookStat v0.3.1 requires the first two.

A specific external integration requires the next two before its evidence may become production authority.

A domain without an admitted Native source or admitted IPC integration is:

```text
NOT_ADMITTED
```

That is truthful coverage state and never becomes implicit success.

## Cross-repository boundary

During a HookStat release/development train:

```text
EXTERNAL_REPOSITORY_WRITE=false
EXTERNAL_REPOSITORY_MERGE_REQUIRED=false
EXTERNAL_REPOSITORY_PACKAGE_REQUIRED=false
EXTERNAL_REPOSITORY_RELEASE_REQUIRED=false
```

HookStat may consume an externally supplied candidate artifact or public source for black-box compatibility/admission testing, but the producer's code changes are owned by its own development track.

If an external producer fails conformance or performance, record the failure against that producer candidate. Do not repair it by writing the external repository from the HookStat release train.

## Reference producer

HookStat must include an in-repository reference HSIP producer used only for:

- protocol conformance;
- broker/WAL/recovery testing;
- deterministic malformed/edge-case fixtures;
- Windows/Unix transport qualification;
- performance baseline and regression testing.

The reference producer is **not** a runtime adapter, is not selected as production authority for Codex or any other runtime, and does not create a third evidence path.

Required invariant:

```text
REFERENCE_PRODUCER_PRODUCTION_AUTHORITY=false
```

## HSIP v1 frame contract

The conformance suite must preserve the frozen protocol invariants already implemented by G35/G36, including:

```text
PROTOCOL_VERSION=1
FRAME_MAX_BYTES=1024
BOUNDED_IDENTIFIERS=true
LOCAL_ONLY=true
NETWORK_LISTENER=false
```

Lifecycle evidence must contain only bounded structural metadata required for correlation and attribution. It must not contain raw prompt, assistant, tool payload, stdout, stderr, credential, token, or raw command content.

## Required producer semantics

A conformant cooperative producer must:

1. emit one START for one observed invocation when START is accepted;
2. emit one COMPLETE for the same invocation when terminal evidence is available;
3. preserve stable runtime/invocation/handler/event identity;
4. fail open when HookStat broker/transport is unavailable;
5. never convert observation failure into runtime Hook failure;
6. never replay a frame after an uncertain write/ACK boundary;
7. never duplicate one production invocation after reconnect/restart;
8. preserve truthful missing/incomplete evidence;
9. avoid raw private content;
10. preserve the observed runtime's normal launch and business semantics.

## Conformance matrix

The HookStat conformance kit must provide deterministic coverage for at least:

```text
START + COMPLETE
START only
COMPLETE only
out-of-order COMPLETE/START
duplicate START
duplicate COMPLETE
replay/idempotence
uncertain ACK / no replay
broker absent
broker unavailable after START
broker restart
concurrent producer startup
client reconnect
malformed frame
unknown frame kind
oversized frame
oversized identifier
truncated frame
WAL valid-prefix recovery
WAL partial-tail recovery
coverage degradation after dropped/withheld evidence
privacy field exclusion
identity stability
```

Synthetic fixtures must be explicitly labeled synthetic/control evidence and must never be described as real runtime event coverage.

## Performance contract

The G28 cooperative budget remains frozen:

```text
P95_MS<=1
P99_MS<=2
OBSERVATION_GAPS=0
```

### HookStat substrate gate

The reference producer + HookStat broker must satisfy the budget under the repository-governed Owner Windows methodology. This proves HookStat's own substrate.

Failure here blocks HookStat G38B/G38D.

### External producer admission gate

Every external producer must independently satisfy the same producer transport budget under a qualified methodology that isolates producer/HSIP overhead from unrelated process-spawn or shell-teardown cost.

Failure here blocks only that producer candidate's admission.

The budget must not be weakened to admit a producer.

## Admission evidence

A named integration admission receipt must identify at minimum:

```text
INTEGRATION_ID
RUNTIME
PRODUCER_VERSION_OR_SHA
PACKAGE_OR_BINARY_SHA256
HOOKSTAT_PROTOCOL_VERSION
HOOKSTAT_REFERENCE_SHA
PLATFORM
PROTOCOL_CONFORMANCE
CORRELATION
FAIL_OPEN
UNCERTAIN_WRITE_DUPLICATE_GUARD
PRIVACY
SECURITY
P50_MS
P95_MS
P99_MS
OBSERVATION_GAPS
INDEPENDENT_REVIEW
ADMISSION_DISPOSITION
```

Allowed admission disposition values:

```text
ADMITTED
NOT_ADMITTED
REVOKED
DEGRADED
UNPROVEN
```

`PACKAGE_REAL`, `CONFORMANT`, `REVIEWED`, and `ADMITTED` are not synonyms.

## Authority selection

HookStat authority routing remains:

```text
if Native admitted:
    Native
else if named IPC integration admitted:
    IPC
else:
    NOT_ADMITTED
```

An integration admission can be domain-scoped. Admission of one event/source domain does not silently admit all runtime event families.

## Compatibility and versioning

HSIP protocol changes must be versioned. A producer pinned to protocol v1 cannot be silently interpreted as a future incompatible protocol.

HookStat may preserve backward-compatible v1 decoding while introducing a future protocol version, but admission evidence must name the protocol version actually qualified.

## Privacy and security

Conformance must verify:

- local per-user endpoint scope;
- bounded frame parsing;
- malformed client rejection without broker corruption;
- safe state-root/path containment;
- no raw private content in frames/WAL/ledger/diagnostics;
- no network listener;
- no trust/config bypass;
- fail-open producer behavior;
- no global mandatory daemon.

## Relationship to G38

G38B owns the reference producer and conformance kit.

G38C owns HookStat Windows broker/diagnostics/recovery/resource hardening.

G38D converges those proofs and closes the existing G38 implementation PR.

External integrations are independent tracks and do not block G38D or G38R unless HookStat later chooses to bundle one, which v0.3.1 explicitly does not.

## v0.3.1 release statement

HookStat v0.3.1 ships HSIP infrastructure and conformance. External cooperative
producers have independent admission lifecycles. A release candidate may prove
the reference producer, broker, WAL, diagnostics, and conformance surface while
still reporting every unadmitted runtime domain as `NOT_ADMITTED`.

The reference producer is a test and qualification instrument only. It cannot
be used to assert Native availability, IPC admission, live runtime coverage, or
production authority for Codex or another runtime. Third-party producers must
qualify their own exact package/binary and receive an explicit domain-scoped
admission receipt; no HookStat Core change is needed to begin that process.
