# HookStat v0.3.0 Release Notes

## Status

`v0.3.0` is the Owner-authorized public release. The release closure binds the
frozen exact commit to its crates.io package, Git tag, and GitHub Release.

## Highlights

- Codex Changes workbench with coverage-aware regression, recovery, new-hook,
  and revision history views derived solely from admitted evidence.
- Hook Catalog with Human aliases, revision metadata, selected-period sample
  confidence, freshness, compact trends, and bounded failure exploration.
- Shared `terminal-ui-contract` `=0.1.0` implementation boundary for the
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

`HS-V03-PUBLIC-RELEASE-OWNER-GATE` authorizes publication of the frozen exact
release. This document records the public release scope; immutable release
surfaces provide the final closure evidence.
