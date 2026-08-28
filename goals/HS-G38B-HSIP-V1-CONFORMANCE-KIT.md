# HS-G38B — HSIP v1 Reference Producer & Conformance Kit

## Status

PLANNED after accepted G38A. Independent of G38C after that point.

## Objective

Prove HookStat's own HSIP v1 protocol/broker substrate without relying on a runtime-specific external repository.

Build an in-repository reference producer and deterministic conformance harness that can be reused by future external integrations as the admission oracle.

## Scope boundary

```text
PRIMARY_REPOSITORY=hookstat
EXTERNAL_REPOSITORY_WRITE=false
REFERENCE_PRODUCER_PRODUCTION_AUTHORITY=false
REFERENCE_PRODUCER_RUNTIME_ADAPTER=false
NO_THIRD_EVIDENCE_PATH=true
```

Do not implement or repair TabBeacon, OpenCode, DeepSeek Harness, Claude Code, Agy, or another external producer.

## Reference producer requirements

The reference producer should be the smallest maintainable in-repository implementation able to generate exact HSIP v1 frames and exercise HookStat's public/internal protocol contract.

It must:

- use the same protocol/frame bounds as real producers;
- support deterministic identity/lifecycle fixtures;
- expose controlled failure modes for conformance tests;
- avoid product-level analytics/TUI dependencies;
- never be selected as production authority for a real runtime.

Prefer a test/support module or dedicated conformance binary only when repository packaging/governance supports it. Do not add a machine-global daemon.

## Required conformance matrix

At minimum prove:

```text
START + COMPLETE
START only
COMPLETE only
out-of-order lifecycle
duplicate START
duplicate COMPLETE
replay idempotence
uncertain write / ACK loss -> no replay
broker absent -> fail open
broker lost after START
broker restart
client reconnect
concurrent producer startup
malformed frame
unknown frame kind
oversized frame
oversized identifier
truncated frame
WAL valid-prefix recovery
WAL partial-tail recovery
dropped/withheld evidence -> visible coverage degradation
privacy field exclusion
identity stability
```

## Controlled concurrency

Exercise at minimum:

```text
1 producer
5 concurrent producers
10 concurrent producers
1,000 frames
10,000 frames
```

Higher bounded stress volume is optional when it reveals useful behavior.

## Frozen performance gate

On the repository-governed Owner Windows methodology, measure the HookStat reference producer + HookStat broker transport.

Required:

```text
REFERENCE_HSIP_P50_MS=<measured>
REFERENCE_HSIP_P95_MS<=1
REFERENCE_HSIP_P99_MS<=2
REFERENCE_HSIP_OBSERVATION_GAPS=0
```

Use release-mode artifacts and isolate HSIP transport latency from unrelated shell/process-spawn teardown cost.

Do not weaken the 1/2 ms limits.

If the reference substrate fails, profile and fix HookStat or the reference client path inside this repository. Do not move the failure to an external integration and do not proceed to G38D with a false PASS.

## Recovery semantics

Verify:

- bounded client behavior while broker is absent;
- broker idle expiry/restart;
- no duplicate canonical invocation after reconnect/recovery;
- no uncertain frame replay;
- WAL valid prefix survives partial tail;
- malformed input cannot corrupt broker state;
- bounded queue/frame/connection behavior remains enforced.

## Privacy/security

Conformance must prove the protocol cannot require or persist:

```text
raw prompt
assistant content
tool payload
raw command
stdout
stderr
credential/token
arbitrary private file content
```

Revalidate local endpoint scope, frame bounds, unsafe state-root handling, and no network listener.

## External-integration admission fixture

Provide a deterministic way for a future integration candidate to be checked without changing HookStat Core. The harness should be able to consume a named candidate/artifact/producer interface and produce a machine-readable admission receipt skeleton containing:

```text
INTEGRATION_ID
RUNTIME
PRODUCER_SHA_OR_VERSION
PACKAGE_OR_BINARY_SHA256
HSIP_PROTOCOL_VERSION
HOOKSTAT_SHA
PLATFORM
PROTOCOL_CONFORMANCE
FAIL_OPEN
DUPLICATE_GUARD
PRIVACY
SECURITY
P50_MS
P95_MS
P99_MS
OBSERVATION_GAPS
ADMISSION_DISPOSITION
```

Do not require this external-candidate mode to be exercised for G38B acceptance.

## Required code gates

At settled exact head:

```text
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 clippy --locked --all-targets --all-features -- -D warnings
cargo +1.97.1 test --locked --all-features --no-fail-fast
cargo +1.97.1 build --locked --all-features
```

Run focused conformance/performance/recovery tests and exact-head hosted CI.

## Independent review

Fresh read-only review must inspect:

- protocol/version/bounds;
- reference producer cannot become production authority;
- uncertain-write replay semantics;
- performance methodology;
- privacy/security;
- no third path;
- no external repository dependency.

## Acceptance

```text
REFERENCE_PRODUCER=true
REFERENCE_PRODUCER_PRODUCTION_AUTHORITY=false
HSIP_V1_CONFORMANCE=PASS
UNCERTAIN_WRITE_DUPLICATE_GUARD=PASS
BROKER_RECOVERY=PASS
WAL_RECOVERY=PASS
PRIVACY=PASS
SECURITY=PASS
REFERENCE_HSIP_P95_MS<=1
REFERENCE_HSIP_P99_MS<=2
REFERENCE_HSIP_OBSERVATION_GAPS=0
EXTERNAL_REPOSITORY_WRITE=false
CODE_CI=PASS
INDEPENDENT_REVIEW=PASS
```

## Next

G38D may begin only after both G38B and G38C are accepted and represented in authoritative `main` or in the explicitly governed convergence state.
