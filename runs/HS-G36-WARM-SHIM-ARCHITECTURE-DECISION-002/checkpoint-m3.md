# M3 — one-process lower bound and architecture decision

The same-invocation oracle separates the actual original child spawn/wait
interval without subtracting a second process.  The current shipping shim's
cache-warmed help-path startup p95 is `12.0281` ms.  Because every
repeated-fresh transparent invocation must first start that executable, it is
the directly observed p95 floor.  Mandatory capsule, IPC, containment, and
exit work can only add to it; stage percentiles are not summed because doing so
would repeat the quantile error rejected at M1.

The raw observed overhead p95 is `16.3280` ms with the complete oracle channel
charged.  The instrumented startup p99 was `1.1668` ms faster than shipping, so
the conservative diagnostic uses `17.4948` ms.  That leaves `2.5052` ms p95
headroom.  The similarly corrected p99 is `17.8196` ms, leaving `7.1804` ms.

Current evidence-backed shipping improvements remain in force: reusable
acknowledged START/COMPLETE producer state, OS-backed child wait, equivalent
capsule-containment validation without duplicate canonicalization,
`codegen-units=1`, and producer reuse across START/COMPLETE.  Thin LTO and the
Windows GUI subsystem remain rejected by their retained A/B results.  The
current candidate already meets the preferred `18/23` diagnostic margin, so no
new profile or semantic optimization is justified before full qualification.

```text
ONE_PROCESS_SHIM_WARM_LOWER_BOUND_P95_MS=12.0281
ONE_PROCESS_SHIM_WARM_OBSERVED_OVERHEAD_P95_MS=16.3280
ONE_PROCESS_SHIM_WARM_CONSERVATIVE_P95_MS=17.4948
ONE_PROCESS_SHIM_WARM_CONSERVATIVE_P99_MS=17.8196
ONE_PROCESS_SHIM_WARM_HEADROOM_MS=2.5052
ONE_PROCESS_ARCHITECTURE=VIABLE
FINAL_G36_SHIM_ARCHITECTURE=ONE_PROCESS_OPTIMIZED
HELPER_PROTOTYPE_IMPLEMENTED=false
ARCHITECTURE_ADR=NOT_REQUIRED_ONE_PROCESS_RETAINED
FROZEN_G28_BUDGET_CHANGED=false
G37_STARTED=false
```
