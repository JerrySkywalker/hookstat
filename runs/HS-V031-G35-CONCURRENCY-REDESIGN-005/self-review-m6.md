# M6 — G35 concurrency redesign self-review

Review disposition: PASS_SELF_REVIEW

Reviewed PR head: `2d5568548b3907b50b63b6eb690733a70b372902`

Reviewed base: `8994ab50f0e3899c7cdaf12119b7f50dee8a75d5`

This is the Implementer's required detailed self-review. It is not the fresh
independent acceptance review required before merge.

## Ordering and append integrity

- One append worker remains the only WAL writer. Connection threads submit
  bounded `QueuedFrame` values; the worker writes one completely encoded HSWL
  record with one `write_all` before releasing `Accepted`.
- The WAL record remains header, bounded length, checksum, and one complete
  HSIP frame. No concurrent file writer or record-interleaving path was added.
- Replay stays sequential and deterministic. Existing checksum-corruption,
  truncated-tail, crash-recovery, and idempotent-replay tests pass.

Disposition: PASS.

## Durability semantics

- Exactly one durability worker owns the independently opened same-file handle.
  Generation state coalesces requests; there is no per-frame unbounded message
  queue and no overlapping `sync_data` execution.
- The append worker releases the current ACK after complete OS-buffer append and
  does not wait for `sync_data`. It also continues with subsequent appends while
  durability work is active.
- Thresholds remain 64 records, 65,536 bytes, and 50 ms; per-record fsync remains
  false. The low-traffic timer independently schedules an interval-due sync, so
  a lone record cannot wait for a later frame indefinitely.
- A cloned/opened handle is verified against the same WAL before the worker is
  started. This preserves the documented OS-buffer ACK claim without claiming
  power-loss durability.

Disposition: PASS.

## Failure and shutdown semantics

- A post-ACK durability error sets the shared failed state. Previously returned
  `Accepted` values remain truthful OS-buffer acknowledgements; later submissions
  fail closed as governed rejection/unavailability. No hook outcome is invented.
- Stop prevents new acceptance, joins connection processing, drains the append
  queue, schedules the final pending durability generation, waits for the final
  sync, joins the durability worker, and exits.
- Deterministic injected-failure, final-below-threshold shutdown, and low-traffic
  timed-flush tests pass.

Disposition: PASS.

## Concurrency and race safety

- Append and durability ownership are separate but serialized within each
  responsibility. Shared durability generation/failure state is synchronized;
  no lock is held across `sync_data`.
- Windows producers use overlapped kernel I/O with a client-owned current-thread
  Tokio runtime; the broker retains the owner-DACL synchronous listener. Unix is
  unchanged.
- Frame write and ACK read remain bounded. A raw listener that withholds the ACK
  passes the deterministic timeout regression 10/10.
- The Windows connector bounds its own helper instead of cancelling an unbounded
  blocking connect. Startup performs an elected endpoint recheck, and lease-file
  transition errors never grant ownership. The 16-thread startup election passed
  50/50 repeated runs after this correction.
- The broker queue capacity and overload visibility are unchanged. Existing
  16-client/10K and 100-client/100K tests pass.

Disposition: PASS.

## Privacy and scope

- IPC/WAL schemas still contain only bounded runtime-neutral identifiers and
  lifecycle/status data. No prompt, tool payload, command, environment, path,
  or network telemetry field was added.
- The new performance stage is numeric and feature-gated. Retained JSON receipts
  passed the sanitized-field scan.
- No third evidence path, TCP/HTTP listener, runtime-specific broker policy,
  Owner configuration mutation, trust/apply/restore action, or G37 work exists
  in the diff.

Disposition: PASS.

## Performance evidence

- Phase-1 correlation classified the original queue tail as
  `POST_ACK_SYNC_DATA_STALL`. Moving physical group sync off the append/ACK
  worker materially reduced queue wait without moving the tail into WAL append
  or broker ACK write.
- The final Windows transport candidate passed 50/50 stage diagnostics (worst
  p95 0.6243 ms / p99 1.3808 ms) and 10/10 corrected collector diagnostics
  (worst client16 p95 0.5093 ms / p99 1.2422 ms).
- Fresh paired-control qualification at code head `ec8a10c...` admitted all 20
  controls and all five runs per series. Worst single-client p95/p99 was
  0.1948/0.4478 ms; worst client16 p95/p99 was 0.6159/1.6066 ms. The later PR
  head adds only the retained qualification receipt and checkpoint.
- The frozen 1 ms p95 / 2 ms p99 budget did not change.

Disposition: PASS_REPEATABLE.

## Residual review notes

- A Tokio runtime is constructed once per persistent Windows `IpcClient`,
  outside the measured persistent-send interval. G36 warm/cold shim budgets must
  therefore be freshly qualified after G35 acceptance and unstacking; the G35
  persistent result does not pre-approve G36 cold-start cost.
- `cargo package` for this candidate is UNPROVEN because registry/package
  preparation stalled in both attempted modes. Packaging is not claimed as a
  G35 pass and must be proved again for G36/release work.
- Fresh exact-head Windows/Linux CI and an independently launched read-only
  review remain separate gates.

No self-review finding requires a G35 code correction.

OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
FROZEN_G28_BUDGET_CHANGED=false
G37_STARTED=false
