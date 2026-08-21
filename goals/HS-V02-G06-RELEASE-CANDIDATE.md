# HS-V02-G06 — v0.2 Release Candidate

## Status

PLANNED after accepted G05. Publication is not authorized by this goal.

## Objective

Settle one exact-head HookStat v0.2 release candidate proving the shared terminal experience, bilingual Human interface, display identity, diagnostics, reliability intelligence, privacy, and unchanged core safety boundaries on Windows and Linux.

## Scope

- Close all accepted G00-G05 requirements and documentation against one candidate head.
- Verify the Jerry Terminal UI System conformance matrix across views, states, sizes, themes, input, refresh, and terminal cleanup.
- Verify `en-US`/`zh-CN`, preference safety, Human identity migration, diagnostics privacy, and intelligence semantics.
- Run format, Clippy warnings-as-errors, all targets/tests locked, locked build, package metadata, `cargo package`, and `cargo publish --dry-run` as required by the release gate.
- Run exact-head hosted Windows and Linux CI.
- Run one representative owned Windows Terminal smoke for navigation, async refresh, bilingual rendering, narrow/resize behavior, and clean exit.
- Prove v0.1 invariants: normal `codex`, no daemon/wrapper, opt-in instrumentation only, no trust bypass, no raw payload persistence, coverage truthfulness, and sample-count denominators.
- Prepare changelog/README/help and a sanitized final receipt.

## Non-goals

- Do not publish to crates.io, create/push a release tag, or create a public GitHub Release without a separate explicit Owner release gate naming the exact version and candidate.
- Do not add DeepSeek Harness, OpenCode, Claude Code, cloud sync, Web UI, remote telemetry, daemon, or AI diagnosis.
- Do not widen instrumentation/trust/restore behavior to satisfy UI acceptance.
- Do not use synthetic fixtures as a substitute for any expressly required real-terminal or real-runtime claim.

## Acceptance criteria

```text
TUI_SYSTEM_PASS=true
I18N_PASS=true
DISPLAY_IDENTITY_PASS=true
DIAGNOSTICS_PASS=true
RELIABILITY_INTELLIGENCE_PASS=true
REGRESSION_PASS=true
WINDOWS_PASS=true
LINUX_PASS=true
WINDOWS_TERMINAL_SMOKE=PASS
PRIVACY_PASS=true
V01_BEHAVIOR_UNCHANGED=true
NORMAL_CODEX_LAUNCH=codex
DAEMON_REQUIRED=false
TRUST_BYPASS=false
RAW_PAYLOAD_PERSISTED=false
FORMAT=PASS
CLIPPY=PASS
TESTS_LOCKED=PASS
BUILD_LOCKED=PASS
CARGO_PACKAGE=PASS
CARGO_PUBLISH_DRY_RUN=PASS
HOSTED_WINDOWS_CI=PASS
HOSTED_LINUX_CI=PASS
PUBLICATION_AUTHORIZED=false
```

## Dependencies

- Accepted `HS-V02-G00-TUI-FOUNDATION`
- Accepted `HS-V02-G01-RELIABILITY-CENTER-TUI`
- Accepted `HS-V02-G02-HUMAN-IDENTITY`
- Accepted `HS-V02-G03-I18N`
- Accepted `HS-V02-G04-DIAGNOSTICS`
- Accepted `HS-V02-G05-RELIABILITY-INTELLIGENCE`
- `dev_governance_files/QUALITY_GATES.md`
- `dev_governance_files/EVIDENCE_CONTRACT.md`
- A separate Owner authorization for any actual publication after RC acceptance

## Next

Owner release-gate decision for the exact accepted v0.2 candidate; publication remains `NOT_AUTHORIZED` until then.
