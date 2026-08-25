# M4C — transport-tail diagnostic and bounded Windows ACK-read candidate

Status: CANDIDATE_SUPPORTED_NOT_ACCEPTED

Base head: `5f8fd06`

After the complete-record WAL write removed append scatter, repeated stage
diagnostics showed that remaining frozen-budget misses were no longer explained
by WAL append or queue wait. The tails appeared in the Windows Named Pipe read
boundary. The production read remains deadline-bounded and nonblocking.

Rejected controlled variants retained here:

- An 8 ms durability coalescing interval reduced physical syncs to 1–2 but did
  not remove transport tails. It was reverted to the 2 ms coalescing window;
  the frozen 50 ms maximum interval never changed.
- A 64-poll eager phase on both broker and client readers passed 20/20 stage
  diagnostics but only 3/5 corrected client-visible diagnostics. It was
  rejected because it caused avoidable reader contention.
- A client-only 32-poll eager phase passed 28/30 stage diagnostics and 9/10
  corrected client-visible diagnostics, including one `read_ack_timeout`.

Selected candidate:

- Broker connection reads retain the existing yield cadence.
- Only the client ACK read gets a bounded 64-empty-poll eager phase, after
  which it returns to the existing yield cadence until the unchanged deadline.
- This variant passed 10/10 corrected client-visible diagnostics. Worst
  client16 p95 was 0.7679 ms and worst p99 was 1.7631 ms.
- It passed 29/30 feature-gated stage diagnostics. The single diagnostic miss
  was p95 0.9052 ms / p99 2.6812 ms, still in the transport-read tail rather
  than WAL append or queue wait. This diagnostic series is not acceptance
  evidence.

The selected candidate passed formatting, Clippy warnings-as-errors, the full
all-feature test suite (including 16-client/10K, 100-client/100K, broker idle
expiry, timeout containment, async durability, failure, and shutdown tests),
and an all-feature locked release build in an isolated target directory.

The next gate is a fresh paired-control qualification. No diagnostic result in
this checkpoint claims G35 acceptance.

OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
FROZEN_G28_BUDGET_CHANGED=false
