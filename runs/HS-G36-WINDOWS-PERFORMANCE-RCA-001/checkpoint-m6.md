# M6 — evidence-backed shim optimization

The first correctly release-profiled corrected-method receipt remains a full
acceptance failure: cooperative worst p95/p99 was `0.1939/0.2973` ms with zero
gaps, cold p95 was `36.6775` ms, and healthy induced timeouts were zero; warm
worst p95/p99 was `45.2967/240.0238` ms.

Release-profile attribution retained the process-startup hotspot and showed
the completed IPC connection reuse working: START p95 was `1.9981` ms and
COMPLETE p95 was `0.3120` ms. A safe containment fast path removed two
post-check canonicalization traversals after the existing canonical-parent and
non-reparse regular-file checks. The HMAC continues to protect the subsequent
read against a replacement race.

A bounded release code-generation A/B then compared the same 100-sample stage
probe after this source change. `codegen-units=1`, without the already-rejected
Thin LTO setting, reduced the shipping binary from `592384` to `550400` bytes,
shipping-startup p95 from `15.3049` to `14.3092` ms, and execution-total p95
from `24.4159` to `23.2149` ms. Its following complete receipt is still a full
acceptance failure (warm worst p95/p99 `62.0514/87.0825` ms), so it is retained
only as a measured candidate, not a performance qualification.

```text
SHIM_WARM_PRIMARY_HOTSPOT=SHIPPING_PROCESS_STARTUP
SHIM_WARM_SECONDARY_HOTSPOT=START_IPC_AND_CAPSULE_CONTAINMENT_VALIDATION
SHIM_OPTIMIZATION_1=REDUNDANT_CANONICALIZATION_REMOVED_AFTER_EQUIVALENT_SAFETY_PROOF
SHIM_OPTIMIZATION_2=RELEASE_CODEGEN_UNITS_1
THIN_LTO=REJECTED_BY_PRIOR_STAGE_AB
WINDOWS_GUI_SUBSYSTEM=REJECTED_BY_STAGE_AB
FROZEN_G28_BUDGET_CHANGED=false
```
