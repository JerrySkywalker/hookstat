# M3 — host-control classification regression

The policy reducer is isolated in the feature-gated
`g36_host_admission` module. It uses only the frozen constants and the three
observed p95/p99 pairs. It cannot derive a threshold from a candidate result.

Deterministic cases:

```text
control PASS + product PASS = ADMITTED_PASS
control PASS + product FAIL = FAIL_FROZEN_BUDGET
pre control FAIL = REJECTED_HOST_SUBSTRATE
post control FAIL = REJECTED_HOST_SUBSTRATE
control FAIL + product FAIL = REJECTED_HOST_SUBSTRATE
```

The exact threshold is inclusive. A candidate failure is never hidden when
both controls pass. A failed predefined control rejects the entire window, and
the candidate observation remains in the immutable window receipt without
counting toward the five required admitted passes.

Validation completed on the implementation worktree with Rust 1.97.1:

```text
HOST_ADMISSION_UNIT_TESTS=PASS_7_OF_7
TIMEOUT_RETENTION_REGRESSION=PASS
RELEASE_QUALIFICATION_HARNESS_NO_RUN_BUILD=PASS
FOCUSED_CLIPPY_DENY_WARNINGS=PASS
FMT_CHECK=PASS
```

The no-run release build proved that the Windows qualification executable, the
exact G28 fixture, the instrumented one-process shim, and the receipt schema
link together. It did not create or claim a performance observation.
