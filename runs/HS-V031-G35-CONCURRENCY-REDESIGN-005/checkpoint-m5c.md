# M5C — overlapped-client exact-head paired-control qualification

Status: PASS_REPEATABLE

Exact head: `ec8a10c576d0e93eed29bb6ca07a075c39180d3b`

Receipt: `g35-full-qualification-004.json`

The qualifier was freshly built from the exact head in an isolated target
directory. All 20 paired before/after controls admitted the host and no run was
rejected. The harness retained five admitted observations for each required
series; every observation passed the frozen G28 cooperative limits.

- Single client: 5/5 passed; worst p50 0.0701 ms, p95 0.1948 ms, p99
  0.4478 ms.
- 16 clients: 5/5 passed; worst p50 0.1432 ms, p95 0.6159 ms, p99
  1.6066 ms.
- Frozen limits remained p95 no greater than 1 ms and p99 no greater than 2 ms.
- The receipt records `owner_live_codex_config_mutated=false`,
  `raw_private_content_captured=false`, and `changed=false` for the frozen
  budget.

This establishes G35 performance acceptance for this exact code head. It does
not establish exact-head CI or independent review, and it does not authorize a
merge by itself.

G35_PERFORMANCE=PASS_REPEATABLE
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
FROZEN_G28_BUDGET_CHANGED=false
G37_STARTED=false
