# HS-V02-G05 — Reliability Intelligence

## Status

PLANNED after accepted G04.

## Objective

Deepen reliability interpretation with sample-aware trends, risk, bounded failure fingerprints, and revision comparison while preserving evidence coverage and runtime-neutral analytics.

## Scope

- Add 7-day and 30-day trend projections with deterministic bucket/window semantics.
- Detect regressions only when the comparison denominator and coverage are sufficient; otherwise report insufficient evidence.
- Define a documented risk score using failure rate, sample count, trend, impact/category, and coverage rather than percentage alone.
- Group failures by the existing bounded taxonomy and admitted metadata such as exit code category, platform, shell classification, and error category; never ingest streams to improve a fingerprint.
- Add current/previous revision comparison based on stable handler key plus revision and a proven configuration timeline.
- Surface intelligence in Overview, Hooks, and Hook Detail without hiding base counts/rates.
- Version analytics/report schema changes and add deterministic fixtures for low-sample, incomplete, revision-boundary, and equal-rate cases.

## Non-goals

- Do not add AI-generated root-cause analysis.
- Do not inspect or persist stderr/stdout, prompts, tool payloads, raw commands, or arbitrary error text.
- Do not rank only by failure percentage.
- Do not invent a previous revision or trend when history is absent.
- Do not change runtime instrumentation, trust, `hooks.json`, proxy behavior, or add another runtime.
- Do not add network telemetry or cloud analysis.

## Acceptance criteria

```text
TREND_7D=PASS
TREND_30D=PASS
REGRESSION_DENOMINATOR_PROVEN=true
LOW_SAMPLE_UNCERTAINTY_VISIBLE=true
RISK_SCORE_DOCUMENTED=true
RISK_SCORE_PERCENTAGE_ONLY=false
RISK_SCORE_COVERAGE_AWARE=true
FAILURE_FINGERPRINT_BOUNDED=true
RAW_ERROR_STREAM_INSPECTED=false
REVISION_COMPARISON=PASS
REVISION_IDENTITY_PROVEN=true
INVENTED_PREVIOUS_REVISION=false
FAILURE_RATE_WITH_SAMPLE_COUNT=true
RUNTIME_NEUTRAL_ANALYTICS=true
REPORT_SCHEMA_VERSIONED=true
PRIVACY_GATE=PASS
```

## Dependencies

- Accepted `HS-V02-G04-DIAGNOSTICS`
- Accepted G02 identity/revision schema
- Existing v0.1 analytics denominator and terminal-status rules
- `dev_governance_files/HOOKSTAT_V02_ROADMAP.md`

## Next

`HS-V02-G06 — Release Candidate`.
