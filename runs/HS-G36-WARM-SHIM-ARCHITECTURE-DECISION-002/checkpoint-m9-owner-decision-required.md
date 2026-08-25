# M9 — architecture decision stop gate

The clean corrected one-process head retains a complete warm failure at
`32.6951/245.2032` ms p95/p99. The helper experiment was intentionally stopped
at its strict floor: a 179-KB fresh frontend plus one fixed local exchange was
already `325.5800/451.8487` ms p95/p99 over 500 cache-warmed samples in the
observed environment. Adding capsule, evidence, handler, timeout, and
containment semantics cannot reduce that population below its floor.

The architecture comparison is durable in
`docs/adr/HS-G36-TRANSPARENT-SHIM-WARM-ARCHITECTURE.md`. Option A remains the
only semantics-complete implementation but lacks reliable warm-tail margin.
Option B is additionally blocked by standard-handle and helper-death truth.
Option C is the safer future helper shape because the frontend retains child,
stream, timeout, and Job ownership, but its shared measured floor blocks it as
a current G36 candidate.

No full performance pass exists on the current exact candidate. CI and
independent acceptance cannot convert this technical stop gate into a pass, so
PR #33 must not be merged. G37 remains stopped.

```text
FINAL_G36_SHIM_ARCHITECTURE=UNRESOLVED_OWNER_DECISION_REQUIRED
OWNER_ARCHITECTURE_DECISION_REQUIRED=true
G36_PERFORMANCE=FAIL
HELPER_PROTOTYPE_IMPLEMENTED=PARTIAL_FLOOR_ONLY
HELPER_SEMANTIC_PROTOTYPE=NOT_IMPLEMENTED_FLOOR_FAILED
G36_MERGED=false
FROZEN_G28_BUDGET_CHANGED=false
NATIVE_ADMISSION_CHANGED=false
G37_STARTED=false
G38_STARTED=false
PUBLICATION_AUTHORIZED=false
```
