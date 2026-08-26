# M2 — deterministic G28 host-control envelope

The G36 release qualification now uses the exact existing G28
`hookstat-hook-fixture` executable. The fixture source and the G28 measurement
implementation are unchanged from `origin/main`; their last source commit is
`c37048fe4fc7ea3c5461ee1bdf18efee227d47d9`, and the fixture source SHA-256 is
`f2246013c61f0ae4266bdf97db4671cb9885ce3a2970cef0929c60ba8e5474d1`.

Each attempted warm window executes:

```text
100-sample PRE control
100-sample same-invocation product series
100-sample POST control
```

The feature-gated oracle also requires an exact shipping-versus-instrumented
startup comparison. Before any comparison can supply a correction, the runner
executes and atomically retains a separate envelope:

```text
100-sample PRE control
5 x 100-sample shipping/instrumented startup comparison
100-sample POST control
```

Failed controls retain `REJECTED_HOST_SUBSTRATE` and retry at the same bounded
low frequency. Passing controls admit the pre-existing fixed `2.0 ms`
build-comparability stop. A correction at or above that value is
`INVALIDATED_BUILD_PROFILE` and stops before product qualification; it is not
converted into a product pass or failure.

Every timed control sample receives 25 unmeasured fresh minimal-fixture
launches. The release-profile harness uses Rust's `Instant` and nearest-rank
p50/p95/p99/max. Thresholds are compile-time constants: p95 `20 ms` and p99
`25 ms` for both control phases and the product.

Each complete triple is written once through a synchronized staging file and
an atomic rename before classification can trigger a retry. It contains only
the methodology identifier, sample counts, sanitized statistics, fixed limits,
artifact/source identity, disposition, and privacy booleans. It contains no
hostname, processor model, path, command, capsule content, prompt, stream,
credential, or Owner content.

Rejected windows are retained and wait 60 seconds by default before a new
attempt. The bounded attempt cap defaults to 25 and may be configured between
5 and 100 without changing any threshold. An admitted product failure stops
the run immediately; it is not retried or reclassified.

```text
HOST_ADMISSION_IMPLEMENTED=true
HOST_CONTROL_FIXTURE=G28_CACHE_WARMED_MINIMAL_SHIM_PROCESS_START
HOST_CONTROL_SAMPLES_PER_PHASE=100
HOST_CONTROL_WARMUPS_PER_TIMED_SAMPLE=25
HOST_CONTROL_PERCENTILE_METHOD=nearest_rank
HOST_CONTROL_P95_LIMIT_MS=20
HOST_CONTROL_P99_LIMIT_MS=25
HOST_CONTROL_SUBTRACTED_FROM_PRODUCT=false
REJECTED_WINDOW_RECEIPT_ATOMIC=true
BUILD_COMPARABILITY_HOST_ADMITTED=true
BUILD_COMPARABILITY_MAX_BIAS_MS=2
PRE_ORACLE_EXIT_RETAINED=true
HELPER_ARCHITECTURE_SHIPPED=false
```
