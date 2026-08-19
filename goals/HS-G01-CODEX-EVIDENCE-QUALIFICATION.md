# HS-G01 — Codex Evidence Qualification

## Goal

Prove, on current Codex plus sanitized fixtures, which evidence source can support retrospective per-handler reliability statistics.

## Candidate surfaces

Inspect read-only: active/archived rollout/session data, local diagnostic/state databases that current Codex actually creates, and relevant public Codex source/contracts. App Server/OTel may be used for comparison but are not automatically admitted as the v0.1 historical source.

## Required matrix

Statuses: completed, failed, blocked, stopped, timeout/protocol failure where observable.
Events: SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, Stop where available.
Identity: include at least one event with two distinct handlers.

For each source record whether it proves: invocation denominator, handler identity, terminal status, timestamp, duration, error material, async coverage, durability after session end.

## Admission

v0.1 source requires durable invocation denominator + per-handler identity + terminal status + timestamp. Duration/error are SHOULD. Coverage may be partial/sync-only if explicitly surfaced.

If no source meets the minimum, finish with `BLOCKED_DATA_SOURCE_DECISION_REQUIRED`. Do not introduce daemon/wrapper/live mutation automatically.
