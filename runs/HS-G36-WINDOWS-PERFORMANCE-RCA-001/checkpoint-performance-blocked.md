# G36 performance checkpoint — blocked before landing gates

The frozen G28 limits are unchanged. Current full release-method receipts
prove the cooperative producer and cold shim path are within their respective
limits, but the transparent warm-shim p95/p99 remain over `20/25` ms after the
evidence-backed connection, containment-validation, OS-backed wait, and
release-codegen changes.

This is therefore not eligible for package/publish-dry-run/fresh-install,
exact-head CI, independent acceptance review, or merge. No G37 work began.

```text
PERFORMANCE_BUDGET=FAIL
COOPERATIVE_IPC=PASS_IN_CURRENT_RELEASE_RECEIPTS
TRANSPARENT_SHIM_WARM=FAIL
TRANSPARENT_SHIM_COLD=PASS_IN_CURRENT_RELEASE_RECEIPTS
HOOKSTAT_INDUCED_TIMEOUTS=0
FROZEN_G28_BUDGET_CHANGED=false
G37_STARTED=false
PUBLICATION_AUTHORIZED=false
```
