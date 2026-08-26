# HS-G36T — Transparent Shim Rearchitecture

## Status

DEFERRED to v0.3.2 or later. This track is not a v0.3.1 dependency and does
not authorize implementation, activation, publication, or another budget
change during G36/G37 landing.

## Starting truth

The optimized one-process `hookstat-hook` implementation and its correctness
evidence are retained. Production performance admission is not granted:

```text
G28_REFERENCE_WARM_P95_P99_MS=20/25
ONE_TIME_V031_WARM_P95_P99_MS=25/30
TRANSPARENT_SHIM_20_25=FAIL
TRANSPARENT_SHIM_25_30=FAIL
TRANSPARENT_SHIM_PRODUCTION_ADMISSION=false
FURTHER_AUTOMATIC_BUDGET_RELAXATION=false
```

The exact admitted failures remain immutable in `G36_PERF_EVIDENCE_INDEX.md`.
`NOT_ADMITTED` is a coverage state within IPC, not a third evidence path.

## Objective

Select and qualify a transparent integration architecture that can preserve the
existing capsule, privacy, timeout, exit-code, standard-stream, and Windows Job
containment semantics with repeatable warm tail margin. Begin from retained
evidence rather than repeating eliminated options.

At minimum compare:

- the retained optimized repeated-fresh one-process shim;
- an independently justified local architecture, only if its smallest complete
  semantic floor improves on the retained one-process evidence;
- any simpler runtime-owned integration discovered after v0.3.1.

Budget relaxation is not an architecture option. The G28 `20/25` values remain
the reference target, and the failed one-time v0.3.1 `25/30` cap is not silently
renewed or raised for a future release.

## Preserved invariants

```text
EVIDENCE_PATHS=2
NO_THIRD_EVIDENCE_PATH=true
NETWORK_TRANSPORT=false
MACHINE_GLOBAL_DAEMON=false
RAW_PRIVATE_CONTENT_PERSISTED=false
ORIGINAL_TIMEOUT_SEMANTICS_PRESERVED=true
NO_ORPHAN_CHILD_PROCESS=true
MISSING_EVIDENCE_NEVER_BECOMES_SUCCESS=true
```

Historical source, correctness tests, negative performance observations, and
architecture option eliminations must remain available. A future candidate
needs fresh exact-head correctness, package, host-admitted performance, CI, and
independent review before its admission state may change.

## Exit gate

```text
OWNER_FUTURE_RELEASE_CONTRACT=EXPLICIT
TRANSPARENT_SHIM_CORRECTNESS=PASS
TRANSPARENT_SHIM_PERFORMANCE=PASS
TRANSPARENT_SHIM_PRODUCTION_ADMISSION=OWNER_REVIEWED
HISTORICAL_G36_FAILURES_PRESERVED=true
```
