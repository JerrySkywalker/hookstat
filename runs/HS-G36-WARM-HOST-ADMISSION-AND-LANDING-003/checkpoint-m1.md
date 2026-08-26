# M1 — Owner architecture and host-admission policy

Exact starting admission:

```text
RUN_ID=HS-G36-WARM-HOST-ADMISSION-AND-LANDING-003
START_MAIN=e67972d027582b18a0c48705084e00127fc693ce
G36_START_HEAD=134ea566a69c80bbd9e57c9d10216f770a9fa399
PR33_BASE=e67972d027582b18a0c48705084e00127fc693ce
PR33_HEAD=134ea566a69c80bbd9e57c9d10216f770a9fa399
G36_WORKTREE_TRACKED_CLEAN=true
GIT_LOCK_COUNT=0
```

The Owner selected the optimized repeated-fresh one-process shim for v0.3.1.
The helper architecture is stopped; Option C remains documentation-only and is
not a landing dependency.

Warm qualification now uses the exact G28 `hookstat-hook-fixture` control. In
the same Rust 1.97.1 release session, each 100-sample candidate series is
bracketed by pre and post 100-sample minimal-shim process-start controls. Every
timed control launch receives 25 unmeasured fresh fixture launches. All
percentiles use nearest rank.

Both controls must independently pass p95 `20 ms` and p99 `25 ms`. A failed
control retains the complete window as `REJECTED_HOST_SUBSTRATE`; it is neither
a product pass nor a product failure. If both controls pass, the unadjusted
same-invocation product metric must independently pass the unchanged
`20/25`-ms product limits. No dynamic or post-hoc threshold and no
host-control subtraction are allowed.

Historical receipts are immutable. The exact `08f83bc...` receipt retains
`RAW_RECEIPT_OUTCOME=FAIL`. It did not execute the predefined G28 pre/post
control, so `NON_ADMITTED_HOST_SUBSTRATE` is not proven and is not applied
retroactively.

```text
FINAL_G36_SHIM_ARCHITECTURE=OPTIMIZED_ONE_PROCESS
HOST_CONTROL_FIXTURE=G28_CACHE_WARMED_MINIMAL_SHIM_PROCESS_START
HOST_CONTROL_P95_LIMIT_MS=20
HOST_CONTROL_P99_LIMIT_MS=25
WARM_ADMITTED_RUNS_REQUIRED=5
HELPER_ARCHITECTURE_SHIPPED=false
FROZEN_G28_BUDGET_CHANGED=false
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
G37_STARTED=false
G38_STARTED=false
PUBLICATION_AUTHORIZED=false
```
