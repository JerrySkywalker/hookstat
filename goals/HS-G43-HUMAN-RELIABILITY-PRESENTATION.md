# HS-G43 — Human Reliability Presentation

## Objective

Make HookStat reliability information immediately understandable to humans without weakening the underlying analytics semantics.

This goal specifically addresses raw Unix milliseconds, unlabeled metric scopes, zero-sample percentages, opaque risk values, revision/hash readability, and coverage explanations.

## Preconditions

```text
G41=PASS
```

G42 may proceed in parallel once shared view-model contracts are stable.

## Human time

Replace normal-TUI raw epoch output with localized Human time.

Required fields include:

```text
first seen
last seen
latest evidence
recent failure time
revision timeline boundaries
change event time
fingerprint first/latest occurrence
```

Normal TUI:

```text
RAW_UNIX_MILLISECONDS=false
```

Machine JSON/debug surfaces may retain raw values.

Prefer local timestamp plus useful relative age, with deterministic formatting tests.

## Metric scope

Define and render explicit scopes so counts from different populations are never visually presented as if identical.

At minimum distinguish:

```text
SELECTED_WINDOW_ALL_REVISIONS
SELECTED_WINDOW_CURRENT_REVISION
ALL_TIME_ALL_REVISIONS
TERMINAL_SAMPLE_DENOMINATOR
```

Audit the current observed pattern where detail header and trend sections can expose different run/sample counts without clear labels.

If the mismatch is a true analytics defect rather than a labeling problem, route the correctness repair through `fix/*` instead of hiding it in presentation code.

## Zero-sample semantics

Forbidden:

```text
0.00% healthy (sample=0)
```

Required concept:

```text
Failure rate: unavailable
Terminal samples: 0
Status: No terminal samples
```

The explanation should tell the user why the denominator is unavailable.

## Coverage explanation

Translate `Complete`, `Partial`, `NotAdmitted`, `Unknown`, etc. into Human explanations that answer what HookStat does and does not know.

Do not turn partial/unknown coverage into a healthy state.

## Risk explanation

Keep the analytics risk score authoritative but present a Human category and reason.

Example:

```text
Low risk (10/100)
Reason: no observed failures; terminal coverage remains incomplete.
```

The reason must derive from bounded existing facts, not AI-generated diagnosis.

## Revision presentation

Use short Human identifiers in primary TUI presentation, with full internal values available only in technical/advanced metadata where appropriate.

Revision timeline should show understandable time ranges, not epoch milliseconds.

## Data freshness

Replace raw `latest_evidence_unix_ms` presentation with a local time and relative freshness state.

Example:

```text
Latest evidence: 2026-08-30 09:22 (12 minutes ago)
```

Freshness thresholds must be documented and not imply runtime health when the evidence source is not admitted.

## Tests

At minimum:

- local time formatting;
- relative age boundaries;
- locale behavior;
- no raw epoch in Human render snapshots;
- zero terminal samples;
- complete/partial/not-admitted explanations;
- current revision vs all-revision metric labeling;
- selected-window vs all-time labeling;
- risk category/reason;
- short/full revision presentation;
- narrow/wide render snapshots.

## Acceptance

```text
RAW_UNIX_MILLISECONDS_IN_NORMAL_TUI=false
ZERO_SAMPLE_HEALTHY_PERCENT=false
METRIC_SCOPE_EXPLICIT=true
METRIC_SCOPE_CONSISTENCY=PASS
COVERAGE_EXPLANATION=PASS
RISK_EXPLANATION=PASS
REVISION_HUMAN_READABLE=true
DATA_FRESHNESS_HUMAN_READABLE=true
AI_DIAGNOSIS_ADDED=false
ANALYTICS_SEMANTICS_WEAKENED=false
CI=PASS
INDEPENDENT_REVIEW=PASS
```

## Next

Converge G42 + G43, then proceed to G44 and G45.
