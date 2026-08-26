# M5 — admitted recalibrated-budget failure

Qualification is stopped on the first admitted product failure under the
Owner-approved v0.3.1 `25/30`-ms release hard cap.

## Exact candidate and artifacts

```text
SOURCE_GIT_HEAD=660142c5cba8ff3e3716d6dc04e43d1460d3ed6b
SOURCE_TRACKED_WORKTREE_CLEAN=true
SHIPPING_BINARY_SIZE_BYTES=551424
SHIPPING_BINARY_SHA256=fd0369c991943352f023c7d42a4a1f2498a040b94a3248f174ed12d7380700f7
INSTRUMENTED_BINARY_SIZE_BYTES=559104
INSTRUMENTED_BINARY_SHA256=089fd1ab1c7f5ba334383bd4a8ce613b9234cd8bf7af03065c838ad5961563b8
STARTUP_TAIL_BIAS_CORRECTION_MS=0.0000
```

Five complete comparator attempts at this head were retained as
`REJECTED_HOST_SUBSTRATE`. Two later independent comparator sessions were
retained as `INVALIDATED_BUILD_PROFILE`; their build corrections exceeded the
fixed `2.0`-ms stop and neither executed a product series. One interrupted
attempt produced no complete receipt and is not classified or counted.

The final comparator passed all fixed gates:

```text
COMPARATOR_PRE_CONTROL_P95_MS=10.2415
COMPARATOR_PRE_CONTROL_P99_MS=10.5836
COMPARATOR_POST_CONTROL_P95_MS=10.4497
COMPARATOR_POST_CONTROL_P99_MS=10.6765
COMPARATOR_DISPOSITION=ACCEPTED
```

## Admitted product windows

The first independently admitted product window passed:

```text
PRODUCT_WINDOW=1
PRE_CONTROL_P95_MS=10.4308
PRE_CONTROL_P99_MS=11.2296
PRODUCT_P95_MS=14.6629
PRODUCT_P99_MS=15.2830
POST_CONTROL_P95_MS=11.6782
POST_CONTROL_P99_MS=14.9489
PRODUCT_DISPOSITION=ADMITTED_PASS
```

The second independently admitted product window failed the recalibrated p99
hard cap:

```text
PRODUCT_WINDOW=2
PRE_CONTROL_P95_MS=11.3245
PRE_CONTROL_P99_MS=13.9147
PRODUCT_P95_MS=23.0369
PRODUCT_P99_MS=38.1377
POST_CONTROL_P95_MS=11.2306
POST_CONTROL_P99_MS=20.1710
PRODUCT_DISPOSITION=FAIL_RECALIBRATED_BUDGET
```

Both controls pass independently. No control value is subtracted from or used
to reclassify the product. The product p95 passes `25` ms, but product p99
exceeds the one-time `30`-ms hard cap. This is a genuine admitted product
failure, not a rejected host window. No later product window is sought and no
further budget relaxation or architecture restart is authorized.

## Retained supporting observations

The failure receipt also records:

```text
COOPERATIVE_WORST_P95_MS=0.1088
COOPERATIVE_WORST_P99_MS=0.1820
WARM_WORST_P95_MS=23.0369
WARM_WORST_P99_MS=38.1377
COLD_WORST_P95_MS=18.3481
HOOKSTAT_INDUCED_TIMEOUTS=0
UNEXPECTED_TERMINAL_RESULTS=0
ADMITTED_WARM_ORACLE_OBSERVATION_GAPS=0
COLD_ORACLE_OBSERVATION_GAPS=1
```

The admitted warm failure is independently decisive. The cold percentile is
retained but the cold series has one oracle observation gap and is not promoted
to final cold acceptance.

```text
WARM_HOST_REJECTED_WINDOWS=5
WARM_ADMITTED_RUNS=1_OF_5_STOPPED_ON_SECOND_ADMITTED_WINDOW
ADMITTED_RECALIBRATED_FAILURE_OCCURRED=true
G36_PERFORMANCE=FAIL_RECALIBRATED_BUDGET
FURTHER_AUTOMATIC_BUDGET_RELAXATION=false
HELPER_ARCHITECTURE_SHIPPED=false
QUALITY_TRAIN=NOT_RUN_PERFORMANCE_STOP
EXACT_HEAD_CI=NOT_RUN_PERFORMANCE_STOP
INDEPENDENT_REVIEW=NOT_RUN_PERFORMANCE_STOP
G36_MERGED=false
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
G37_STARTED=false
PUBLICATION_AUTHORIZED=false
```
