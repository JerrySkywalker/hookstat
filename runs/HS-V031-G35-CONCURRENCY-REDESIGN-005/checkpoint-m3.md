# HS-V031-G35-CONCURRENCY-REDESIGN-005 — M3 correctness proof

```text
RUN_ID=HS-V031-G35-CONCURRENCY-REDESIGN-005
MILESTONE=M3_CORRECTNESS_FAILURE_SHUTDOWN_PROOF
M2_HEAD=f213ff1
ACK_AFTER_COMPLETE_OS_BUFFER_APPEND=PASS
GROUP_SYNC_NOT_ON_CURRENT_ACK_PATH=PASS
GROUP_SYNC_NOT_ON_SUBSEQUENT_APPEND_PATH=PASS
ONE_DURABILITY_WORKER=true
OVERLAPPING_SYNC_DATA=false
UNBOUNDED_THREAD_SPAWN=false
DURABILITY_REQUEST_COALESCING=PASS
```

Deterministic unit proof covers:

- exact default 64-record, 65,536-byte, and 50 ms group thresholds;
- a low-traffic record schedules the 50 ms sync without a later frame;
- ten accepted append generations continue while the first physical sync is
  deliberately blocked, then coalesce into two serialized sync calls;
- a below-threshold final group is scheduled only by clean shutdown, and
  shutdown cannot finish until that sync is released and completed;
- a cloned, sync-only WAL handle completes `sync_data()` and the original WAL
  owner replays the appended record;
- injected async sync failure preserves the earlier truthful `Accepted`,
  increments visible durability failure health, stops acceptance, and prevents
  a later frame from receiving `Accepted`;
- checksum corruption fails closed and only a truncated final tail is removed;
- bounded overload never manufactures acceptance.

Repository-wide `cargo test --locked --all-features` passed on Windows at the
candidate. This includes IPC startup race, idle expiry, 16-client/10K and
100-client/100K concurrency, deterministic replay/idempotence, privacy, legacy
evidence, timeout, containment, and unrelated product regression coverage.
Only the pre-existing explicit environment/scale opt-in tests remained ignored.

```text
GROUP_MAX_RECORDS=64
GROUP_MAX_BYTES=65536
GROUP_MAX_INTERVAL_MS=50
PER_RECORD_FSYNC=false
LOW_TRAFFIC_TIME_FLUSH=PASS
CLEAN_SHUTDOWN_FINAL_FLUSH=PASS
DURABILITY_FAILURE_FAIL_CLOSED=PASS
WAL_APPEND_ORDER=PASS
WAL_CRASH_RECOVERY=PASS
TRUNCATED_TAIL_RECOVERY=PASS
CHECKSUM_CORRUPTION_FAIL_CLOSED=PASS
REPLAY_IDEMPOTENT=PASS
QUEUE_OVERLOAD_VISIBLE=PASS
BROKER_IDLE_EXPIRY=PASS
STARTUP_RACE=PASS
CLIENT_1=PASS
CLIENT_16=PASS
CLIENT_100=PASS
FRAMES_10K=PASS
FRAMES_100K=PASS
ALL_FEATURES_TESTS=PASS
```

Linux clone-sync semantics and full exact-head hosted CI remain later gates.
Performance remains unclaimed until M4 and M5.

```text
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
FROZEN_G28_BUDGET_CHANGED=false
G37_STARTED=false
PUBLICATION_AUTHORIZED=false
```
