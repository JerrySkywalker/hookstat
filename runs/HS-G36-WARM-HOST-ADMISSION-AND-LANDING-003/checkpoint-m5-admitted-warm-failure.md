# M5 — admitted warm failure and final technical stop

The final bounded search executed for `1553.00` seconds at clean exact source
head `93a5a1e9ea47a5a093570ed05cdaace1acbe93ff`. The first two
build-comparability attempts were retained as `REJECTED_HOST_SUBSTRATE`. The
third comparison was admitted:

```text
BUILD_COMPARABILITY_PRE_P95_MS=16.6426
BUILD_COMPARABILITY_PRE_P99_MS=21.1159
SHIPPING_STARTUP_WORST_P99_MS=22.7592
INSTRUMENTED_STARTUP_WORST_P99_MS=29.4838
STARTUP_TAIL_BIAS_CORRECTION_MS=0.0000
BUILD_COMPARABILITY_POST_P95_MS=16.6146
BUILD_COMPARABILITY_POST_P99_MS=20.1441
BUILD_COMPARABILITY_DISPOSITION=ACCEPTED
```

The first warm product window then had both exact G28 controls pass:

```text
PRE_CONTROL_P95_MS=17.3319
PRE_CONTROL_P99_MS=21.3444
PRODUCT_P50_MS=18.2602
PRODUCT_P95_MS=24.2950
PRODUCT_P99_MS=26.5946
PRODUCT_MAX_MS=26.8934
POST_CONTROL_P95_MS=19.2518
POST_CONTROL_P99_MS=22.4338
CANDIDATE_HOOKSTAT_INDUCED_TIMEOUTS=0
CANDIDATE_UNEXPECTED_TERMINAL_RESULTS=0
CANDIDATE_ORACLE_OBSERVATION_GAPS=0
RAW_RECEIPT_OUTCOME=FAIL_FROZEN_BUDGET
RELEASE_ACCEPTANCE_CLASSIFICATION=FULL_ACCEPTANCE_FAIL
```

This is a real admitted product failure. It is not relabelled as host noise,
not excluded as a worst run, and not retried. The runner correctly stopped
before score-fishing for five passing populations.

The same receipt re-establishes the other performance paths:

```text
COOPERATIVE_WORST_P95_MS=0.2118
COOPERATIVE_WORST_P99_MS=0.4732
COOPERATIVE_OBSERVATION_GAPS=0
COLD_WORST_P95_MS=28.9614
HOOKSTAT_INDUCED_TIMEOUTS=0
UNEXPECTED_TERMINAL_RESULTS=0
ORACLE_OBSERVATION_GAPS=0
ORACLE_PRIMARY_RECORD_WORST_P95_MS=0.0433
ORACLE_PRIMARY_RECORD_WORST_P99_MS=0.0719
```

The admitted build correction is zero, so build comparability does not explain
the failure. The fixed oracle record cost is below `0.072 ms` at p99. Existing
stage evidence identifies repeated-fresh shipping startup as the primary floor,
but this admitted window adds no per-stage evidence isolating a new removable
source hotspot. The retained one-process optimizations are already frozen:
reusable producer, OS-backed wait, containment fast path, bounded capsule read,
exact pre-spawn deadline, wide Windows exit codes, and codegen-units 1. Thin
LTO, GUI subsystem, and helper architecture remain rejected or out of scope.

No credible evidence-backed final one-process source change remains inside this
train. Under the Owner's Phase 8 stop gate:

```text
FINAL_G36_SHIM_ARCHITECTURE=OPTIMIZED_ONE_PROCESS
WARM_ADMITTED_RUNS=0_OF_5_STOPPED_ON_FIRST_ADMITTED_FAILURE
ADMITTED_WARM_FAILURE_OCCURRED=true
G36_PERFORMANCE=FAIL_FROZEN_BUDGET
DISPOSITION=OWNER_BUDGET_POLICY_DECISION_REQUIRED
HELPER_ARCHITECTURE_SHIPPED=false
FROZEN_G28_BUDGET_CHANGED=false
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
G37_STARTED=false
G38_STARTED=false
PUBLICATION_AUTHORIZED=false
```

Package, fresh-install, exact-head CI, independent review, and merge gates are
not run because the mandatory performance gate failed first. Earlier
correctness regression and focused qualification validation remain retained;
they do not override this admitted performance failure.
