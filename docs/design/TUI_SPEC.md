# HookStat TUI Specification

Status: **normative design baseline**. The overall information hierarchy and compact terminal style below are frozen for v0.x unless an explicit presentation goal changes this file.

## Primary design principles

- Statistics first, not a doctor/checklist UI.
- Failure rate MUST always accompany sample count.
- Coverage MUST be visible when incomplete or source-limited.
- Partial coverage MUST NOT render as `0.00% healthy`.
- Blocked/denied/stopped runtime decisions MUST NOT automatically equal execution failure.
- Runtime summaries MUST NOT hide per-handler reliability.
- Unsupported runtimes are omitted, not shown as fake zero rows.
- Handler revision boundaries remain a planned query dimension even though v0.1 may defer revision analytics.

## Frozen multi-runtime home baseline

```text
Hook Reliability                          Last 7 days

Runtime          Runs       Failed      Failure     Δ prev
────────────────────────────────────────────────────────────
Codex            12,481         34        0.27%     +0.08%
DeepSeek H.       3,802          6        0.16%     -0.04%
OpenCode              —          —            —     partial


Most unreliable hooks

HAPI / SessionStart / Codex
  17 / 239                              7.11%   ↑

ntfy / Stop / Codex
   9 / 811                              1.11%   ↓

foo / PreToolUse / DSH
   4 / 1,823                            0.22%   →
```

The above is the long-term layout. v0.1 must omit unsupported runtime rows rather than rendering placeholders.

## Frozen v0.1 Codex-only home rendering

```text
Hook Reliability                          Last 7 days

Runtime          Runs       Failed      Failure     Δ prev
────────────────────────────────────────────────────────────
Codex            12,481         34        0.27%     +0.08%


Most unreliable hooks

HAPI / SessionStart / Codex
  17 / 239                              7.11%   ↑

ntfy / Stop / Codex
   9 / 811                              1.11%   ↓

PUA / PreToolUse / Codex
   4 / 9,182                            0.04%   →
```

## Frozen hook detail baseline

```text
HAPI · SessionStart · Codex

24h       2 / 62        3.23%
7d       17 / 239       7.11%
30d      41 / 851       4.82%
All      53 / 1421      3.73%

p50       81 ms
p95      412 ms
p99     1184 ms

Current revision
failure     2.1%

Previous revision
failure     8.7%
```

For v0.1, if revision identity is not yet implemented, the `Current revision` / `Previous revision` block is omitted, not replaced by invented data. If duration is not proven by the admitted evidence source, the latency block is omitted.

## Interaction baseline

- Up/Down or `j`/`k`: select handler.
- Enter: detail view.
- `1`, `7`, `3`, `a`: 24h, 7d, 30d, All where practical; an equivalent compact selector is acceptable.
- `r`: refresh/ingest new evidence.
- Esc/Backspace: back.
- `q`: quit.

## States

Empty: explain that no admitted Codex hook evidence was found; do not call it healthy.

Partial: show a visible coverage label near affected runtime/handler statistics.

Error: preserve previously accepted ledger and show the new ingest error without erasing history.

Small terminal: preserve counts/rate/handler identity before secondary columns such as delta or latency.
