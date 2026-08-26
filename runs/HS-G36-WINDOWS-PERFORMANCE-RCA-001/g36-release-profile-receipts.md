# Retained corrected-method release receipts

Both receipts use `cargo test --release`, record `build_profile=release`, use
five 100-sample series per metric, warm each timed warm sample with 25 fresh
shipping `hookstat-hook --help` processes, and preserve signed alternating
transparent-minus-direct pairs. They are full acceptance failures, not
reduced diagnostics.

| Receipt | Candidate | Cooperative worst p50 / p95 / p99 (ms) | Gaps | Warm worst p50 / p95 / p99 (ms) | Cold worst p50 / p95 / p99 (ms) | Healthy induced timeouts | Classification |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| `001` | reusable producer + OS-backed child wait | 0.1261 / 0.1939 / 0.2973 | 0 | 28.0862 / 45.2967 / 240.0238 | 28.3708 / 36.6775 / 42.9859 | 0 | `FULL_ACCEPTANCE_FAIL` |
| `002` | `001` plus containment validation fast path and `codegen-units=1` | 0.1198 / 0.2798 / 0.7591 | 0 | 27.5013 / 62.0514 / 87.0825 | 27.9801 / 38.4506 / 48.4768 | 0 | `FULL_ACCEPTANCE_FAIL` |

Receipt `001` has a 5,028.9045 ms warm maximum in run 2 and receipt `002`
has a 62.0514 ms warm p95 in run 2. Neither is hidden or treated as a
selection opportunity. Every warm series in both receipts nevertheless
exceeds the frozen 20 ms p95 budget, so their failure conclusion does not
depend on either exceptional tail.

```text
RELEASE_ARTIFACTS=true
BUILD_PROFILE=release
WARM_HARNESS_SELF_LOAD=false
WARM_PAIRING=ALTERNATING_SIGNED_TRANSPARENT_MINUS_DIRECT
FROZEN_G28_BUDGET_CHANGED=false
HOOKSTAT_INDUCED_TIMEOUTS_FOR_HEALTHY_HOOK=0
```
