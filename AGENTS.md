# HookStat Agent Instructions

These rules apply to the entire repository.

## Product authority

HookStat is reliability analytics for hooks across coding-agent runtimes. v0.1 implements only Codex, but domain and adapter boundaries must not encode Codex as the universal runtime model.

The primary v0.1 user value is: after ordinary `codex` usage, `hookstat` reports each observable hook handler's historical invocation count, failure count, and failure rate.

## Hard v0.1 invariants

- Normal Codex launch remains `codex`; never require `hookstat codex`.
- No daemon/service is required for v0.1.
- Normal analysis is read-only toward Codex. An explicit, opt-in per-handler
  instrumentation plan may install a transparent proxy only after dry-run and
  explicit apply; it must never be a `hookstat codex` launcher wrapper, daemon,
  implicit default mutation. `--apply` must never grant trust; an explicit
  `--trust` action may use Codex's official App Server only after it proves the
  exact current HookStat manifest, journal, and supported user-handler targets.
  Unmanaged/owner-live Codex configuration remains untouched during unattended
  development trains unless a goal expressly authorizes bounded activation.
- Do not report incomplete coverage as `0.00% healthy`.
- `Blocked`/`Stopped`/policy denial are not automatically execution failures.
- Every displayed failure rate must be accompanied by its sample count.
- Do not persist raw prompts or tool payloads.
- No network telemetry in v0.1.

## Architecture

Start as one publishable Rust package (modular monolith). Preserve explicit concepts for Runtime, EvidenceSource, HookInvocation, Coverage, HandlerIdentity, and runtime-native status normalization. Add a workspace only when a real second-runtime implementation justifies it.

## Governance

`dev_governance_files/ROADMAP.md` defines order. `QUALITY_GATES.md` defines required proof. `FAST_LANE.md` prevents redundant ceremony. Each `goals/` file is an execution contract, not a suggestion.

Passive durable evidence remains preferred. If it is unavailable, an owner may
explicitly admit an opt-in instrumented receipt source. The canonical model,
ledger, analytics, JSON, and TUI must remain evidence-source-neutral.

## Git and scope

- Never force-push `main` or rewrite published history.
- Prefer one train branch for unattended work and push checkpoints after meaningful milestones.
- Do not touch unrelated repositories or user configuration.
- Before release/publication, require an explicit release gate; repository development does not authorize crates.io publication.

## Validation

During iteration: `cargo fmt` plus focused tests. At a settled code candidate: formatting, Clippy warnings-as-errors, tests, locked build, and the active risk-specific gates. Do not rerun unchanged evidence merely because HEAD advanced.
