# M5B — complete-record candidate paired-control qualification

Status: FAIL_CONFIRMED_AFTER_REDESIGN

Exact head: `3fb3de647ac92ae7f0c21f13b57c963379f82387`

Receipt: `g35-full-qualification-002.json`

Five single-client admitted runs all passed (worst p95 0.1257 ms, worst p99
0.3103 ms). The client16 series retained three sanitized `read_ack_timeout`
measurement errors and one rejected noisy pre-control. Client16 attempt 4 was
admitted and passed at p95 0.7038 ms / p99 1.1548 ms. Attempt 6 was also fully
admitted but failed at p95 1.0442 ms / p99 2.6254 ms, so qualification stopped
with `FAIL_FROZEN_G28_BUDGET`.

The result proves the complete-record single write materially improves ordinary
latency but does not yet make every admitted client16 run repeatable. G35 remains
open and unmerged. All rejected and admitted observations are retained.

G35_PERFORMANCE=FAIL_CONFIRMED_AFTER_REDESIGN
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
FROZEN_G28_BUDGET_CHANGED=false
