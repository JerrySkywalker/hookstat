# HS-G40 v0.4 rebaseline audit

## Release and roadmap authority

HookStat v0.3.1 is closed as a public release. The closeout changes active
governance and stale public text only; it does not alter historical receipts.

```text
PUBLIC_VERSION=0.3.1
PUBLIC_MAIN=651620cbc9f204f312fc31efee424c747895927a
PUBLIC_TAG=v0.3.1
PUBLIC_RELEASE=true
V031_GOVERNANCE_CLOSED=true
README_PUBLIC_VERSION=0.3.1
V040_ROADMAP_AUTHORITY=true
```

`ROADMAP_V031.md` remains the historical v0.3.1 execution record. `ROADMAP_V040.md`
is the current product authority. The branch roles and promotion constraints
are governed by
[`EXPERIMENTAL_BRANCH_AND_PROMOTION_POLICY.md`](../process/EXPERIMENTAL_BRANCH_AND_PROMOTION_POLICY.md):

```text
EXPERIMENT_BRANCH_POLICY=PASS
EXP_DIRECT_MAIN_MERGE=false
PROMOTION_NOT_MERGE=true
G41_G46R_GOALS_DEFINED=true
```

## Human-time audit

Normal Human TUI rendering currently writes raw Unix milliseconds in
`src/tui/rendering.rs`. These are presentation defects, not evidence-schema
changes. The locations audited on the G40 base are:

| Area | Raw field(s) | Rendering location |
| --- | --- | --- |
| Change detail | `first_seen_unix_ms`, `last_seen_unix_ms`, `latest_evidence_unix_ms` | 773, 779, 785 |
| Revision timeline | epoch first/last boundaries | 818-819 |
| Recent failure | invocation timestamp | 1164-1166 |
| Hook catalog history | first seen, last seen, latest evidence, freshness | 1192, 1194, 1196, 1202 |
| Hook detail fingerprint occurrences | first/latest occurrence | 1218, 1220 |
| Failure-cluster overview/table | latest and first/latest occurrence | 1338, 1380-1381 |
| Failure-cluster detail | first/latest/freshness | 1467, 1473, 1479 |

```text
HUMAN_TIME_AUDIT=PASS
RAW_UNIX_MILLISECONDS_IN_NORMAL_TUI=false (G43 acceptance requirement)
```

G43 must introduce one reusable, deterministic Human formatter. Normal TUI
output must use localized wall-clock datetime plus useful relative age, for
example `2026-08-30 09:22 (12 minutes ago)`. Machine JSON and deliberate debug
surfaces may retain epoch values. The formatter must be tested at relative-age
boundaries and across supported locale behavior; it must not depend on an
uncontrolled host locale for snapshot tests.

## Metric-scope audit

The observed pattern such as `runs = 5`, `failure sample = 0`, and a `7-day`
trend sample of `227` is not an analytics data-correctness defect. It is a
legitimate multiple-population presentation that the current TUI fails to
label clearly enough.

Trace on the G40 base:

| Layer | Observed population |
| --- | --- |
| `Ledger::invocations_for_reliability` | Bounded, supplied reliability working set |
| `instrumented_report` / aggregate | Selected window, all revisions for the handler key |
| `ReliabilityCenterViewModel::hook_detail` | Copies selected aggregate runs and terminal denominator |
| `reliability_intelligence` trend projections | Each trend's own window, all revisions in supplied data |
| `enrich_report_from_ledger` all trend | Exact all-time aggregate from the ledger |
| `revision_epoch_metrics` / revision comparison | All observed time for the contiguous current/previous revision epoch |

```text
METRIC_SCOPE_ROOT_CAUSE=LEGITIMATE_MULTIPLE_POPULATIONS_WITH_UNLABELED_PRESENTATION
METRIC_SCOPE_AUDIT=PASS
CORRECTNESS_DEFECT=false
PRESENTATION_DEFECT=true
METRIC_FIX_REQUIRED=false
```

G43 must preserve the analytics and make the population explicit. Frozen
minimum labels are:

| Value | Required Human scope label |
| --- | --- |
| Detail summary runs/failures | `Selected <period>, all revisions` |
| Failure-rate denominator | `Terminal samples in selected scope` |
| Trend | `<trend window>, all revisions` |
| Current revision comparison | `All observed time, current revision` |
| Previous revision comparison | `All observed time, previous revision` |

If a future audit proves values from the same stated population disagree, route
that through a separate `fix/*` correctness train rather than conceal it with
presentation wording.

## Runtime-presentation privacy audit

The current canonical domain deliberately uses privacy-safe handler identity
and structural/fingerprint data. The ledger schema and insert path persist
identity/reliability values, not raw command, matcher, source-path, prompt, or
tool-payload presentation text. The v0.4 architecture freezes a separate
ephemeral current-runtime snapshot for values that must be shown to a Human.

```text
RUNTIME_PRESENTATION_PRIVACY_CONTRACT=PASS
RUNTIME_PRESENTATION_IN_MEMORY_ONLY=true
RAW_COMMAND_LEDGER_PERSISTENCE=false
RAW_MATCHER_LEDGER_PERSISTENCE=false
RAW_SOURCE_PATH_LEDGER_PERSISTENCE=false
RAW_RUNTIME_PRESENTATION_RECEIPT_WRITES=0
RUNTIME_PRESENTATION_DIAGNOSTICS_EXPORT=false
NO_THIRD_EVIDENCE_PATH=true
```

The zero write assertions are G41 implementation gates: G41 must add tests
that prove its snapshot cannot enter the ledger, receipts, diagnostics export,
or network egress. This G40 audit does not claim such an implementation exists
yet.

## G40 conclusion

```text
CODEX_HOOKS_SOURCE_PINNED=true
CODEX_HOOKS_PARITY_MATRIX_COMPLETE=true
EVENT_SURFACE=PINNED_CATALOG_WITH_INTERRUPT_AND_FUTURE_EVENT_PRESERVATION
INTERRUPT_RUNTIME_CATALOG_VISIBLE=true
INTERRUPT_CANONICAL_RELIABILITY=UNPROVEN
PRODUCT_SRC_CHANGED=false
```

The next DAG node after G40 acceptance is G41, Live Runtime Hook Catalog.
