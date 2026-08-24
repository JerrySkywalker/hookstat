# M5 — first full paired-control qualification

Status: FAIL_CONFIRMED_AFTER_REDESIGN

Exact head: `a6012d2e95826bbcaaac5cbf2e52a1ae5b10a88e`

Receipt: `g35-full-qualification-001.json`

Every before/after control admitted the host. Five single-client runs passed;
their worst p95 was 0.2186 ms and worst p99 was 0.4619 ms. The 16-client
series admitted four runs: the first three passed, but attempt 4 measured
p50=0.2021 ms, p95=0.7476 ms, p99=2.6105 ms, max=5.2700 ms. Because the
admitted p99 exceeds the frozen 2 ms limit, the harness stopped immediately and
reported `FAIL_FROZEN_G28_BUDGET`.

This is not a rejected host window and is not acceptance. G35 remains open and
unmerged. Further work stays inside targeted G35 latency diagnosis; no budget or
evidence semantic is changed.

G35_PERFORMANCE=FAIL_CONFIRMED_AFTER_REDESIGN
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
FROZEN_G28_BUDGET_CHANGED=false
