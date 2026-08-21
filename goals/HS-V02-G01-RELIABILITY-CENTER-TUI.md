# HS-V02-G01 — Reliability Center TUI

## Status

PLANNED after accepted G00.

## Objective

Replace the v0.1 two-screen data viewer with a TabBeacon-compatible Reliability Center that makes coverage, denominators, reliability, and high-risk handlers understandable without changing evidence or instrumentation behavior.

## Scope

- Implement Overview, Hooks, and Hook Detail from `HOOKSTAT_V02_VIEW_MODEL.md`.
- Use the shared title/navigation/content/footer shell and semantic components from G00.
- Build immutable view models outside rendering and off the UI event thread.
- Preserve 24h, 7d, 30d, and All windows.
- Show every failure rate with its terminal sample count.
- Keep incomplete/unknown coverage visibly distinct from healthy zero.
- Keep Blocked and Stopped distinct from execution failures.
- Preserve stable handler selection across refresh and resize.
- Add search/filter UI state and deterministic result ordering.
- Retain accepted data with a stale/error marker after refresh failure.
- Use the display-identity interface; until G02 resolves a friendly name, use an explicit safe fallback without making the internal key the visual title.

## Non-goals

- Do not implement display-identity persistence/migration; G02 owns it.
- Do not complete bilingual coverage or preference persistence; G03 owns it.
- Do not add `hookstat doctor`, diagnostics export, repair, instrumentation apply, or trust actions.
- Do not implement final risk score, failure clustering, or revision comparison; G05 owns them.
- Do not modify receipt, proxy, manifest, Codex configuration, trust, or ledger semantics.
- Do not show unsupported runtimes as fake zero rows.

## Acceptance criteria

```text
RELIABILITY_CENTER_TUI=PASS
OVERVIEW_VIEW=PASS
HOOKS_VIEW=PASS
HOOK_DETAIL_VIEW=PASS
DISPLAY_IDENTITY_BOUNDARY_USED=true
FAILURE_RATE_WITH_SAMPLE_COUNT=true
INCOMPLETE_COVERAGE_HEALTHY_ZERO=false
BLOCKED_STOPPED_AUTO_FAILURE=false
UNSUPPORTED_RUNTIME_PLACEHOLDERS=false
STABLE_SELECTION_AFTER_REFRESH=true
SEARCH_FILTER=PASS
DATABASE_IN_RENDER=false
REFRESH_ERROR_PRESERVES_HISTORY=true
EMPTY_PARTIAL_ERROR_NORMAL_STATES=PASS
NORMAL_NARROW_MINIMUM_BUFFERS=PASS
V01_BEHAVIOR_REGRESSION=PASS
```

## Dependencies

- Accepted `HS-V02-G00-TUI-FOUNDATION`
- `docs/design/HOOKSTAT_V02_VIEW_MODEL.md`
- Existing runtime-neutral `HookInvocation`, analytics, `MachineReport`, and ledger contracts

## Next

`HS-V02-G02 — Human-readable Hook Identity`.
