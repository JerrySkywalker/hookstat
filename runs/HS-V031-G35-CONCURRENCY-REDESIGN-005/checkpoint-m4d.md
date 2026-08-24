# M4D — overlapped Windows client transport candidate

Status: CANDIDATE_SUPPORTED_NOT_ACCEPTED

Candidate parent: `d0967edde22a530c0d520eb79f81d306778e75e0`

The client-only eager-poll checkpoint did not survive full paired-control
qualification. `g35-full-qualification-003.json` admitted all five
single-client runs (worst p95 0.1296 ms / p99 0.2439 ms), then admitted and
failed the first client16 run at p95 1.1121 ms / p99 2.8119 ms.

Further bounded diagnostics rejected two transport variants:

- The feature-gated ACK-handoff stage was small (50 stage runs; resume-after-ACK
  p95 no greater than 0.0216 ms and p99 no greater than 0.0747 ms), disproving
  the per-request acknowledgement channel as the remaining primary tail.
- Yielding the broker connection thread after every ACK and a 250 microsecond
  time-based client spin both increased contention and were reverted. Their
  rejected observations remain in this run directory.

Selected Windows transport candidate:

- The broker retains its synchronous owner-DACL listener and one ordered WAL
  append owner. No listener, WAL, durability, framing, queue-capacity, or
  evidence semantic changed.
- A persistent Windows `IpcClient` owns a current-thread Tokio runtime and an
  overlapped Named Pipe stream. Frame writes and ACK reads use the unchanged
  bounded operation deadline. Unix remains on the synchronous local-socket
  implementation.
- The client uses the platform connector's bounded wait directly. An outer
  timeout around the generic connector was rejected because cancellation left
  its blocking connection helper alive and could later consume a listener
  instance.
- Startup election now rechecks the endpoint after acquiring the lease, and
  lease disappearance/access-denied transitions remain non-owning contention
  under the existing deadline. The 16-thread startup race passed 50/50 repeated
  runs after this correction.
- A raw-listener regression that consumes a frame but withholds the ACK passed
  10/10 at the configured 5 ms client deadline.

Non-acceptance diagnostic evidence:

- 50/50 stage runs passed the frozen cooperative limits. Worst round-trip p50
  was 0.1308 ms, p95 was 0.6243 ms, and p99 was 1.3808 ms. Worst queue-wait p95
  was 0.4289 ms.
- 10/10 corrected per-thread collector runs passed. Worst client16 p50 was
  0.1391 ms, p95 was 0.5093 ms, and p99 was 1.2422 ms.
- The candidate did not move the remaining latency into WAL append or broker
  ACK write; the retained stage receipts expose every measured stage.

Current local gates pass: formatting, diff integrity, Clippy
warnings-as-errors, the full all-feature test suite (including 16-client/10K,
100-client/100K, timeout containment, startup race, WAL recovery, durability
failure, and clean shutdown), and the locked all-feature release build.

`cargo package` remains UNPROVEN for this candidate because both online and
offline archive attempts stalled in registry/package preparation and were
stopped without a receipt. This is not represented as a package pass.

The next acceptance gate is a fresh exact-head paired-control qualification.
No result in this checkpoint claims G35 acceptance.

OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
FROZEN_G28_BUDGET_CHANGED=false
G37_STARTED=false
