# M4 — first recalibrated-contract comparator invalidated

The first strict qualification attempt ran at tracked-clean exact source head
`540f0bef71b159ca2bb988f91884d5fbe363b577`. It built isolated ordinary and
feature-gated release artifacts with pinned Rust 1.97.1 and retained the atomic
comparator receipt before stopping.

```text
SHIPPING_BINARY_SIZE_BYTES=551424
SHIPPING_BINARY_SHA256=fd0369c991943352f023c7d42a4a1f2498a040b94a3248f174ed12d7380700f7
INSTRUMENTED_BINARY_SIZE_BYTES=559104
INSTRUMENTED_BINARY_SHA256=089fd1ab1c7f5ba334383bd4a8ce613b9234cd8bf7af03065c838ad5961563b8

PRE_CONTROL_P95_MS=19.6993
PRE_CONTROL_P99_MS=22.9814
SHIPPING_STARTUP_WORST_P99_MS=44.3570
INSTRUMENTED_STARTUP_WORST_P99_MS=30.2078
STARTUP_TAIL_BIAS_CORRECTION_MS=14.1492
POST_CONTROL_P95_MS=16.0558
POST_CONTROL_P99_MS=21.5450
COMPARATOR_DISPOSITION=INVALIDATED_BUILD_PROFILE
COMPARATOR_RECEIPT_SHA256=BF1C6DCABFAF537A450CC13E8E00AEF072E053A0BC3627EDE72A2FB9F56B0074
```

Both G28 host controls pass, so this is not a rejected host window. The fixed
build-comparability stop is `2.0 ms`; the instrumented oracle therefore cannot
represent the shipping product in this attempt. The runner stopped before any
warm product series, so this proves neither product pass nor product failure.
It is retained and is not counted among the five required admitted warm runs.

The five comparator populations have similar medians but asymmetric fresh-
process tails. No product source or threshold is changed in response. The next
safe action is one independent, low-frequency exact-build comparator attempt;
it must satisfy the same predefined stop before product qualification may
begin.

```text
WARM_HOST_REJECTED_WINDOWS=0
WARM_ADMITTED_RUNS=0
ADMITTED_RECALIBRATED_FAILURE_OCCURRED=false
PRODUCT_OBSERVATION_OCCURRED=false
FURTHER_AUTOMATIC_BUDGET_RELAXATION=false
HELPER_ARCHITECTURE_SHIPPED=false
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
G37_STARTED=false
PUBLICATION_AUTHORIZED=false
```
