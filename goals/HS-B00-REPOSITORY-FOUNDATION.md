# HS-B00 — Repository Foundation

## Goal

Materialize a governed, compilable HookStat repository baseline from the initial README-only remote.

## Authorized scope

Rust dependency-free skeleton, pinned toolchain, CI, license/metadata, governance, roadmap, architecture/ADRs, normative TUI spec, and future goal contracts. No runtime evidence parser, SQLite, analytics, Ratatui, or release publication.

## Acceptance

- Rust 1.97.1 and Cargo `rust-version` agree.
- `cargo fmt --check`, Clippy warnings-as-errors, tests and locked build pass.
- Hosted Windows/Linux CI checks the exact checkout SHA.
- Product invariants/non-goals and G01 stop gate are committed.
- TUI baseline is committed as normative design.
- main remains linear; no unrelated drift.

## Receipt

```text
GOAL_ID=HS-B00
START_MAIN=7494b4f85d1851347b41931c6bbb403896d3851b
FINAL_HEAD=<...>
LOCAL_VALIDATION=<...>
HOSTED_CI=<...>
IMPLEMENTATION_STARTED=false
```
