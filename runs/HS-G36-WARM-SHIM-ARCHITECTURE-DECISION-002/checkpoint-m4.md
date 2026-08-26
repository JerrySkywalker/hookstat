# M4 — one-process acceptance candidate

The complete release qualification sampled five independent 100-sample
populations for cooperative, warm transparent, and cold transparent paths.
Its first reducer selected the maximum signed difference from one adjacent
shipping/instrumented startup run pair.  That reintroduced the independent-
scheduling identifiability defect established at M1, so the original FAIL
receipt is preserved and classified `INVALIDATED_BY_METHOD`.

The corrected reducer compares the complete worst-of-five p99 envelopes for
the two builds.  Shipping is `16.1891` ms; instrumented is `15.3259` ms.  The
conservative correction is therefore `0.8632` ms, below the frozen `2.0`-ms
build-comparability stop.  Adding that constant to every retained raw-oracle
population is an exact deterministic reduction: translating all samples by a
constant translates every reported percentile by that same constant.  No
population was rerun, dropped, or selected.

The derived PASS receipt is bound to the source receipt's SHA-256:

```text
SOURCE_RECEIPT_SHA256=c6921cd41b3bf1be5a3252f0f47e0fa6fa34d2aceb35207d6616a2d54fa3ac7b
COOPERATIVE_WORST_P95_MS=0.1733
COOPERATIVE_WORST_P99_MS=0.3467
WARM_WORST_P95_MS=18.3269
WARM_WORST_P99_MS=20.5058
COLD_WORST_P95_MS=18.5793
HOOKSTAT_INDUCED_TIMEOUTS=0
ORACLE_PRIMARY_RECORD_WORST_P95_MS=0.0544
ORACLE_PRIMARY_RECORD_WORST_P99_MS=0.0870
FULL_ACCEPTANCE_STATUS=PASS
```

The p95 is `0.3269` ms above the preferred engineering-margin target of 18 ms,
but it is `1.6731` ms below the unchanged 20-ms product gate.  The p99 retains
`4.4942` ms of product-gate headroom.
