# G36 performance evidence index

## Preservation and scope

This index classifies the committed G36 measurements without deleting,
rewriting, or selecting among them.  It contains only receipt-relative paths,
sanitized numeric statistics, and methodology observations.  It does not
contain Owner configuration, commands, capsule contents, paths outside this
repository, or raw payloads.

The frozen G28 baseline receipt and its reference values remain unchanged:

```text
cooperative p95 <= 1 ms; p99 <= 2 ms
transparent shim warm p95 <= 20 ms; p99 <= 25 ms
transparent shim cold p95 <= 50 ms
healthy-hook HookStat-induced timeouts = 0
```

For prospective v0.3.1 release qualification, the Owner approved a one-time
warm product hard-cap recalibration to p95 `25 ms` and p99 `30 ms`. Host
admission remains p95 `20 ms` and p99 `25 ms`; cooperative, cold, and timeout
limits are unchanged. The versioned authority and evidence basis are recorded
in `docs/performance/HS-G36-WARM-BUDGET-RECALIBRATION.md`. Historical receipt
outcomes and the G28 `20/25`-ms reference target are not rewritten.

## Evidence taxonomy

The only classification values used by this index are:

```text
FULL_ACCEPTANCE_FAIL
FULL_ACCEPTANCE_PASS
DIAGNOSTIC_ONLY
INVALIDATED_BY_METHOD
INVALIDATED_BY_BUILD_PROFILE
```

Receipt-level outcomes are preserved as they were recorded.  Where one
receipt contains measurements with different evidentiary status, the table
classifies those measurement groups separately rather than rewriting the
receipt's historical `outcome` field.

| Evidence / measurement group | Classification | Why | Retained conclusion |
| --- | --- | --- | --- |
| `runs/HS-V031-G35-G36-LANDING-G37-FOUNDATION-007/g36-performance-qualification.json`: receipt outcome and cooperative series | `FULL_ACCEPTANCE_FAIL` | Release artifacts, five independent 100-sample series, and all required result fields are present. Cooperative p95/p99 are 2.2717/4.9652 ms and it has one observation gap, independently failing the frozen contract. | The pre-RCA cooperative implementation was not acceptable. |
| Same receipt: unpaired saturating warm subtraction | `INVALIDATED_BY_METHOD` | It subtracts independent process lifetimes and clips negative samples. | It cannot prove a warm pass or failure. |
| Same receipt: cold p95 and healthy near-timeout result | `DIAGNOSTIC_ONLY` | Cold p95 is 40.0164 ms and the induced-timeout count is zero, but the receipt already fails cooperative acceptance and cannot be promoted piecemeal to a full pass. | It supports continuity only. |
| `runs/HS-V031-G35-G36-LANDING-G37-FOUNDATION-007/g36-performance-qualification-rerun-001.json`: receipt outcome and cooperative series | `FULL_ACCEPTANCE_FAIL` | Cooperative p95/p99 are 15.6190/19.6304 ms with 168 total observation gaps, independently failing the frozen contract. | The old producer was not acceptable. |
| Same receipt: unpaired saturating warm subtraction | `INVALIDATED_BY_METHOD` | It has the same independent-lifetime and clipping defects. | It cannot prove a warm pass or failure. |
| Same receipt: cold p95 and healthy near-timeout result | `DIAGNOSTIC_ONLY` | Cold p95 is 40.6802 ms and the induced-timeout count is zero, but the full receipt fails cooperative acceptance. | It supports continuity only. |
| `runs/HS-V031-G35-G36-LANDING-G37-FOUNDATION-007/checkpoint.md` reduced measurements | `DIAGNOSTIC_ONLY` | It declares itself a reduced warmup diagnostic and lacks five complete per-run series, release-artifact provenance, sample vectors, observation-gap accounting, and a complete receipt schema. | The values identify a direction only. |
| `runs/HS-G36-WINDOWS-PERFORMANCE-RCA-001/g36-corrected-method-debug-receipts.md` receipts `001` and `002` | `INVALIDATED_BY_BUILD_PROFILE` | Both complete series were executed through the debug test profile while claiming release artifacts. | Their numbers are retained but cannot prove acceptance. |
| `runs/HS-G36-WINDOWS-PERFORMANCE-RCA-001/g36-release-profile-receipts.md` receipts `001` and `002`: recorded receipt outcomes | `FULL_ACCEPTANCE_FAIL` | These release receipts truthfully record failure under the then-current paired-subtraction method. | The historical failure outcomes are preserved. |
| Same release receipts: cooperative, cold, and healthy near-timeout groups | `DIAGNOSTIC_ONLY` | Cooperative worst p95/p99 is 0.2798/0.7591 ms with zero gaps, cold worst p95 is 38.4506 ms, and induced timeouts are zero. A complete acceptance pass still requires a valid warm metric on one exact candidate. | Current continuity evidence supports cooperative/cold/timeout status but is not a standalone full pass. |
| Same release receipts: alternating paired warm subtraction | `INVALIDATED_BY_METHOD` | Alternating adjacent samples remove order preference and reduce low-frequency drift, but each delta still subtracts two distinct child process lifetimes with independent Windows scheduling noise. | The 62.0514/87.0825-ms tail cannot establish the transparent overhead tail. |
| `runs/HS-G36-WINDOWS-PERFORMANCE-RCA-001/g36-cooperative-stage-diagnostic-001.json` | `DIAGNOSTIC_ONLY` | Feature-gated 100-sample stage attribution, explicitly not acceptance evidence. | It identifies the old per-frame connect/close lifecycle and supports the reusable-client change. |
| `runs/HS-G36-WINDOWS-PERFORMANCE-RCA-001/g36-shim-stage-diagnostic-001.json` | `DIAGNOSTIC_ONLY` | Feature-gated 100-sample stage attribution, explicitly not acceptance evidence. | Shipping startup is the primary one-process cost; child wait is not HookStat overhead. |
| `runs/HS-G36-WARM-SHIM-ARCHITECTURE-DECISION-002/g36-same-invocation-oracle-smoke-001.json` | `DIAGNOSTIC_ONLY` | Bounded ten-sample implementation smoke, not a tail series. | It proved the fixed-record channel end to end and retained its loaded-host outliers. |
| `runs/HS-G36-WARM-SHIM-ARCHITECTURE-DECISION-002/g36-same-invocation-oracle-001.json` | `DIAGNOSTIC_ONLY` | One 100-sample release oracle series with shipping/instrumented startup comparison; it explicitly sets `acceptance_evidence=false`. | Raw same-invocation overhead is 16.3280/16.6528 ms p95/p99 with the entire oracle channel included. A 1.1668-ms p99 startup-tail correction still yields 17.4948/17.8196 ms, supporting one-process viability but not full acceptance. |
| `runs/HS-G36-WARM-SHIM-ARCHITECTURE-DECISION-002/g36-performance-qualification-same-invocation-001.json` | `INVALIDATED_BY_METHOD` | The complete release populations are retained, but the first reducer selected the maximum signed p99 difference from one adjacent shipping/instrumented startup pair. Those two independently scheduled populations do not make that pairwise tail identifiable. | Its raw samples remain the immutable input to the corrected deterministic reduction; its recorded FAIL outcome is not acceptance evidence. |
| `runs/HS-G36-WARM-SHIM-ARCHITECTURE-DECISION-002/g36-performance-qualification-same-invocation-derived-001.json` | `FULL_ACCEPTANCE_PASS` | It is SHA-256-bound to the complete source receipt and changes only its reducer: the conservative build correction is the shipping worst-of-five p99 envelope minus the instrumented worst-of-five p99 envelope. A constant translation exactly translates every retained raw-oracle quantile. | Cooperative is 0.1733/0.3467 ms p95/p99, warm is 18.3269/20.5058 ms, cold p95 is 18.5793 ms, observation gaps are zero, and induced timeouts are zero. |
| `runs/HS-G36-WINDOWS-PERFORMANCE-RCA-001/g36-full-acceptance-08f83bc-fail.json` | `FULL_ACCEPTANCE_FAIL` | Clean exact source head `08f83bc...`, ordinary and instrumented release-artifact hashes, five independent 100-sample populations, and the accepted same-invocation reducer are recorded. | Cooperative is 0.1357/0.2187 ms p95/p99, cold p95 is 20.5353 ms, and induced timeouts are zero. Warm is 32.6951/245.2032 ms and therefore fails the frozen contract. Four warm populations remain close to the gate while one retains a large fresh-process scheduling tail; the one-process candidate lacks repeatable acceptance margin. |
| `runs/HS-G36-WARM-SHIM-ARCHITECTURE-DECISION-002/g36-idle-helper-frontend-floor-001.json` | `DIAGNOSTIC_ONLY` | A package-excluded bounded prototype sends only one fixed eight-byte request and response; it has no capsule, handler, containment, evidence, or cold-fallback semantics and explicitly denies acceptance status. | Its 179-KB fresh frontend plus one local helper exchange is 325.5800/451.8487 ms p95/p99 over 500 cache-warmed samples in the observed environment. The helper option cannot improve this strict floor by adding required semantics. |
| `runs/HS-G36-WARM-HOST-ADMISSION-AND-LANDING-003/g36-host-admitted-qualification-1012a81-001-window-*.json` | `INVALIDATED_BY_BUILD_PROFILE` | The immutable window receipts retain eight raw `REJECTED_HOST_SUBSTRATE` dispositions and one raw `FAIL_FROZEN_BUDGET` disposition. The session's instrumented build was `9.9580 ms` faster than shipping under the complete-envelope reducer, exceeding the pre-existing `2.0 ms` build-comparability stop. The comparator ran before, and outside, the later passing-control window, so its cross-regime correction cannot establish a shipping-binary product tail. | Window 9's controls pass at 12.0442/14.6764 ms pre and 13.3663/15.1660 ms post. Its raw same-invocation oracle is 17.5643/21.0011 ms; adding the invalid 9.9580-ms cross-regime correction produces the receipt's preserved raw 27.5223/30.9591-ms failure disposition. This session proves neither product PASS nor product FAIL for release acceptance and is not relabelled as host rejection. |
| `runs/HS-G36-WARM-HOST-ADMISSION-AND-LANDING-003/g36-host-admitted-qualification-36173fd-001-startup-comparability-*.json` | `DIAGNOSTIC_ONLY` | Ten exact-head, SHA-bound build-comparability attempts each retain their own pre control, five complete shipping/instrumented startup populations, post control, and raw `REJECTED_HOST_SUBSTRATE` disposition. No attempt had both controls pass, so no build correction was admitted and warm product qualification never began. | Attempts 7 through 10 approached the fixed control gate but still failed at least one phase. This series proves only that the observed 13,651.85-second search had no admitted build-comparability window; it proves neither product PASS nor product FAIL. |
| `runs/HS-G36-WARM-HOST-ADMISSION-AND-LANDING-003/g36-host-admitted-qualification-93a5a1e-001.json` and its atomic subreceipts | `FULL_ACCEPTANCE_FAIL` | Clean exact source head `93a5a1e...`, exact artifact hashes, an admitted zero-bias build-comparability envelope, and a complete 100-sample warm pre/product/post window are retained. Both warm controls pass, so the predefined policy admits the independent product result. | Warm product p95/p99 is 24.2950/26.5946 ms and fails both then-governing G28 reference limits. Cooperative passes at 0.2118/0.4732 ms with zero gaps, cold passes at 28.9614 ms p95, and induced timeouts, unexpected terminals, and oracle gaps are all zero. Its raw `FAIL_FROZEN_BUDGET` remains immutable. It supplies the shipping evidence for the prospective recalibration but is not itself rewritten as a pass. |

The current `FULL_ACCEPTANCE_PASS` is the deterministic reduction of one
complete release qualification, not a rerun or selected subset.  The two
corrected-method debug receipts remain `INVALIDATED_BY_BUILD_PROFILE`; the two
older committed full receipts each retain an independently valid cooperative
failure while their warm shim submeasurement remains
`INVALIDATED_BY_METHOD`.

## Owner-approved prospective warm admission overlay

The retained taxonomy above describes each receipt under the methodology and
release contract in force when it was produced. It is not rewritten by either
later Owner decision. For v0.3.1 landing, each new warm series must be
bracketed in the same session by pre and post executions of the exact G28
cache-warmed minimal-shim process-start control. Each control uses 25
unmeasured launches before every one of at least 100 measured launches and
nearest-rank percentiles. The predefined control limits remain p95 `20 ms` and
p99 `25 ms`.

Both controls must pass to admit the product series. A control failure produces
`REJECTED_HOST_SUBSTRATE`, not product PASS or FAIL. If both pass, the
same-invocation product metric is evaluated against the Owner-approved one-time
v0.3.1 `25/30`-ms release hard cap. The G28 `20/25`-ms product values remain the
reference target. No host-control subtraction or post-hoc threshold is
permitted, and five independently admitted passing warm runs are required. An
admitted product result above `25/30` ms is
`FAIL_RECALIBRATED_BUDGET` and stops landing; it cannot authorize a further
increase. The already-frozen instrumented-versus-shipping comparability stop
remains independent: a materially faster instrumented build invalidates the
product metric rather than proving either release PASS or FAIL.

Prospective build comparability is itself bracketed by the same predefined G28
pre/post controls before its correction can affect any warm product window.
This prevents a comparison observed in a rejected host regime from being
transferred into a later admitted product result. A pre-oracle exit is retained
as an observation gap and terminal class; it cannot abort before the
post-control or silently reduce an accepted sample population. An independently
observed non-success terminal result remains a product failure when both
controls pass.

The exact `08f83bc...` receipt and every earlier G36 receipt predate this
prospective envelope. None contains the predefined contemporaneous pre and
post G28 controls. Therefore:

```text
RAW_RECEIPT_OUTCOME_08F83BC=FAIL
RELEASE_ACCEPTANCE_CLASSIFICATION_08F83BC=NOT_RETROACTIVELY_CLASSIFIED
NON_ADMITTED_HOST_SUBSTRATE_08F83BC=UNPROVEN_NO_CONTEMPORANEOUS_CONTROL
G28_REFERENCE_WARM_P95_P99_MS=20/25
V031_RELEASE_WARM_P95_P99_MS=25/30
HOST_ADMISSION_P95_P99_MS=20/25
FURTHER_AUTOMATIC_BUDGET_RELAXATION=false
FROZEN_G28_HISTORICAL_EVIDENCE_CHANGED=false
```

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
keeps the signed transparent-minus-direct value for every pair.  That was a
necessary interim correction, but the M1 audit below proves it is still not an
identifiable warm-tail acceptance metric.

### M1 warm metric identifiability audit

The alternating signed method corrected clipping, warmup self-load, and fixed
order.  It did not make the incremental tail identifiable.  For adjacent
observations `i` and `j`, the measured value is:

```text
delta = transparent_i - direct_j
      = HookStat_overhead_i + original_child_i - direct_child_j
```

The two child terms are different process lifetimes.  Alternation can balance
order effects and adjacency can reduce slow machine-load drift, but neither
creates the covariance needed to cancel independent Windows process-scheduling
noise.  Quantiles are also not linear: the p95 or p99 of a difference is not
the difference of the component quantiles.  The method can therefore move the
reported tail both above and below the real HookStat overhead tail.

The frozen G28 warm limit was calibrated from a cache-warmed fresh minimal-shim
startup.  Operationally it governs HookStat-added synchronous transparent
overhead, including shipping shim startup, pre-handler instrumentation,
post-handler instrumentation, and shim exit, while excluding the original
handler's own child spawn/wait lifetime.  The numeric 20/25-ms limits are
unchanged.

The acceptance replacement is a same-invocation oracle:

```text
parent-observed full shipping-shim lifetime
- shim-observed interval from immediately before child spawn
  through completion of the child wait
= HookStat transparent overhead
```

This subtraction uses one actual transparent invocation.  Scheduler delay
while the original child lifecycle is active belongs to the subtracted
interval; scheduler delay in the shim's own startup/pre/post/exit regions
remains charged to HookStat.  The oracle is diagnostic until its bounded timing
side channel and instrumented-versus-shipping startup effect are measured.

```text
WARM_ACCEPTANCE_METRIC=OTHER_PROVEN_METRIC
OTHER_PROVEN_METRIC=SAME_INVOCATION_PARENT_LIFETIME_MINUS_CHILD_SPAWN_WAIT
PAIRED_METHOD_IDENTIFIABLE=false
PAIRED_INCREMENTAL_STATUS=PAIRED_INCREMENTAL_NOT_IDENTIFIABLE
FROZEN_G28_BUDGET_CHANGED=false
```

## M1 disposition

```text
FULL_ACCEPTANCE_FAIL_RECEIPTS=5
FULL_ACCEPTANCE_PASS_RECEIPTS=1
DIAGNOSTIC_ONLY_MEASUREMENT_GROUPS=9
INVALIDATED_BY_METHOD_MEASUREMENT_GROUPS=4
INVALIDATED_BY_BUILD_PROFILE_RECEIPTS=2
WARM_ACCEPTANCE_METRIC=OTHER_PROVEN_METRIC
PAIRED_METHOD_IDENTIFIABLE=false
FROZEN_G28_BUDGET_CHANGED=false
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
```
