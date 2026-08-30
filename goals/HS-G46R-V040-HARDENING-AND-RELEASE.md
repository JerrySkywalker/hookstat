# HS-G46R — v0.4 Hardening & Release

## Objective

Freeze, qualify, package, and prepare the exact HookStat v0.4 release candidate after G40–G45 are accepted.

Use the v0.3.1 Fast Lane. Do not recreate candidate churn or serial acceptance ceremony.

## Preconditions

```text
G40=PASS
G41=PASS
G42=PASS
G43=PASS
G44=PASS_OR_TRUTHFULLY_UPSTREAM_UNAVAILABLE
G45=PASS
```

## Architecture freeze

After G45 acceptance:

```text
NEW_PRODUCT_FEATURE=false
NEW_RUNTIME=false
EXPERIMENT_PROMOTION=false
```

Allowed work:

```text
release defects
packaging defects
documentation defects
upgrade/fresh-install defects
privacy/security blockers
CI regressions
```

DeepSeek/OpenCode/other `exp/*` work is explicitly outside this train.

## Candidate lifecycle

Settle all product code and release documentation before freezing.

```text
CANDIDATE_SHA=<exact SHA>
CANDIDATE_FROZEN=true
POST_FREEZE_CODE_COMMITS=0
POST_FREEZE_DOC_COMMITS=0
POST_FREEZE_RECEIPT_COMMITS=0
```

Any real defect creates a new candidate SHA. Acceptance evidence is attached through PR comments/artifacts or other non-mutating exact-SHA receipts.

## Parallel acceptance DAG

```text
                 candidate freeze
                       │
       ┌───────────────┼───────────────┐
       ▼               ▼               ▼
   Hosted CI     Independent review  Owner A/B
       │               │               │
       └───────────────┼───────────────┘
                       ▼
                 Release gate
                       │
                       ▼
                 merge decision
```

Run independent gates concurrently when their dependencies allow.

## Required code/product gates

At exact candidate:

```text
CODEX_HOOKS_INFORMATION_PARITY=PASS
LIVE_RUNTIME_HOOK_CATALOG=PASS
INSTALLED_UNOBSERVED_VISIBLE=true
HISTORICAL_NOT_INSTALLED_DISTINCT=true
UNKNOWN_RUNTIME_EVENTS_VISIBLE=true
RAW_UNIX_MILLISECONDS_IN_NORMAL_TUI=false
ZERO_SAMPLE_HEALTHY_PERCENT=false
METRIC_SCOPE_CONSISTENCY=PASS
COVERAGE_EXPLANATION=PASS
RISK_EXPLANATION=PASS
RUNTIME_PRESENTATION_PERSISTENCE=0
NO_THIRD_EVIDENCE_PATH=true
```

Safe write parity may be:

```text
PASS
```

or truthful:

```text
UPSTREAM_UNAVAILABLE
```

provided read/information parity is complete.

## Release gate

Extend/reuse the existing exact-SHA release orchestrator rather than manually repeating package proof.

Require at minimum:

```text
VERSION=0.4.0
WINDOWS_CI=PASS
UBUNTU_CI=PASS
PACKAGE=PASS
PUBLISH_DRY_RUN=PASS
FRESH_INSTALL=PASS
UPGRADE_FROM_PUBLIC_BASELINE=PASS
LEGACY_EVIDENCE_PRESERVED=PASS
REPORT_SMOKE=PASS
DOCTOR_SMOKE=PASS
TUI_DETERMINISTIC_TESTS=PASS
OWNER_CODEX_HOOKS_AB_DOGFOOD=PASS
INDEPENDENT_REVIEW=PASS
```

Upgrade proof must preserve v0.3.1 ledger/history/preferences and must not persist ephemeral runtime presentation metadata.

## Documentation

Public documentation must state:

- v0.4 Hooks Control Center product behavior;
- runtime truth versus reliability history distinction;
- human-readable current hook fields;
- local ephemeral command/source/matcher privacy boundary;
- safe write capability and any upstream limitations;
- Codex production support baseline and version/capability caveats;
- experimental runtimes remain non-production unless separately promoted.

## Publication gate

Repository development does not authorize public publication.

Without explicit Owner authorization, do not:

```text
cargo publish
git tag v0.4.0
git push public tag
create GitHub Release
```

## Acceptance

```text
V040_ARCHITECTURE_FROZEN=true
V040_HOOKS_CONTROL_CENTER=PASS
CODEX_HOOKS_INFORMATION_PARITY=PASS
HUMAN_RELIABILITY_PRESENTATION=PASS
PRIVACY=PASS
SECURITY=PASS
WINDOWS_CI=PASS
UBUNTU_CI=PASS
OWNER_DOGFOOD=PASS
INDEPENDENT_REVIEW=PASS
PACKAGE=PASS
PUBLISH_DRY_RUN=PASS
FRESH_INSTALL=PASS
UPGRADE=PASS
CANDIDATE_FROZEN=true
POST_FREEZE_COMMITS=0
PUBLICATION=OWNER_GATE
```

## Next

After public v0.4 publication, rebaseline product work independently from all `exp/*` tracks. Promote an experimental runtime only after it reaches `PROMOTION_READY` and receives an explicit productization decision.
