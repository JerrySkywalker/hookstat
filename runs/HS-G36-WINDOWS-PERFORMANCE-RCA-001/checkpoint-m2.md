# M2 — cooperative stage RCA

`g36-cooperative-stage-diagnostic-001.json` is a 100-sample feature-gated
developer receipt. It is diagnostic only and cannot satisfy a frozen budget.

The old fresh one-frame operation has a 2.1071 ms p95 and 3.2200 ms p99. Its
dominant measured stage is acknowledgement read (1.9453 ms p95), which includes
the Windows overlapped wake-up after a connection that is immediately dropped.
The pre-established persistent client is 0.1912 ms p95 / 0.7392 ms p99, and
the current reusable producer is 0.2214 ms p95 / 0.9730 ms p99 with zero gaps.
Broker processing remains below 0.213 ms p95 in the producer comparison.

```text
COOPERATIVE_PRIMARY_HOTSPOT=PER_FRAME_CONNECT_AND_CLOSE_LIFECYCLE
COOPERATIVE_CONNECTION_MODEL_BEFORE=fresh_connection_and_explicit_post_ACK_shutdown_per_emit
COOPERATIVE_CONNECTION_MODEL_AFTER=one_try_lock_guarded_cached_connection_with_25ms_safe_reuse_window
COOPERATIVE_OBSERVATION_GAPS=0
ACK_AFTER_WAL_APPEND=true
G35_ASYNC_DURABILITY_PRESERVED=true
WINDOWS_OVERLAPPED_CLIENT_PRESERVED=true
```

The producer reconnects before sending any later lifecycle frame if its last
acknowledgement is at least 25 milliseconds old. This is below the broker's
50-millisecond idle read release and allows a long-running original Hook to
publish `COMPLETE` on a fresh connection without replaying an ambiguous frame.
