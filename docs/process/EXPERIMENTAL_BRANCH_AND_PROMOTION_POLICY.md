# Experimental Branch and Promotion Policy

## Purpose

HookStat now has two different kinds of future work:

1. product work with a clear intention to ship;
2. exploratory runtime work whose purpose is to discover whether and how a capability should become productized.

These must not share the same branch semantics.

The repository keeps `main` as the only production truth and uses short-lived branches. This policy does **not** introduce a permanent GitFlow-style `develop` branch.

## Branch namespaces

### `main`

`main` contains only accepted, reviewed, release-quality repository state.

```text
MERGE_TARGET=none
PRODUCTION_TRUTH=true
EXPERIMENTAL_CONTENT=false unless explicitly promoted and production-qualified
```

Published tags/releases are cut from accepted `main` according to the release gate.

### `agent/*` — product implementation with merge intent

Use for planned product goals that are expected to merge when their acceptance gates pass.

Examples:

```text
agent/hs-v040-g40-rebaseline-hooks-control-center-001
agent/hs-v040-g41-live-runtime-hook-catalog-001
agent/hs-v040-g42-hooks-control-center-001
agent/hs-v040-g43-human-reliability-001
```

Properties:

```text
BASE=current accepted main
MERGE_INTENT=true
MERGE_TARGET=main
LIFETIME=short
QUALITY_GATES=production changed-risk gates
```

### `fix/*` — narrow production correction

Use for a defect in released/accepted behavior that should be fixed independently of a larger feature train.

Examples:

```text
fix/hs-v031-metric-scope-correctness-001
fix/hs-v040-hook-catalog-join-001
```

A correctness/security/privacy defect must not be hidden inside a long feature train merely to avoid a maintenance release decision.

Properties:

```text
BASE=current accepted main
MERGE_INTENT=true
MERGE_TARGET=main
SCOPE=narrow defect
```

### `exp/*` — exploration with no direct merge intent

Use for uncertain runtime/capability research.

Examples:

```text
exp/deepseek-hook-surface
exp/opencode-plugin-surface
exp/claude-hook-surface
```

Properties:

```text
MERGE_INTENT=false
MERGE_TARGET=NONE
PUBLIC_RELEASE_BLOCKER=false
FAILED_EXPERIMENT_ALLOWED=true
ARCHITECTURE_MAY_CHANGE=true
```

An experiment can prototype code, fixtures, adapters, or tooling, but the branch itself is never assumed to be production-quality.

### `promote/*` — productization of proven experiment results

A successful experiment is **promoted, not merged**.

Create a fresh branch from the latest accepted `main`:

```text
promote/deepseek-adapter-001
promote/opencode-adapter-001
```

Then port only the minimal proven design/code needed for production.

Properties:

```text
BASE=current accepted main
SOURCE_EVIDENCE=one or more exp/* tracks
MERGE_INTENT=true
MERGE_TARGET=main
QUALITY_GATES=full production/admission gates
```

## Why no permanent `dev` or `exp` integration branch

A permanent `develop` branch would duplicate the repository's existing goal-based workflow and create another long-lived truth that must be reconciled with `main`.

A permanent `exp` integration branch would combine unrelated failed/unfinished research and make promotion boundaries ambiguous.

Instead:

```text
main = accepted truth
agent/* = product work
fix/* = production repair
exp/* = bounded research
promote/* = deliberate productization
```

## Allowed synchronization direction

Experiments should periodically absorb relevant accepted product changes.

Allowed:

```text
main ─────→ exp/deepseek
main ─────→ exp/opencode
```

This may be merge or rebase according to branch ownership and repository rules, but must not rewrite published history.

Direct reverse integration is prohibited:

```text
exp/* ──X──→ main
```

The reverse path is:

```text
exp/* evidence
   ↓
Owner/product promotion decision
   ↓
promote/* from current main
   ↓
minimal implementation
   ↓
production CI/review/dogfood/admission
   ↓
main
```

## Experiment lifecycle

Every significant experiment should have a durable experiment note under a suitable `experiments/` or `docs/experiments/` path on the experiment branch.

Standard states:

```text
EXPERIMENT_STARTED
SURFACE_DISCOVERED
CAPABILITY_MAPPED
FIXTURES_PASS
REAL_OWNER_PROOF
CONFORMANT
PROMOTION_READY
```

Valid terminal non-success states:

```text
UPSTREAM_UNSUITABLE
ABANDONED
DEFERRED
```

An experiment is not required to end in production code. Proving a path unsuitable is useful output.

## Required experiment record

At minimum record:

```text
EXPERIMENT_ID
RUNTIME_OR_CAPABILITY
BASE_MAIN
UPSTREAM_VERSION_OR_SOURCE_PIN
QUESTION
OBSERVED_SURFACES
CAPABILITY_MATRIX
PRIVACY_BOUNDARY
PERFORMANCE_CHARACTERISTICS where relevant
OWNER_PROOF_STATE
LIMITATIONS
DISPOSITION
PROMOTION_RECOMMENDATION
```

Do not include private prompt/tool/session payloads or credentials.

## Production release interaction

Exploration does not block product releases unless an Owner explicitly promotes the experiment into the product critical path.

For v0.4:

```text
DEEPSEEK_EXPERIMENT_BLOCKS_V040=false
OPENCODE_EXPERIMENT_BLOCKS_V040=false
CLAUDE_EXPERIMENT_BLOCKS_V040=false
AGY_EXPERIMENT_BLOCKS_V040=false
```

The v0.4 product branch may continue while experiments run independently.

## Shared-core discoveries

An experiment may discover a genuine core limitation that affects product architecture.

Do not merge the experiment merely because the finding is important.

Use one of:

1. create an `agent/*` architecture goal from `main` if it is planned product evolution;
2. create a `fix/*` branch if it is a released correctness defect;
3. create a `promote/*` branch if the change is specifically part of productizing that experiment.

Reference the experiment evidence in the new production branch/PR.

## Experimental directory guidance

Prefer isolating exploratory code under:

```text
experiments/<runtime-or-capability>/
```

when possible.

If a prototype must modify production modules to answer the research question, that is allowed on `exp/*`, but those diffs are still not directly mergeable. Promotion starts from clean current `main` and re-implements/cherry-picks only reviewed minimal changes.

## Experiment CI

Experiments use the smallest proof needed to answer their research question. They are not required to run every production release gate on every iteration.

However:

- they must not claim production admission from fixture-only evidence;
- a performance claim requires reproducible measurement;
- a security/privacy claim requires appropriate review;
- `PROMOTION_READY` requires a clear list of production gates still needed.

## Promotion gate

Before creating `promote/*`, require:

```text
CAPABILITY_MAPPED=true
CORE_SEMANTICS_COMPATIBLE=true|explicit_change_required
PRIVACY_MODEL_KNOWN=true
REAL_OWNER_PROOF=PASS_OR_EXPLICITLY_NOT_REQUIRED
PROMOTION_SCOPE_DEFINED=true
```

Promotion does not inherit experiment acceptance automatically. The production branch must re-run the relevant final gates on the promoted implementation.

## Multi-runtime versioning policy

Do not preassign production versions to experiments.

Forbidden planning assumption:

```text
v0.5 = DeepSeek merely because exp/deepseek exists
v0.6 = OpenCode merely because exp/opencode exists
```

Instead, whichever experiment first reaches `PROMOTION_READY` and is approved for productization may inform the next production roadmap/version.

## Branch cleanup

After an experiment is terminal:

- preserve its durable evidence/PR/history as needed;
- it may be closed without merge;
- do not delete published evidence merely to reduce branch count;
- local worktrees/build artifacts may be removed only under normal ownership/safety rules.

## Summary

```text
                           main
                            │
              ┌─────────────┴─────────────┐
              ▼                           ▼
          Product                     Exploration
          agent/*                      exp/*
          fix/*                          │
              │                          ▼
              ▼                   capability evidence
             main                         │
              ▲                           ▼
              └──────── promote/* ◄───────┘
```

The essential rule is:

> Experiments produce evidence. Promotion produces product code.
