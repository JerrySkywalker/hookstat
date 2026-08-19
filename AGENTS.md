# HookStat Agent Instructions

These rules apply to the entire repository.

## Product authority

HookStat is reliability analytics for hooks across coding-agent runtimes. v0.1 implements only Codex, but domain and adapter boundaries must not encode Codex as the universal runtime model.

The primary v0.1 user value is: after ordinary `codex` usage, `hookstat` reports each observable hook handler's historical invocation count, failure count, and failure rate.

## Hard v0.1 invariants

- Normal Codex launch remains `codex`; never require `hookstat codex`.
- No daemon/service is required for v0.1.
- Do not mutate live Codex config, hook definitions, trust state, or session history.
- Do not install wrappers around the user's hooks to manufacture observability.
- Do not report incomplete coverage as `0.00% healthy`.
- `Blocked`/`Stopped`/policy denial are not automatically execution failures.
- Every displayed failure rate must be accompanied by its sample count.
- Do not persist raw prompts or tool payloads.
- No network telemetry in v0.1.

## Architecture

Start as one publishable Rust package (modular monolith). Preserve explicit concepts for Runtime, EvidenceSource, HookInvocation, Coverage, HandlerIdentity, and runtime-native status normalization. Add a workspace only when a real second-runtime implementation justifies it.

## Governance

`dev_governance_files/ROADMAP.md` defines order. `QUALITY_GATES.md` defines required proof. `FAST_LANE.md` prevents redundant ceremony. Each `goals/` file is an execution contract, not a suggestion.

If HS-G01 cannot prove a durable per-handler Codex evidence source, stop at `BLOCKED_DATA_SOURCE_DECISION_REQUIRED`. Do not solve that blocker by silently adding a daemon, wrapper, App Server dependency, live config mutation, or other product-scope expansion.

## Git and scope

- Never force-push `main` or rewrite published history.
- Prefer one train branch for unattended work and push checkpoints after meaningful milestones.
- Do not touch unrelated repositories or user configuration.
- Before release/publication, require an explicit release gate; repository development does not authorize crates.io publication.

## Validation

During iteration: `cargo fmt` plus focused tests. At a settled code candidate: formatting, Clippy warnings-as-errors, tests, locked build, and the active risk-specific gates. Do not rerun unchanged evidence merely because HEAD advanced.
