# M1-M3 — v0.3.1 warm recalibration governance and tooling

## Admission

```text
RUN_ID=HS-G36-WARM-BUDGET-RECALIBRATION-LANDING-004
START_MAIN=e67972d027582b18a0c48705084e00127fc693ce
G36_START_HEAD=c996101b626c2ece73f7ceb0a4c236f5f6cc6fb0
PR=33
PR_BASE=main
PR_STATE=OPEN_DRAFT
WORKTREE_TRACKED_STATE_AT_ADMISSION=CLEAN
REPOSITORY_LOCKS=NONE
```

Remote main and the PR base matched the Owner-provided main SHA after fetch.
The local and remote G36 branch matched the Owner-provided G36 SHA. Unrelated
worktrees and their contents were not changed.

## M1 — Owner policy and governance

The versioned record
`docs/performance/HS-G36-WARM-BUDGET-RECALIBRATION.md` separates three
contracts:

```text
G28_REFERENCE_WARM_P95_P99_MS=20/25
V031_RELEASE_WARM_P95_P99_MS=25/30
HOST_ADMISSION_P95_P99_MS=20/25
COOPERATIVE_P95_P99_MS=1/2
COLD_P95_MS=50
HEALTHY_HOOK_INDUCED_TIMEOUTS=0
FURTHER_AUTOMATIC_BUDGET_RELAXATION=false
```

The historical G28 budget document was not edited. Its SHA-256 remains
`AFD03A4B10350BDB8071C6204BF9FC8260E416550A6B2862365E1C513FB729A3` at this
checkpoint. The historical admitted G36 receipt was not edited; its SHA-256
remains
`E77B805E6DC2799221965AB5594D334ACCC2F858A251FC6C37B3F67F4ED727DF` and its
serialized outcome remains `FAIL_FROZEN_BUDGET`.

## M2 — qualification tooling

The developer-only policy now uses the v0.3.1 product limits `25/30` while
retaining independent host constants `20/25`, G28 reference constants
`20/25`, and the fixed `2.0`-ms build-comparability stop. New product failures
serialize as `FAIL_RECALIBRATED_BUDGET`. Schema-v2 product-window and final
receipts record all three numeric contracts and
`further_automatic_budget_relaxation=false`.

Ordinary `cargo test --all-targets --locked` previously failed on Windows
because the Windows-only qualification integration test imported a module
compiled only by `performance-harness`. The integration target is now itself
gated by both Windows and `performance-harness`. No developer performance
binary or module was added to the ordinary build.

## M3 — deterministic regression proof

```text
CONTROL_19_24_PRODUCT_24_29=ADMITTED_PASS
CONTROL_19_24_PRODUCT_26_29=FAIL_RECALIBRATED_BUDGET
CONTROL_19_24_PRODUCT_24_31=FAIL_RECALIBRATED_BUDGET
CONTROL_21_24=REJECTED_HOST_SUBSTRATE
CONTROL_19_26=REJECTED_HOST_SUBSTRATE
CANDIDATE_INFLUENCES_CONTROL_ADMISSION=false
```

Validation on pinned Rust 1.97.1:

```text
CARGO_FMT=PASS
HOST_ADMISSION_UNIT_TESTS=11_PASS_0_FAIL
QUALIFICATION_FOCUSED_TESTS=19_PASS_0_FAIL_1_IGNORED
ORDINARY_ALL_TARGETS_TESTS=PASS
FOCUSED_CLIPPY_DENY_WARNINGS=PASS
GIT_DIFF_CHECK=PASS
```

No performance acceptance claim is made by this checkpoint. The production
shim architecture and source remain frozen; the next gate is a tracked-clean,
exact-head, strict five-run host-admitted qualification.

```text
FINAL_G36_SHIM_ARCHITECTURE=OPTIMIZED_ONE_PROCESS
HELPER_ARCHITECTURE_SHIPPED=false
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
NATIVE_ADMISSION_CHANGED=false
G37_STARTED=false
PUBLICATION_AUTHORIZED=false
```
