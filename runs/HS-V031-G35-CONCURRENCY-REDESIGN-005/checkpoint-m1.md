# HS-V031-G35-CONCURRENCY-REDESIGN-005 — M1 queue-wait cause

```text
RUN_ID=HS-V031-G35-CONCURRENCY-REDESIGN-005
MILESTONE=M1_QUEUE_WAIT_FLUSH_CORRELATION
BASELINE_G35_HEAD=f1de6fcb5b2f587b374e81a48aaf5c517e4fefe6
MEASUREMENT_COLLECTOR_MODEL=per_thread_local_buffers
CLIENT16_START_BARRIER=true
PRODUCTION_BEHAVIOR_CHANGED=false
QUEUE_WAIT_PRIMARY_CAUSE=POST_ACK_SYNC_DATA_STALL
POST_ACK_SYNC_CORRELATION=PASS
```

The feature-gated diagnostic measures only monotonic timing, queue depth, and
bounded counters. It records no frame content, path, command, process detail,
or environment value. Each request's enqueue-to-dequeue interval is intersected
with the append worker's measured group-`sync_data()` interval.

All observations are retained:

| Receipt | Result | queue wait p95 | sync overlap p95 | residual p95 | sync p95 | p95-tail overlap |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `g35-flush-correlation-001.json` | rejected `read_ack_timeout` | — | — | — | — | — |
| `g35-flush-correlation-002.json` | measured | 0.8595 ms | 0.5684 ms | 0.3516 ms | 0.7655 ms | 81 / 81 |
| `g35-flush-correlation-003.json` | measured | 1.1444 ms | 0.8188 ms | 0.3626 ms | 0.9135 ms | 81 / 81 |

Both measured runs performed 24 group-sync attempts, reached queue depth 16,
and recorded worker-dequeue handoff p95 of 0.0002 ms. Every request in the p95
queue-wait tail overlapped group durability. Removing the directly measured
sync overlap reduced residual queue-wait p95 below 0.363 ms. This falsifies a
primary worker-scheduling-hop explanation and proves post-ACK group sync as the
material bounded-concurrency stall. Serial service remains a smaller residual,
not the primary cause.

The production sequence at the baseline head is therefore:

```text
append complete WAL record
 -> release Accepted to connection thread
 -> same append worker executes due sync_data()
 -> queued subsequent requests cannot dequeue until sync_data() completes
```

The frozen G28 budget, WAL-before-ACK semantics, group thresholds, and failure
semantics were not changed by M1.

```text
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
FROZEN_G28_BUDGET_CHANGED=false
G37_STARTED=false
PUBLICATION_AUTHORIZED=false
```
