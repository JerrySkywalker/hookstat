# G36 landing checkpoint

`G36_PERFORMANCE=FAIL`; this branch is not eligible to merge.

The sanctioned five-run release-artifact measurement used only disposable
capsule and state roots. It recorded no Owner Codex configuration mutation and
no raw private content. The healthy near-timeout fixture completed five times
without a HookStat-induced timeout.

The frozen limits were nevertheless missed:

```text
COOPERATIVE_P95_MS<=1       observed worst=1.9582 (diagnostic series)
COOPERATIVE_P99_MS<=2       observed worst=2.1502 (diagnostic series)
SHIM_WARM_P95_MS<=20        observed worst=26.0996 (diagnostic series)
SHIM_WARM_P99_MS<=25        observed worst=36.3580 (diagnostic series)
SHIM_COLD_P95_MS<=50        observed worst=29.0213 (diagnostic series)
HOOKSTAT_INDUCED_TIMEOUTS=0 observed=0
```

The committed receipts retain the completed full-series failures. A reduced
warmup diagnostic used only for failure classification is intentionally not
acceptance evidence. No performance budget was changed.
