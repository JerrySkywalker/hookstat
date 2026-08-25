# M3 — cooperative producer connection lifecycle

The cooperative producer now owns at most one local acknowledged connection,
shared only by its clones through a nonblocking `try_lock`. It never introduces
a pool or background retention thread. A frame whose write or ACK is uncertain
is discarded and never replayed; a contended emitter returns the existing
truthful fail-open `Busy` observation.

The broker releases its server-side idle slot after 50 ms. The producer drops
its cached client before a new send once its last acknowledged frame is 25 ms
old, so a `START -> long-running original Hook -> COMPLETE` sequence reconnects
under the existing bounded policy rather than writing to an ambiguous idle
connection. The specific unit test sleeps beyond this window and proves the
later `COMPLETE` is accepted on a fresh connection.

The follow-up 100-sample release diagnostic records current reusable-producer
p95/p99 of `0.1886/0.3131` ms and zero observation gaps. The same probe's
fresh one-frame path is `4.4309/9.3290` ms and its persistent direct client is
`0.2379/0.3615` ms. This reconfirms the causal result without turning a
developer diagnostic into a performance acceptance receipt.

```text
COOPERATIVE_PRIMARY_HOTSPOT=PER_FRAME_CONNECT_AND_CLOSE_LIFECYCLE
COOPERATIVE_CONNECTION_MODEL_BEFORE=FRESH_CONNECTION_PLUS_POST_ACK_CLOSE_PER_FRAME
COOPERATIVE_CONNECTION_MODEL_AFTER=ONE_TRY_LOCK_GUARDED_ACKNOWLEDGED_CONNECTION_25MS_REUSE
COOPERATIVE_OBSERVATION_GAPS=0
ACK_AFTER_WAL_APPEND=true
G35_ASYNC_DURABILITY_PRESERVED=true
WINDOWS_OVERLAPPED_CLIENT_PRESERVED=true
```
