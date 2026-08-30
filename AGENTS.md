# HookStat Agent Instructions

These rules apply to the entire repository.

## Product authority

HookStat is local-first reliability analytics and Human hook control/inspection across coding-agent runtimes.

Public baseline at the current rebaseline is HookStat v0.3.1. Codex remains the only production runtime unless a later promoted runtime train is explicitly admitted and released.

The current v0.4 product direction is **Hooks Control Center / Human Usability**:

> Runtime Truth First, Reliability Second.

For Codex hooks, the Human interface must expose at least the human-readable current-runtime information provided by the pinned official `/hooks` baseline, then add HookStat reliability/history/diagnosis as an additive layer.

Runtime-specific experimentation is independent from the v0.4 product critical path.

## Hard invariants

- Normal Codex launch remains `codex`; never require `hookstat codex`.
- No global mandatory daemon/service is required for normal HookStat operation.
- Normal analysis is read-only toward Codex unless an active goal explicitly admits a bounded write action through a proven official runtime route.
- Existing opt-in instrumentation remains explicit, reversible, drift-safe, and separately governed.
- `--apply` never grants trust implicitly. Trust actions require exact supported targets and an official verified route.
- Do not report incomplete, unknown, not-admitted, or zero-terminal-sample coverage as `0.00% healthy`.
- `Blocked`/`Stopped`/policy denial are not automatically execution failures.
- Every displayed failure rate is paired with its terminal sample denominator.
- Do not persist raw prompts or tool payloads.
- Do not persist raw command/matcher/source presentation metadata merely to render the v0.4 TUI; presentation-sensitive runtime catalog data is ephemeral/in-memory unless a separate explicit contract says otherwise.
- No remote/network telemetry by default.
- `NOT_ADMITTED` is a truthful coverage state, not a transport or success state.
- Native and admitted IPC remain the only production evidence transports; do not add a third evidence path casually.
- Managed runtime hooks are never mutated by HookStat through guessed configuration writes.

## Architecture

HookStat remains one publishable modular Rust package unless a real production runtime integration justifies a workspace split.

Preserve explicit concepts for Runtime, EvidenceSource, CanonicalEvidence/HookInvocation, Coverage, HandlerIdentity, authority, and runtime-native normalization.

Current-runtime Human presentation and durable reliability evidence are distinct:

```text
Runtime Presentation Snapshot = ephemeral current runtime truth
Ledger / Analytics            = durable admitted reliability truth
```

Do not persist presentation-sensitive current-runtime fields to improve TUI convenience or joining.

## Governance

`dev_governance_files/ROADMAP.md` defines top-level authority.

For active v0.4 work, `dev_governance_files/ROADMAP_V040.md` and G40–G46R contracts are authoritative.

`QUALITY_GATES.md` defines required proof. `FAST_LANE.md` prevents redundant ceremony. Each `goals/` file is an execution contract, not a suggestion.

Passive/runtime-native durable evidence remains preferred when it can be admitted truthfully. Runtime-specific differences must be absorbed by adapters/capability models rather than contaminating core analytics semantics.

## Branch and exploration policy

See `docs/process/EXPERIMENTAL_BRANCH_AND_PROMOTION_POLICY.md`.

Required branch meanings:

```text
main        = accepted production truth
agent/*     = planned product work with merge intent
fix/*       = narrow released/accepted defect repair
exp/*       = exploration with no direct merge intent
promote/*   = productization of proven experiment results from current main
```

Do not introduce a permanent GitFlow `develop` branch merely to host normal product work.

Do not directly merge `exp/*` into `main`. Experiments produce evidence; promotion produces product code.

DeepSeek Harness, OpenCode, Claude Code, Agy, and other experimental runtime tracks do not block v0.4 unless an Owner explicitly promotes one into the product critical path.

## Git and scope

- Never force-push `main` or rewrite published history.
- Prefer one train branch for unattended product work and push checkpoints after meaningful milestones.
- One writer per branch/worktree where practical.
- Do not touch unrelated repositories or user configuration.
- Preserve foreign worktrees/dirty state; do not use destructive cleanup to simplify a goal.
- Before release/publication, require the explicit release gate. Repository development does not authorize crates.io publication/tag/Release.
- After a release candidate freezes, do not create post-freeze code/doc/receipt commits; bind acceptance evidence to the exact SHA through non-mutating receipts/comments/artifacts.

## Validation

During iteration: format plus focused changed-risk tests.

At a settled code candidate: formatting, Clippy warnings-as-errors, tests, locked build, and active risk-specific gates.

Use the risk-aware Fast Lane. Do not rerun unchanged evidence merely because another independent acceptance gate finishes later.

Human/TUI changes require deterministic render/state tests plus the owner visual/interaction proof named by the active goal.

Runtime information parity claims require an exact upstream source/version pin and a parity matrix; do not claim parity against an unspecified moving runtime.
