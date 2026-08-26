# M4 — first complete host-admission search

The corrected runner executed for `7534.23` seconds at clean exact source head
`1012a812695e198f72707e6c933e8e9a4dc94089`. It atomically retained nine
complete pre/product/post windows before a later cold-series invocation exited
before emitting its developer oracle record. No aggregate final receipt was
written, and the cold interruption has no warm-window disposition.

The immutable raw window dispositions are:

| Attempt | Pre p95/p99 ms | Product p95/p99 ms | Post p95/p99 ms | Timeouts | Raw disposition |
| ---: | ---: | ---: | ---: | ---: | --- |
| 1 | 87.3308 / 105.1273 | 125.2412 / 132.7653 | 86.3619 / 93.8882 | 1 | `REJECTED_HOST_SUBSTRATE` |
| 2 | 90.8743 / 95.5982 | 125.3479 / 131.5828 | 90.7967 / 99.1579 | 0 | `REJECTED_HOST_SUBSTRATE` |
| 3 | 85.7038 / 94.2864 | 133.0991 / 137.6578 | 93.1547 / 109.2550 | 0 | `REJECTED_HOST_SUBSTRATE` |
| 4 | 91.0508 / 107.9535 | 129.9640 / 145.5000 | 93.5785 / 112.4884 | 0 | `REJECTED_HOST_SUBSTRATE` |
| 5 | 88.6135 / 105.5318 | 129.1626 / 145.2986 | 84.6076 / 89.3637 | 0 | `REJECTED_HOST_SUBSTRATE` |
| 6 | 84.2024 / 90.7297 | 130.1016 / 134.1052 | 91.6593 / 100.1778 | 0 | `REJECTED_HOST_SUBSTRATE` |
| 7 | 83.2173 / 87.2175 | 138.5877 / 157.7767 | 87.5102 / 100.5805 | 0 | `REJECTED_HOST_SUBSTRATE` |
| 8 | 95.4809 / 114.3171 | 134.9566 / 138.4436 | 97.5961 / 174.8974 | 0 | `REJECTED_HOST_SUBSTRATE` |
| 9 | 12.0442 / 14.6764 | 27.5223 / 30.9591 | 13.3663 / 15.1660 | 0 | `FAIL_FROZEN_BUDGET` |

Attempt 9 is not relabelled as host noise: both predefined controls pass. Its
raw same-invocation product oracle is `17.5643/21.0011 ms`, but the runner adds
a `9.9580 ms` shipping-versus-instrumented startup correction that was measured
before these windows, while attempts 1 through 8 prove the process substrate
was outside admission. The correction exceeds the pre-existing `2.0 ms`
build-comparability stop encoded before this run. Therefore the session-wide
release classification is:

```text
RAW_RECEIPT_OUTCOME_ATTEMPT_009=FAIL_FROZEN_BUDGET
RELEASE_ACCEPTANCE_CLASSIFICATION=INVALIDATED_BY_BUILD_PROFILE
NON_ADMITTED_HOST_SUBSTRATE_ATTEMPTS=8
WARM_ADMITTED_RUNS=0
ADMITTED_WARM_FAILURE_OCCURRED=UNPROVEN_INVALID_PRODUCT_BUILD_COMPARABILITY
FROZEN_G28_BUDGET_CHANGED=false
```

This is not a post-hoc threshold or control subtraction. The `2.0 ms` stop was
already present in the harness and prior checkpoint before measurement. The
implementation defect was applying the invalid cross-regime correction to a
later host-admitted window and allowing the raw host-window reducer to label it
as a shipping-product failure. The next runner revision must admit the build
comparison under its own predefined pre/post host controls, retain that result
atomically, and stop before product qualification if an admitted comparison
exceeds the unchanged comparability limit.

The final interruption occurred after the warm loop had already stopped at
attempt 9. It does not erase or modify any warm receipt. It proves that a
pre-oracle child exit also needs a retained observation path before another
long qualification can safely run.
