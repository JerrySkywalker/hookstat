# G36 performance evidence index

## Preservation and scope

This index classifies the committed G36 measurements without deleting,
rewriting, or selecting among them.  It contains only receipt-relative paths,
sanitized numeric statistics, and methodology observations.  It does not
contain Owner configuration, commands, capsule contents, paths outside this
repository, or raw payloads.

The frozen G28 limits remain unchanged:

```text
cooperative p95 <= 1 ms; p99 <= 2 ms
transparent shim warm p95 <= 20 ms; p99 <= 25 ms
transparent shim cold p95 <= 50 ms
healthy-hook HookStat-induced timeouts = 0
```

## Evidence taxonomy

| Evidence | Classification | Why | Retained conclusion |
| --- | --- | --- | --- |
| `runs/HS-V031-G35-G36-LANDING-G37-FOUNDATION-007/g36-performance-qualification.json` | `FULL_ACCEPTANCE_FAIL` | Release artifacts, five independent 100-sample series, and all required result fields are present. Cooperative p95/p99 are 2.2717/4.9652 ms and it has one observation gap, independently failing the frozen contract. Its cold worst p95 is 40.0164 ms and its healthy near-timeout count is zero. | A full series proves the pre-RCA cooperative implementation is not acceptable. Its warm subtraction result is retained only as a diagnostic because the warm methodology defect below invalidates that particular warm-tail claim. |
| `runs/HS-V031-G35-G36-LANDING-G37-FOUNDATION-007/g36-performance-qualification-rerun-001.json` | `FULL_ACCEPTANCE_FAIL` | Release artifacts, five independent 100-sample series, and all required fields are present. Cooperative p95/p99 are 15.6190/19.6304 ms with 168 total observation gaps, independently failing the frozen contract. Its cold worst p95 is 40.6802 ms and its healthy near-timeout count is zero. | A second full series confirms that the old producer is not qualifying. Its warm subtraction result is diagnostic only for the same methodology reason. |
| `runs/HS-V031-G35-G36-LANDING-G37-FOUNDATION-007/checkpoint.md` | `REDUCED_DIAGNOSTIC_ONLY` | It declares itself a reduced warmup diagnostic and does not include five complete per-run series, release-artifact provenance, sample vectors, observation-gap accounting, or a complete receipt schema. | The values identify a possible direction for investigation only; they cannot pass or fail G36 acceptance. |
| `runs/HS-G36-WINDOWS-PERFORMANCE-RCA-001/g36-corrected-method-debug-receipts.md` receipt `001` | `INVALIDATED_BY_HARNESS_DEFECT` | It has the corrected warm-up and paired design and a complete five-by-100 series, but was executed through the debug test profile while claiming release artifacts. | The cooperative result is diagnostic evidence for connection reuse; it is not an acceptance result. |
| `runs/HS-G36-WINDOWS-PERFORMANCE-RCA-001/g36-corrected-method-debug-receipts.md` receipt `002` | `INVALIDATED_BY_HARNESS_DEFECT` | Same release-profile harness defect; it separately records the post-wait candidate's complete five-by-100 series. | The warm, cold, and timeout result is diagnostic only. |
| `runs/HS-G36-WINDOWS-PERFORMANCE-RCA-001/g36-release-profile-receipts.md` receipt `001` | `FULL_ACCEPTANCE_FAIL` | Corrected warm/pair method, five 100-sample release series, and release-profile provenance. Cooperative and cold meet their frozen limits, but every warm series exceeds the frozen p95 limit. | The first current candidate is not acceptable. |
| `runs/HS-G36-WINDOWS-PERFORMANCE-RCA-001/g36-release-profile-receipts.md` receipt `002` | `FULL_ACCEPTANCE_FAIL` | Same qualifying methodology and release provenance after the targeted containment/profile improvements. Its worst warm p95/p99 is 62.0514/87.0825 ms. | The startup improvements are insufficient at full warm tails. |

There is no current `FULL_ACCEPTANCE_CAPABLE` G36 receipt. The two new
corrected-method debug receipts are `INVALIDATED_BY_HARNESS_DEFECT`; the two
older committed full receipts each retain an independently valid cooperative
failure while their *warm shim submeasurement* is invalidated as a
tail-acceptance metric by the methodology defects below.

## Why the checkpoint values differ

The checkpoint's `1.9582 / 2.1502`, `26.0996 / 36.3580`, and `29.0213` are
not percentiles copied from either committed full series.  They came from a
reduced diagnostic run, while the two full receipts preserve the worst value
from five independent 100-sample series:

| Metric | Checkpoint diagnostic | Full receipt 1 worst | Full receipt 2 worst |
| --- | ---: | ---: | ---: |
| Cooperative p95 / p99 (ms) | 1.9582 / 2.1502 | 2.2717 / 4.9652 | 15.6190 / 19.6304 |
| Shim warm p95 / p99 (ms) | 26.0996 / 36.3580 | 39.4509 / 44.5804 | 63.4428 / 130.5096 |
| Shim cold p95 (ms) | 29.0213 | 40.0164 | 40.6802 |

The different sample plans, host scheduling conditions, and worst-of-five
aggregation explain the numerical differences.  The diagnostic also uses the
same over-warm/self-load implementation and therefore cannot supersede the
complete retained failures.

## Methodology audit decision

### Cooperative timed region

The existing timed `emit_start` call includes endpoint derivation, connection,
frame encoding/write, broker append/ACK, ACK read, and the explicit Windows
`shutdown()` that runs before `CooperativeProducer::emit_start` returns.  That
shutdown is part of the old synchronous producer operation and must remain in
its old-operation measurement; moving it outside the clock without changing
the operation would be a budget evasion.  A new connection-reuse design may
validly remove per-frame shutdown from the shipped synchronous operation only
if it proves bounded broker retention and truthful stale-connection handling.

The existing benchmark also times only `START` but admits that sample only when
an untimed `COMPLETE` succeeds.  This is not a clean definition of one
cooperative synchronous operation.  The replacement records each successful
emit independently, its disposition, and its observation gaps.

### Transparent shim warm path

G28 defines warm as 25 unmeasured launches of the minimal shim fixture before
each measured fresh start.  G36 instead performs 25 complete shipping-shim /
handler / START / COMPLETE transactions before every timed sample.  Those
transactions create real broker/WAL/process/filesystem work that G28 did not
put in the warm-up definition.  They are a benchmark self-load defect, not an
equivalent cache warm-up.  The replacement warms the actual shipping shim with
25 unmeasured fresh `--help` launches, which parse the shipping executable and
exit before broker/WAL/handler work, then measures fresh shipping-shim
transactions against a ready broker.

Finally, `transparent_duration.saturating_sub(independent_original_duration)`
does not preserve tail percentiles: independently sampled process-duration
tails are not paired observations of one operation, and clipping negative
differences biases the distribution.  It is retained as historical diagnostic
data only.  The corrected test uses an alternating-order paired sequence and
keeps the signed transparent-minus-direct value for every pair; acceptance uses
that paired incremental distribution against the frozen G28 warm budget.

## M1 disposition

```text
FULL_RECEIPTS_CLASSIFIED=2 FULL_ACCEPTANCE_FAIL
REDUCED_DIAGNOSTIC_CLASSIFIED=1
FROZEN_G28_BUDGET_CHANGED=false
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
```
