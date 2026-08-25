# HS-G35 implementer review

Run: `HS-V031-G35-RUNTIME-NEUTRAL-IPC-BROKER-001`

This is an implementer self-review, not the independent acceptance review
required before merge.

| Review focus | Result | Evidence |
| --- | --- | --- |
| A. Binary protocol correctness | PASS | v1 magic/version/type/flags/length framing rejects malformed, truncated, oversized, invalid-enum, and trailing payload cases. |
| B. WAL persistence | PASS | Append-only bounded records use checksums and batch `sync_data`; producers do not write SQLite or issue per-record sync. |
| C. Crash recovery | PASS | Valid append order replays; truncated final records are safely discarded; malformed non-tail records fail closed. |
| D. Queue/backpressure | PASS | Fixed `sync_channel`, connection cap, acknowledgement deadline, drop/reject/busy values, and bounded health counters are exercised. |
| E. Windows Named Pipe security | PASS, Windows test environment | Generic local socket maps to Named Pipe with owner-rights DACL; no TCP endpoint is constructed. |
| F. Unix socket/path security | PASS by implementation and platform-gated test | Unix socket is `0600`; state and IPC directories reject symlink/unsafe object conditions; stale reclaim is limited to a dead socket in the verified directory. Linux CI remains required for platform acceptance. |
| G. Runtime-neutral boundary | PASS | Broker transports opaque bounded references only and normalizes to `CanonicalEvidence`; no runtime config, trust, matcher, or source-path schema crosses the boundary. |
| H. Privacy | PASS | Wire/WAL share one compact binary lifecycle schema and tests assert no raw content field/value reaches either representation. |
| I. G36 scope | PASS | Only a synthetic test client exists. No transparent shim, handler execution, Hook configuration, or runtime setup was added. |

## Open acceptance boundary

Independent review is still required before merge. The review should inspect the
exact PR head, including Windows Named Pipe and Linux Unix Domain Socket CI.
No Owner live Codex configuration was read or mutated for this run.

Performance acceptance is also blocked: a later 16-client optimized Windows
sample under unrelated host load measured p95 2.141 ms and p99 8.314 ms,
outside the frozen G28 budget. The sanitized evidence retains both that outlier
and the earlier passing release measurement; it does not relax the budget.
