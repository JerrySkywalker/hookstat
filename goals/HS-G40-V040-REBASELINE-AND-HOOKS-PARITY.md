# HS-G40 — v0.4 Rebaseline & Codex `/hooks` Parity Contract

## Objective

Close the post-v0.3.1 governance tail and freeze the v0.4 Hooks Control Center product contract before implementation.

This goal is primarily governance/specification plus narrow correctness audit. It must not begin broad G41/G42 implementation.

## Starting point

```text
PUBLIC_VERSION=v0.3.1
PUBLIC_MAIN=651620cbc9f204f312fc31efee424c747895927a
V031_RELEASED=true
V040_PRODUCT_THEME=Hooks Control Center
```

## Required work

1. Mark v0.3.1 as publicly released/closed in active governance.
2. Correct stale README/public-version/release-candidate wording.
3. Pin one exact official Codex source/version baseline for `/hooks` parity.
4. Produce a field/capability parity matrix against the pinned baseline.
5. Audit the current HookStat Human interface for raw machine values, especially Unix millisecond timestamps.
6. Audit metric scope consistency, including any case where top-level/current-revision counts differ from trend/all-revision sample counts without labels.
7. Freeze the ephemeral runtime presentation architecture.
8. Freeze branch roles: `main`, `agent/*`, `fix/*`, `exp/*`, `promote/*`.
9. Update top-level roadmap authority so v0.4 product work is active and runtime experiments are independent.

## Codex `/hooks` baseline audit

Inspect the exact current official Codex implementation and record at minimum:

```text
CODEX_VERSION_OR_SOURCE_PIN=
HOOKS_BROWSER_SOURCE=
HOOKS_LIST_PROTOCOL_SOURCE=
EVENT_SURFACE=
HANDLER_FIELD_SURFACE=
READ_CAPABILITIES=
WRITE_CAPABILITIES=
```

The parity floor must include event-level Installed/Active/Review/Description and handler-level Event/Matcher/Source/Handler Type/Command or MCP/Mode/Timeout/Context/Trust plus enabled/managed/review state where exposed.

Do not infer future API stability from one source pin.

## Metric consistency audit

Investigate the currently observed UX pattern where one Hook detail may show values similar to:

```text
runs=5
failure sample=0
7-day trend sample=227
```

Determine whether this is:

- a legitimate difference between current revision and all-revision history;
- a selected-window versus all-time difference;
- a view-model/render labeling defect;
- or a real analytics/data correctness defect.

Required disposition:

```text
METRIC_SCOPE_ROOT_CAUSE=
CORRECTNESS_DEFECT=true|false
PRESENTATION_DEFECT=true|false
```

If a correctness defect exists, open a separate `fix/*` train and do not bury it inside future v0.4 UI work.

## Human-time audit

Find every normal TUI rendering of Unix milliseconds and classify it. At minimum inspect:

```text
first seen
last seen
latest evidence
recent failure times
revision timeline boundaries
change occurrence times
fingerprint occurrence times
```

Freeze a local-time + relative-time presentation contract for G43.

## Privacy boundary

Confirm the runtime presentation snapshot can show current runtime-owned values locally while preserving:

```text
RAW_COMMAND_PERSISTED_IN_LEDGER=false
RAW_MATCHER_PERSISTED=false
RAW_SOURCE_PATH_PERSISTED_FOR_PRESENTATION=false
RUNTIME_PRESENTATION_DIAGNOSTICS_EXPORT=false
NO_THIRD_EVIDENCE_PATH=true
```

## Branch governance

Adopt `docs/process/EXPERIMENTAL_BRANCH_AND_PROMOTION_POLICY.md`.

Required semantics:

```text
main=production truth
agent/*=product merge intent
fix/*=narrow production repair
exp/*=no direct merge intent
promote/*=productization from latest main
```

No permanent `develop` or permanent integration `exp` branch is introduced.

## Deliverables

At minimum settle:

- `dev_governance_files/ROADMAP_V040.md`;
- `docs/design/HOOKS_CONTROL_CENTER_SPEC.md`;
- `docs/architecture/RUNTIME_PRESENTATION_SNAPSHOT.md`;
- `docs/process/EXPERIMENTAL_BRANCH_AND_PROMOTION_POLICY.md`;
- G41–G46R goal contracts;
- top-level roadmap/agent guidance;
- README v0.3.1 public-state correction;
- exact parity audit receipt/matrix.

## Quality gates

This is docs/governance unless the audit discovers and separately authorizes a code fix.

Use the Fast Lane docs path. Do not run full Rust gates merely because governance text changes.

If README/build metadata only changes as text, use the classified lightweight path.

## Acceptance

```text
V031_GOVERNANCE_CLOSED=true
README_PUBLIC_VERSION=0.3.1
V040_ROADMAP_AUTHORITY=true
CODEX_HOOKS_SOURCE_PINNED=true
CODEX_HOOKS_PARITY_MATRIX_COMPLETE=true
RUNTIME_PRESENTATION_PRIVACY_CONTRACT=PASS
METRIC_SCOPE_AUDIT=PASS
HUMAN_TIME_AUDIT=PASS
EXPERIMENT_BRANCH_POLICY=PASS
G41_G46R_GOALS_DEFINED=true
PRODUCT_SRC_CHANGED=false unless a separately admitted fix train exists
```

## Next

Begin G41 from current accepted main after G40 merges.

Exploration tracks may begin independently on `exp/*`; they do not block G41–G46R.
