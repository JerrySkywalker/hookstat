# HS-V031-G35-CONCURRENCY-REDESIGN-005 — M2 redesign candidate

```text
RUN_ID=HS-V031-G35-CONCURRENCY-REDESIGN-005
MILESTONE=M2_CONCURRENCY_REDESIGN_CANDIDATE
M1_HEAD=d15f252
REDESIGN_SELECTED=ORDERED_APPEND_OWNER_PLUS_ASYNC_GROUP_DURABILITY
DEDICATED_DURABILITY_WORKER=true
ONE_DURABILITY_WORKER=true
UNBOUNDED_THREAD_SPAWN=false
OVERLAPPING_SYNC_DATA=false
APPEND_WORKER_WAITS_FOR_SYNC_DATA=false
DURABILITY_REQUEST_COALESCING=true
```

The candidate retains one logical WAL append owner and adds exactly one
physical durability owner. The append worker writes one complete checksum-
framed record, returns `Accepted`, cuts a due durability generation, and
continues dequeuing. Constant-size coordinator state coalesces later due
generations while the single durability worker is active; it does not enqueue
one message or create one thread per record.

The sync-only handle is produced by `File::try_clone()` from the already
validated append WAL. Rust specifies a shared underlying file handle. Windows
`DuplicateHandle` refers to the same object and `FlushFileBuffers` flushes the
specified file; Unix duplicate descriptors refer to the same open-file
description and `fsync` operates on that file. The clone never writes or seeks.
One append owner therefore remains responsible for framing and order.

The 64-record, 65,536-byte, and 50 ms request thresholds are unchanged. A 2 ms
append-worker receive poll evaluates the time trigger even without a later
frame. Clean shutdown schedules the final pending generation and waits for the
durability worker before exit. Sync failure is linearized against append/ACK
through a small failure-publication gate, then stops acceptance and makes later
evidence rejected or unavailable without rewriting an earlier `Accepted`.

Focused proof at this candidate:

```text
GROUP_SYNC_NOT_ON_CURRENT_ACK_PATH=PASS
GROUP_SYNC_NOT_ON_SUBSEQUENT_APPEND_PATH=PASS
COALESCED_DURABILITY_REQUESTS=PASS
CLEAN_SHUTDOWN_WAITS_FOR_FINAL_SYNC=PASS
LOW_TRAFFIC_TIME_TRIGGER=PASS
DURABILITY_FAILURE_FAIL_CLOSED=PASS
CLONED_HANDLE_SYNC_AND_RECOVERY_WINDOWS=PASS
IPC_FEATURE_GATED_UNIT_TESTS=23_PASS_0_FAIL
```

Full correctness, cross-platform CI, and performance evidence remain M3 and
later gates. This checkpoint is not acceptance.

```text
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
FROZEN_G28_BUDGET_CHANGED=false
G37_STARTED=false
PUBLICATION_AUTHORIZED=false
```
