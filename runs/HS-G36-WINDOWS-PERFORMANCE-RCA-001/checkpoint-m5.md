# M5 — G28-traceable warm-method correction

The pre-RCA warm loop performed 25 complete shim/handler/IPC transactions
before every timed sample. That self-loaded the broker, WAL, original handler,
and filesystem and was not G28-equivalent.

The corrected qualification keeps the frozen sample plan (five independent
100-sample series and nearest-rank percentiles) and changes only warm-up:

```text
warm-up = 25 unmeasured fresh launches of the actual shipping hookstat-hook --help executable
timed operation = fresh balanced-order pair of transparent shim and direct handler
pair value = signed transparent duration minus its adjacent direct duration
```

The shipping executable is genuinely launched for every warm-up, while `--help`
exits before it can create broker/WAL/handler work. This matches G28's
operational cache-warmed fresh-start intent without redefining warm as a
persistent process or using a lucky preheated sample. Alternating direct-first
and shim-first order keeps the paired measurement real while avoiding a
one-way cache-order preference. Negative paired values remain negative; no
saturating subtraction distorts tail ranks.

The first two full corrected series were inadvertently invoked through the
debug test profile. Their timing structure is useful diagnostic evidence, but
their `release_artifacts=true` claim was not independently true and they are
therefore `INVALIDATED_BY_BUILD_PROFILE` for acceptance. They are retained,
including their cooperative and shim numbers, and are not rerun-selected.

The qualification test now refuses a debug-profile invocation before it can
write a receipt and records `build_profile=release`. A fresh release-profile
series is required before any candidate may be compared with the frozen G28
limits.

```text
WARM_HARNESS_SELF_LOAD=false
WARM_BENCHMARK_METHOD_VALID=REQUIRES_RELEASE_PROFILE_RECEIPT
WARM_BENCHMARK_METHOD_CHANGED=true
WARM_METHOD_G28_EQUIVALENCE=ACTUAL_SHIPPING_SHIM_FRESH_START_25_UNMEASURED_LAUNCHES
FROZEN_G28_BUDGET_CHANGED=false
```
