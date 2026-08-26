# M1 — warm metric and identifiability audit

Exact admission:

```text
RUN_ID=HS-G36-WARM-SHIM-ARCHITECTURE-DECISION-002
START_MAIN=e67972d027582b18a0c48705084e00127fc693ce
G36_START_HEAD=f2db93cc3562b2d115cc2ad4d297265cf49bbdae
PR33_BASE=e67972d027582b18a0c48705084e00127fc693ce
PR33_HEAD=f2db93cc3562b2d115cc2ad4d297265cf49bbdae
WORKTREE_TRACKED_CLEAN=true
```

The retained release receipts record current continuity evidence of cooperative
p95/p99 `0.2798/0.7591` ms with zero gaps, cold p95 `38.4506` ms, and zero
HookStat-induced healthy-hook timeouts.  Their historical receipt outcome is
preserved.  Their alternating paired warm tail of `62.0514/87.0825` ms is now
classified `INVALIDATED_BY_METHOD`, not deleted.

Adjacent signed pairs remain two distinct process lifetimes.  Alternation
balances order and adjacency reduces slow drift, but the delta still contains
the difference between independent child-process scheduling terms.  Tail
quantiles of that delta cannot identify the tail of HookStat-only overhead.

The frozen 20/25-ms warm contract governs HookStat-added transparent overhead.
The selected candidate metric subtracts the original child spawn/wait interval
measured inside one real transparent invocation from the full shim lifetime
observed by its parent.  It remains diagnostic until the side channel and
instrumented-build startup effect are bounded.

```text
COOPERATIVE_FINAL_STATUS=PASS_CURRENT_RELEASE_CONTINUITY
COLD_SHIM_FINAL_STATUS=PASS_CURRENT_RELEASE_CONTINUITY
WARM_SHIM_FINAL_STATUS=UNPROVEN_PRIOR_METHOD_INVALIDATED
ZERO_INDUCED_TIMEOUT_STATUS=PASS_CURRENT_RELEASE_CONTINUITY
WARM_ACCEPTANCE_METRIC=OTHER_PROVEN_METRIC
OTHER_PROVEN_METRIC=SAME_INVOCATION_PARENT_LIFETIME_MINUS_CHILD_SPAWN_WAIT
PAIRED_METHOD_IDENTIFIABLE=false
PAIRED_INCREMENTAL_STATUS=PAIRED_INCREMENTAL_NOT_IDENTIFIABLE
FROZEN_G28_BUDGET_CHANGED=false
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
G37_STARTED=false
PUBLICATION_AUTHORIZED=false
```
