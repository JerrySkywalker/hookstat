# HS-G36 v0.3.1 transparent-shim warm budget recalibration

## Status and authority

Owner-approved, one-time release-contract recalibration for HookStat v0.3.1.
This record is prospective for new G36 qualification evidence. It does not
edit, supersede, or relabel the G28 baseline receipt or any historical G36
receipt.

```text
G28_REFERENCE_WARM_P95_MS=20
G28_REFERENCE_WARM_P99_MS=25

V031_RELEASE_WARM_P95_MS=25
V031_RELEASE_WARM_P99_MS=30

HOST_ADMISSION_P95_MS=20
HOST_ADMISSION_P99_MS=25

FURTHER_AUTOMATIC_BUDGET_RELAXATION=false
```

The G28 `20/25`-ms values remain the reference performance target. The
v0.3.1 `25/30`-ms values are the release hard cap for the semantics-complete
transparent-shim product metric. The independent G28 minimal-shim pre/post
host controls remain `20/25` ms. A host-control value is never subtracted from,
added to, or otherwise used to correct the product result.

The cooperative IPC `1/2`-ms p95/p99 limits, transparent-shim cold `50`-ms p95
limit, and zero HookStat-induced timeout requirement are unchanged.

## Evidence basis

G28 calibrated its reference from the cache-warmed, fresh minimal-shim
process-start fixture before the shipping capsule, IPC, containment, and
finalization work existed:

```text
G28_MINIMAL_SHIM_CACHE_WARMED_P95_MS=14.82
G28_MINIMAL_SHIM_CACHE_WARMED_P99_MS=18.47
```

G36 produced the first strict host-admitted observation of the optimized,
semantics-complete shipping architecture:

```text
PRE_CONTROL_P95_MS=17.3319
PRE_CONTROL_P99_MS=21.3444
PRODUCT_P95_MS=24.2950
PRODUCT_P99_MS=26.5946
POST_CONTROL_P95_MS=19.2518
POST_CONTROL_P99_MS=22.4338
COOPERATIVE_WORST_P95_MS=0.2118
COOPERATIVE_WORST_P99_MS=0.4732
COLD_WORST_P95_MS=28.9614
HOOKSTAT_INDUCED_TIMEOUTS=0
OBSERVATION_GAPS=0
```

Both predefined host controls passed. The one-process architecture had already
exhausted the credible measured G36 optimizations, and the measured helper
floor plus larger semantic and privacy surface rejected a helper architecture
for v0.3.1. The Owner therefore selected an explicit shipping-evidence
recalibration rather than a post-hoc host reclassification or a new
architecture.

## Historical evidence preservation

The admitted `24.2950/26.5946`-ms receipt was created under the then-governing
G28 `20/25`-ms product contract. Its serialized outcome and evidence-index
classification remain historical truth:

```text
RAW_RECEIPT_OUTCOME=FAIL_FROZEN_BUDGET
RAW_HISTORICAL_OUTCOME=FAIL_FROZEN_G28_BUDGET
HISTORICAL_RELEASE_ACCEPTANCE_CLASSIFICATION=FULL_ACCEPTANCE_FAIL
```

No old `FAIL_FROZEN_BUDGET` receipt becomes a pass under this record. Only a
new exact-head qualification using this versioned contract can establish
v0.3.1 release acceptance.

## Prospective decision rule

Each warm product population is bracketed by the unchanged G28 cache-warmed
minimal-shim pre and post controls. Classification order is fixed before the
candidate is observed:

1. If either control exceeds `20/25` ms, retain the complete window as
   `REJECTED_HOST_SUBSTRATE`.
2. With both controls passing, if the product exceeds `25/30` ms, retain it as
   `FAIL_RECALIBRATED_BUDGET` and stop G36 product landing.
3. With both controls passing and product at or below `25/30` ms, classify it
   as `ADMITTED_PASS`.

Five independently admitted product passes are required. Rejected host
windows remain evidence and cannot be selected away. Candidate results cannot
change host admission, thresholds, or build-comparability admission. No
further automatic warm-budget relaxation is authorized.

## Terminal result and v0.3.1 scope disposition

Exact tracked-clean source `660142c5cba8ff3e3716d6dc04e43d1460d3ed6b`
obtained an admitted zero-bias comparator and then two host-admitted product
windows. The first passed at p95/p99 `14.6629/15.2830` ms. The second retained
passing pre controls `11.3245/13.9147` ms and post controls
`11.2306/20.1710` ms, but the product measured `23.0369/38.1377` ms. The p99
exceeded the one-time `30`-ms hard cap, so the immutable outcome is:

```text
RAW_OUTCOME=FAIL_RECALIBRATED_BUDGET
TRANSPARENT_SHIM_20_25_RESULT=FAIL
TRANSPARENT_SHIM_25_30_RESULT=FAIL
FURTHER_AUTOMATIC_BUDGET_RELAXATION=false
```

The Owner responded by reducing v0.3.1 release scope, not by reclassifying the
receipt or increasing a threshold. Cooperative IPC remains subject to its
unchanged `1/2`-ms p95/p99 contract. The transparent shim implementation and
correctness evidence remain retained, but its release state is
`QUALIFIED_NOT_ADMITTED_PERFORMANCE` and production activation is false.
Rearchitecture is deferred to G36T for v0.3.2 or later.

```text
WARM_RECALIBRATION_GOVERNANCE=OWNER_APPROVED
FINAL_G36_SHIM_ARCHITECTURE=OPTIMIZED_ONE_PROCESS
HELPER_ARCHITECTURE_SHIPPED=false
FROZEN_G28_HISTORICAL_EVIDENCE_CHANGED=false
NATIVE_ADMISSION_CHANGED=false
G37_STARTED=false
PUBLICATION_AUTHORIZED=false
```
