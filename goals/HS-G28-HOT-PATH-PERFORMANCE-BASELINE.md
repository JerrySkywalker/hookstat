# HS-G28 — Hot Path Performance Baseline

## Status

PLANNED after public v0.3.0 baseline `a33a3be56982c6ca00699019883a047a1aca748b` and accepted v0.3.1 master plan.

## Objective

Establish a reproducible Windows-first performance laboratory for HookStat's current and proposed Hook evidence paths before changing the production architecture.

This Goal answers **where the v0.3 hot-path time actually goes** and freezes the numeric performance budget that governs G35–G38R.

## Safety boundary

- Use disposable fixtures/config roots for destructive or mutating Hook experiments.
- Do not mutate the Owner's live `~/.codex` Hook declarations merely to obtain benchmark numbers.
- No production architecture migration occurs in G28.
- No timeout is increased to make a benchmark pass.
- Benchmarking must not require prompt/tool/stdout/stderr persistence.

## Required benchmark decomposition

Measure independently on the Owner Windows environment where applicable:

```text
A. direct original executable/fixture
B. current v0.3.0 `hookstat.exe codex proxy`
C. HookStat executable startup only
D. `cmd.exe /C` process/shell overhead
E. direct CreateProcess/Rust Command spawn overhead
F. Windows Job Object create/assign/release overhead
G. current ReceiptStart write
H. current ReceiptCompletion write
I. current journal append
J. current `sync_data()` durability cost
K. Windows Named Pipe cold connection
L. Windows Named Pipe warm connection
M. one-way bounded frame write
N. bounded write + acknowledgement round-trip
O. minimal-shim process-start fixture
```

On non-Windows CI, add corresponding Unix process/UDS fixtures only where they improve deterministic regression coverage. Windows real evidence is the release-relevant baseline.

## Measurement rules

For meaningful paths record:

```text
warm p50 / p95 / p99
cold p50 / p95 / p99
sample count
machine/runtime/toolchain identification
```

At minimum exercise representative scales of 100, 1,000, and 10,000 iterations/events where practical. Avoid letting benchmark harness setup dominate the measured interval.

Clearly separate:

```text
original handler time
HookStat incremental overhead
transport time
persistence/durability time
process/shell time
```

## Current production evidence to reproduce

Use disposable qualification fixtures to model the Owner-observed failure class:

```text
Codex declaration timeout: 1 second
current HookStat v0.3 instrumentation enabled
start evidence emitted
proxy path exceeds outer deadline
completion missing
result becomes truthful Incomplete
```

This reproduction must not rely on the Owner's current live broken Hook configuration.

## Benchmark tooling

Prefer deterministic Rust benchmark/test harnesses that can remain in the repository and be rerun. External scripts are acceptable for real Windows orchestration when they record sanitized machine-readable results.

Do not introduce a benchmark dependency that becomes part of the production hot path.

## Performance budget freeze

The following are provisional design targets only until G28 closes:

```text
NATIVE_ADDED_SYNCHRONOUS_LATENCY_MS=0
COOPERATIVE_IPC_P95_MS<=1
COOPERATIVE_IPC_P99_MS<=2
TRANSPARENT_SHIM_WARM_P95_MS<=15
TRANSPARENT_SHIM_WARM_P99_MS<=25
TRANSPARENT_SHIM_COLD_P95_MS<=50
HOOKSTAT_INDUCED_TIMEOUTS_FOR_HEALTHY_HOOK=0
```

G28 may adjust these numeric thresholds **once**, based on real measured Windows constraints. The final values and rationale must be committed. Later Goals may improve them but may not silently weaken them.

## Required artifacts

Commit at least:

- a performance methodology document;
- machine-readable sanitized benchmark receipt(s);
- deterministic benchmark/fixture code suitable for regression use;
- the frozen v0.3.1 performance budget;
- a concise cost decomposition identifying the dominant v0.3 hot-path contributors.

## Required tests

- benchmark harness measures the intended region rather than fixture setup;
- warm/cold classification is explicit;
- no raw Hook payload is recorded;
- timeout reproduction preserves truthful incomplete semantics;
- results remain parseable and bounded;
- Windows-specific timing harness fails clearly when prerequisites are unavailable rather than fabricating a pass.

## Risk vector

```text
CODE_CHANGED=true
ARCHITECTURE_CHANGED=false
PERSISTENCE_CHANGED=false
CODEX_INTEGRATION_CHANGED=false
USER_LIVE_CONFIG_MUTATION=false
SECURITY_OR_PRIVACY_CHANGED=limited
RELEASE_BOUNDARY=false
```

## Acceptance

```text
OWNER_WINDOWS_BASELINE=PASS
CURRENT_PROXY_P50_P95_P99=RECORDED
PROCESS_SPAWN_COST=RECORDED
SHELL_COST=RECORDED
JOB_OBJECT_COST=RECORDED
RECEIPT_IO_COST=RECORDED
SYNC_DATA_COST=RECORDED
NAMED_PIPE_COST=RECORDED
MINIMAL_SHIM_FIXTURE_COST=RECORDED

ONE_SECOND_FAILURE_CLASS_REPRODUCED=PASS
LIVE_OWNER_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false

DOMINANT_COSTS_IDENTIFIED=true
PERFORMANCE_BUDGET_FROZEN=true
NO_UNMEASURED_PERFORMANCE_CLAIMS=true
CODE_CI=PASS
```

## Stop gate

If local IPC cannot demonstrate a credible low-latency path on the Owner Windows environment, stop before IPC implementation and revise the transport design. Do not proceed because the planned architecture expects Named Pipes to be fast.

## Estimated effort

**5–8 effective engineering hours.**

## Next

`HS-G29 — Runtime-Neutral Evidence Core`.
