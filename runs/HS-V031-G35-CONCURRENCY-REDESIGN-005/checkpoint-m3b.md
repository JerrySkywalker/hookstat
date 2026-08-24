# M3B — independent durability handle and bounded coalescing

Status: PASS_CORRECTNESS

The initial cloned-handle async durability candidate retained measurable Windows
sync interference. A feature-only no-sync causal probe reduced 16-client queue
wait p95 from 0.6229–0.8649 ms to 0.2044–0.2964 ms, proving that physical sync
activity, rather than WAL append or ACK write, remained the material source.

The settled candidate:

- keeps one logical ordered WAL append owner and one durability worker;
- opens a distinct sync-only WAL handle and verifies cross-platform same-file
  identity before use;
- schedules every 64-record, 64-KiB, or 50-ms threshold without weakening it;
- coalesces record/byte-triggered requests for at most 2 ms and never beyond the
  existing 50-ms deadline;
- never delays interval-triggered or shutdown durability requests;
- performs no overlapping sync, unbounded messaging, or unbounded thread spawn;
- fails closed for later evidence after an asynchronous durability error; and
- drains accepted appends and waits for the final required sync on clean shutdown.

Focused proofs pass for independent-handle sync/replay, current and subsequent
ACK-path independence, request coalescing, exact threshold triggers, low-traffic
timed flush, clean shutdown, and injected failure. The full all-feature suite
passes: 164 library tests passed / 1 explicit scale test ignored, all ordinary
integration tests passed, and only the pre-established Owner/environment tests
remained ignored. The established 16-client/10K and 100-client/100K IPC matrix
passed.

OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
FROZEN_G28_BUDGET_CHANGED=false
