# First host-admission attempt — retained harness interruption

The first run on clean source head
`e9384651ac6a9b904ac3bbfddf671c9d8f4adf82` ran for `2325.35` seconds and
then stopped inside the first candidate series because the harness asserted
that every instrumented healthy invocation must exit zero.

No complete pre/product/post triple had been written. Therefore this attempt
has no host or product classification:

```text
RAW_RECEIPT_OUTCOME=NO_RECEIPT_HARNESS_INTERRUPTION
RELEASE_ACCEPTANCE_CLASSIFICATION=NOT_CLASSIFIED
PRE_CONTROL=COMPLETED_RESULT_NOT_EMITTED
POST_CONTROL=NOT_REACHED
WARM_ADMITTED_RUNS=0
REJECTED_HOST_SUBSTRATE=UNPROVEN
FAIL_FROZEN_BUDGET=UNPROVEN
```

The fixed oracle record had already been received, which proves
`run_capsule` returned a normal `ShimOutcome`; capsule, spawn, child-wait, and
containment setup errors exit before that record. For the fixed `cmd.exe`
exit-0 fixture, the only nonzero normal outcome is timeout exit `124`. The
immediate cause was therefore a healthy candidate timeout under the observed
fresh-process scheduling delay, followed by a harness assertion that prevented
the required post-control from running. The pre-control completed internally,
but its percentile result was never emitted and cannot be reconstructed; it is
not treated as admission evidence.

The harness now retains candidate timeout and unexpected-terminal counts,
always runs the post-control after a normal oracle record, and applies the
predefined precedence:

```text
controls PASS + candidate timeout = FAIL_FROZEN_BUDGET
either control FAIL + candidate timeout = REJECTED_HOST_SUBSTRATE
```

Rejected candidate timeouts remain in the immutable window receipt but do not
count as accepted zero-induced-timeout evidence. Timeouts or unexpected
terminal results in an admitted warm window, cold run, or healthy near-timeout
run fail the final product qualification.

The fast real-process regression first reproduced the old panic with exit 124,
then passed after the retention change. Seven pure admission tests also pass.
No product shim source, timeout budget, host setting, live configuration, or
frozen numeric limit changed.
