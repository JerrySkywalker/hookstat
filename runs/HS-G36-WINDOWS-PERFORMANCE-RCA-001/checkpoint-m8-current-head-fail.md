# M8 — exact-head one-process qualification failure

The qualifying release series at clean source head
`08f83bcc79c09ac6e2bd27939ab435bc4c812890` is retained in
`g36-full-acceptance-08f83bc-fail.json`. Both the ordinary shipping artifact
and the feature-gated oracle artifact are SHA-256-bound in that receipt.

Cooperative IPC, cold shim, and timeout truth remain within the frozen G28
limits. Warm same-invocation transparent overhead does not:

```text
COOPERATIVE_WORST_P95_MS=0.1357
COOPERATIVE_WORST_P99_MS=0.2187
COLD_SHIM_WORST_P95_MS=20.5353
HOOKSTAT_INDUCED_TIMEOUTS_FOR_HEALTHY_HOOK=0
WARM_SHIM_WORST_P95_MS=32.6951
WARM_SHIM_WORST_P99_MS=245.2032
```

Four warm populations had p95 values from `18.9488` through `20.0188` ms;
their p99 values ranged from `21.0158` through `33.7476` ms. The fifth
population retained a `32.6951/245.2032` ms p95/p99 tail. The same period also
produced large fresh-process startup tails in both ordinary and instrumented
binaries. These observations are not removed as machine noise: scheduler time
outside the same-invocation original-child interval is operationally charged
to HookStat by the accepted metric.

The prior exact-candidate pass establishes that the one-process architecture
can pass, but this exact-head failure and the absence of the preferred
`18/23`-ms engineering margin establish that it cannot yet pass reliably.
Another identical run would be score selection, not an architecture fix.

```text
ONE_PROCESS_ARCHITECTURE=MARGINAL
ONE_PROCESS_ACCEPTANCE_MARGIN=INSUFFICIENT
FULL_ACCEPTANCE_RESULT=FAIL
HELPER_PROTOTYPE_JUSTIFIED=true
FROZEN_G28_BUDGET_CHANGED=false
```
