# M4 — cross-regime and interruption correction

The first host search proved two qualification-harness defects without changing
shipping product source:

1. the shipping/instrumented startup correction was measured before the host
   controls and transferred across eight rejected host regimes into a later
   passing-control product window;
2. a later cold invocation that exited before its oracle record panicked the
   harness instead of retaining the terminal class and observation gap.

The correction makes build comparability a separately retained qualification
prerequisite:

```text
PRE exact G28 control
five 100-sample shipping/instrumented startup populations
POST exact G28 control
```

The controls keep the unchanged `20/25 ms` limits. A control failure is
`REJECTED_HOST_SUBSTRATE`. When both controls pass, the pre-existing fixed
`2.0 ms` maximum faster-instrumented correction is applied. A result at or
above that stop is `INVALIDATED_BUILD_PROFILE` and terminates before product
qualification. Only `ACCEPTED` comparability supplies a correction to warm
windows. Every attempt is atomically bound to source head, artifact sizes, and
both artifact SHA-256 values.

The oracle receiver now returns a bounded missing-record observation when the
exact owned shim exits early, produces an incomplete record, or exceeds the
two-second diagnostic connection bound. The runner retains its exit class and
gap and still reaches the post-control. A failed host control keeps
`REJECTED_HOST_SUBSTRATE` precedence. With passing controls, an observed timeout
or unexpected terminal result remains `FAIL_FROZEN_BUDGET`; a gap with no
terminal failure is `INVALIDATED_BY_METHOD`. Neither path can become an
acceptance pass.

Validation on Rust 1.97.1:

```text
HOST_AND_COMPARABILITY_REDUCER_TESTS=PASS_10_OF_10
FOCUSED_QUALIFICATION_TESTS=PASS_19_OF_19_IGNORED_1
TIMEOUT_RETENTION_REAL_PROCESS=PASS
PRE_ORACLE_EXIT_RETENTION_REAL_PROCESS=PASS
FOCUSED_CLIPPY_DENY_WARNINGS=PASS
RELEASE_QUALIFICATION_HARNESS_NO_RUN_BUILD=PASS
FMT_CHECK=PASS
SHIPPING_PRODUCT_SOURCE_CHANGED=false
FROZEN_G28_BUDGET_CHANGED=false
```
