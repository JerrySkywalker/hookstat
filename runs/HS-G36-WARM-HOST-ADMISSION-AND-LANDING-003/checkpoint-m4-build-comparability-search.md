# M4 — host-admitted build-comparability search

The corrected qualification runner executed for `13651.85` seconds at clean
exact source head `36173fd14a7993e80a608a45c7eeb47d31fb7c97`. It atomically
retained ten build-comparability attempts. Every attempt used the exact G28
pre/post control around five complete 100-sample shipping/instrumented startup
populations. All artifacts are size- and SHA-256-bound in every receipt.

No attempt had both controls pass:

| Attempt | Pre p95/p99 ms | Shipping / instrumented worst p99 ms | Bias ms | Post p95/p99 ms | Raw disposition |
| ---: | ---: | ---: | ---: | ---: | --- |
| 1 | 61.5707 / 372.0373 | 662.4734 / 592.7103 | 69.7631 | 15.4757 / 43.7981 | `REJECTED_HOST_SUBSTRATE` |
| 2 | 11.6231 / 11.9577 | 336.5887 / 571.3629 | 0.0000 | 92.0056 / 233.2150 | `REJECTED_HOST_SUBSTRATE` |
| 3 | 97.9935 / 111.1472 | 132.0119 / 150.6920 | 0.0000 | 97.4525 / 105.8610 | `REJECTED_HOST_SUBSTRATE` |
| 4 | 91.4121 / 118.1481 | 420.3692 / 301.6430 | 118.7262 | 99.4561 / 103.4683 | `REJECTED_HOST_SUBSTRATE` |
| 5 | 101.6573 / 108.3932 | 371.1162 / 301.1669 | 69.9493 | 138.6229 / 203.6976 | `REJECTED_HOST_SUBSTRATE` |
| 6 | 115.5852 / 163.2546 | 57.1885 / 54.4087 | 2.7798 | 14.5914 / 16.1705 | `REJECTED_HOST_SUBSTRATE` |
| 7 | 13.2204 / 19.4957 | 57.8202 / 151.5322 | 0.0000 | 23.5268 / 27.1309 | `REJECTED_HOST_SUBSTRATE` |
| 8 | 16.1340 / 27.2230 | 28.2450 / 25.6483 | 2.5967 | 18.1699 / 22.8683 | `REJECTED_HOST_SUBSTRATE` |
| 9 | 17.5141 / 20.7814 | 28.9293 / 44.3883 | 0.0000 | 21.1183 / 22.0440 | `REJECTED_HOST_SUBSTRATE` |
| 10 | 20.2685 / 21.6944 | 23.8867 / 33.3796 | 0.0000 | 20.2029 / 28.3820 | `REJECTED_HOST_SUBSTRATE` |

The final four attempts approached the gate, but no worst-run or phase is
excluded. The runner correctly refused to admit a correction or start warm
product qualification.

```text
RAW_BUILD_COMPARABILITY_ATTEMPTS=10
BUILD_COMPARABILITY_ADMITTED=0
BUILD_COMPARABILITY_REJECTED_HOST_SUBSTRATE=10
WARM_PRODUCT_SERIES_STARTED=false
WARM_ADMITTED_RUNS=0
ADMITTED_WARM_FAILURE_OCCURRED=false
RELEASE_ACCEPTANCE_CLASSIFICATION=NON_ADMITTED_HOST_SUBSTRATE
FROZEN_G28_BUDGET_CHANGED=false
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
```
