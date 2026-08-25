# HS-V031-G35-CONCURRENCY-PERFORMANCE-INTEGRITY-004

## Measurement-integrity finding

`G35_PERF_FINDING_02=CONCURRENT_MEASUREMENT_COLLECTOR_CONTAMINATION`

The pre-correction runner executed `ACK completion -> shared samples Mutex ->
before.elapsed()`. The retained historical frozen-budget receipt is immutable
historical evidence; this finding prevents treating it alone as a broker
product failure.

The corrected runner executes `ACK completion -> elapsed capture ->
worker-local collection`, aggregates only after worker joins, and releases a
16-client start barrier only after all persistent connections are established.
The regression test proves an artificial post-capture collector delay cannot
alter a recorded latency sample.

## A/B diagnostic

`g35-collector-model-ab-001.json` is non-acceptance evidence. It records:

```text
legacy client16 p95=2.0949 ms, p99=4.1487 ms
per-thread client16 p95=0.6896 ms, p99=0.9653 ms
collector contamination effect at p95=1.4053 ms
```

## Corrected qualification and stage diagnostic

`g35-corrected-qualification-001.json` is the first acceptance-capable
corrected receipt. Its five admitted single-client runs pass; its second
admitted 16-client run records p95=1.0531 ms and p99=1.7588 ms. The frozen
budget is unchanged, so this receipt is `FAIL_FROZEN_G28_BUDGET` and has not
been used for acceptance.

`g35-stage-timing-001.json` is feature-gated diagnostic evidence only. Its
16-client p95 estimates identify queue wait (0.6039 ms) as the material broker
stage. Activity bookkeeping (0.0002 ms), acknowledgement-channel allocation
(0.0005 ms), queue submission (0.0004 ms), WAL append (0.0172 ms), worker
acknowledgement handoff (0.0025 ms), and broker ACK write (0.0072 ms) are not
the primary hotspot.

`g35-stage-timing-002.json` retains a later diagnostic measurement failure as
the bounded class `read_ack_timeout`; it contains no raw I/O text or private
content and is not used to alter the diagnosis or the frozen budget.

## Conclusion

The observed result is **both** a collector contamination defect and a real
bounded-concurrency serialization effect. No production broker semantics were
changed by this checkpoint. In particular, WAL-before-ACK and the governed
64-record / 64 KiB / 50 ms group-durability policy remain intact.

```text
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
FROZEN_G28_BUDGET_CHANGED=false
G37_STARTED=false
PUBLICATION_AUTHORIZED=false
```
