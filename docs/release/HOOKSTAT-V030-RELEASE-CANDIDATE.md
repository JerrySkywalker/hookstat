# HookStat v0.3.0 Release Candidate

## Status

`v0.3.0` is an unpublished release candidate. It is the only input to the
separate Owner publication gate after its exact commit is merged and frozen.
This document neither authorizes nor performs a crates.io publication, Git tag,
or GitHub Release.

## Highlights

- Codex Changes workbench with coverage-aware regression, recovery, new-hook,
  and revision history views derived solely from admitted evidence.
- Hook Catalog with Human aliases, revision metadata, selected-period sample
  confidence, freshness, compact trends, and bounded failure exploration.
- Shared `jerry-terminal-ui` `=0.1.0` implementation boundary for the
  TabBeacon-compatible Human shell, navigation, footer, editing, Help, locale,
  CJK-safe layout, and terminal lifecycle.

## Preserved boundaries

- production runtime is Codex only; no non-Codex adapter is a release dependency;
- normal launch remains `codex`; HookStat is neither a launcher wrapper nor a
  daemon;
- instrumentation and trust remain explicit opt-in actions; `--apply` does not
  grant trust;
- coverage stays truthful, every failure rate retains its sample count, and
  HookStat persists no raw prompts, tool payloads, command streams, or telemetry.

## Publication boundary

Publication requires the subsequent `HS-V03-PUBLIC-RELEASE-OWNER-GATE` to
authorize the frozen exact candidate. Until then, `PUBLICATION_AUTHORIZED=false`.
