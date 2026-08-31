# HS-G46R — v0.4 Hardening & Release

## Objective

Freeze, qualify, package, and prepare the exact HookStat v0.4 release candidate only after the full product track, the G45V visual-correctness correction train, and the Owner re-dogfood gate are accepted.

Use the v0.3.1 Fast Lane. Do not recreate candidate churn or serial acceptance ceremony.

## Preconditions

```text
G40=PASS
G41=PASS
G42=PASS
G43=PASS
G44=PASS_OR_TRUTHFULLY_UPSTREAM_UNAVAILABLE
G45_AUTOMATED_PREPARATION=PASS
G45V_A=PASS
G45V_B=PASS
G45V_C=PASS
G45_OWNER_REDOGFOOD=PASS
```

The first G45 Owner visual pass failed with finding `G45-OV-001`; that failure is historical evidence and may not be overwritten. G46R is forbidden until the correction train and re-dogfood close it.

## Architecture freeze

After G45R acceptance:

```text
NEW_PRODUCT_FEATURE=false
NEW_RUNTIME=false
EXPERIMENT_PROMOTION=false
NEW_VISUAL_ARCHITECTURE=false
```

Allowed work:

```text
release defects
packaging defects
documentation defects
upgrade/fresh-install defects
privacy/security blockers
CI regressions
visual-baseline defects that prove a release regression
```

DeepSeek/OpenCode/other `exp/*` work is explicitly outside this train.

## Candidate lifecycle

Settle all product code, visual baselines, and release documentation before freezing.

```text
CANDIDATE_SHA=<exact SHA>
CANDIDATE_FROZEN=true
POST_FREEZE_CODE_COMMITS=0
POST_FREEZE_DOC_COMMITS=0
POST_FREEZE_BASELINE_COMMITS=0
POST_FREEZE_RECEIPT_COMMITS=0
```

Any real defect creates a new candidate SHA. Acceptance evidence is attached through PR comments/artifacts or other non-mutating exact-SHA receipts.

## Parallel acceptance DAG

```text
                       candidate freeze
                             │
       ┌─────────────────────┼─────────────────────┐
       ▼                     ▼                     ▼
 ordinary hosted CI      CI / tui-visual      independent review
       │                     │                     │
       └─────────────────────┼─────────────────────┘
                             ▼
                    release orchestrator
                             │
                             ▼
                       merge decision
```

Owner G45R evidence is a precondition and does not need to be regenerated after candidate freeze unless the candidate changes a Human-visible product surface.

## Required code/product gates

At exact candidate:

```text
CODEX_HOOKS_INFORMATION_PARITY=PASS
LIVE_RUNTIME_HOOK_CATALOG=PASS
EVENT_DISPLAY_IDENTITY_DUPLICATES=0
KNOWN_EVENT_LOCALIZATION=PASS
REAL_WIRE_TO_FRAME_E2E=PASS
TUI_VISUAL_REGRESSION_CI=PASS
TUI_GOLDEN_BASELINES=PASS
TUI_STRUCTURAL_INVARIANTS=PASS

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

provided read/information parity is complete. The accepted G44 disposition for the current v0.4 train is `UPSTREAM_UNAVAILABLE` unless a separately admitted correction changes that fact before architecture freeze.

## Visual gate

Run the dedicated G45V-B visual regression gate at the exact release candidate.

Required:

```text
CI_TUI_VISUAL=PASS
BASELINE_DIFF_UNREVIEWED=false
STRUCTURAL_VISUAL_INVARIANTS=PASS
OWNER_PRIVATE_VISUAL_DATA=0
```

Do not regenerate snapshots merely to make the release candidate green. Any intentional baseline change is a product diff and creates a new candidate SHA requiring fresh review.

## Release gate

Extend/reuse the existing exact-SHA release orchestrator rather than manually repeating package proof.

Require at minimum:

```text
VERSION=0.4.0
WINDOWS_CI=PASS
UBUNTU_CI=PASS
TUI_VISUAL_CI=PASS
PACKAGE=PASS
PUBLISH_DRY_RUN=PASS
FRESH_INSTALL=PASS
UPGRADE_FROM_PUBLIC_BASELINE=PASS
LEGACY_EVIDENCE_PRESERVED=PASS
REPORT_SMOKE=PASS
DOCTOR_SMOKE=PASS
TUI_DETERMINISTIC_TESTS=PASS
TUI_GOLDEN_BASELINES=PASS
REAL_WIRE_TO_FRAME_E2E=PASS
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
- deterministic TUI visual-regression coverage at release;
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
EVENT_DISPLAY_IDENTITY_DUPLICATES=0
KNOWN_EVENT_LOCALIZATION=PASS
REAL_WIRE_TO_FRAME_E2E=PASS
TUI_VISUAL_REGRESSION_CI=PASS
TUI_GOLDEN_BASELINES=PASS
TUI_STRUCTURAL_INVARIANTS=PASS
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
