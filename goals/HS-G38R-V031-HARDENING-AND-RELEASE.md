# HS-G38R — v0.3.1 Hardening & Release

## Status

PLANNED after accepted G38.

## Objective

Freeze the v0.3.1 architecture, close release blockers, prove upgrade/fresh-install behavior and real Codex operation, then prepare the exact public v0.3.1 candidate. Public publication remains an explicit Owner gate.

## Freeze rule

After G38 acceptance:

```text
NEW_ARCHITECTURE_WORK=false
NEW_PRODUCT_FEATURE=false
NEW_RUNTIME=false
```

Allowed work is limited to:

```text
release blockers
test/CI failures
packaging
documentation
upgrade/fresh-install defects
performance regressions
compatibility defects
privacy/security release blockers
```

Do not use release hardening as permission to begin OpenCode, DeepSeek Harness, or v0.4 work.

## Required release path

From exact accepted main:

```text
freeze exact candidate SHA
set/verify version 0.3.1
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --locked --release
Windows exact-head CI
Ubuntu exact-head CI
cargo package --locked
cargo publish --dry-run --locked
fresh local install
upgrade public 0.3.0 -> candidate 0.3.1
real normal `codex` smoke
performance budget recheck
```

Use the repository-governed Rust toolchain/MSRV and exact dependency policy.

## Upgrade proof

Upgrade must preserve:

```text
legacy v0.3 ledger
legacy receipt history
historical incomplete/failure evidence
aliases
revision history
interface preferences
```

The migration may add new v0.3.1 evidence/broker state but may not silently rewrite historical semantics.

Rollback/restore behavior for Codex IPC integration must be tested against the candidate.

## Fresh-install proof

A fresh user-local installation must be able to:

```text
install HookStat 0.3.1 candidate
run report/doctor/TUI without prior state
qualify/configure Codex IPC path where Native is unavailable
launch normal `codex`
collect truthful evidence
```

No HookStat launcher wrapper is admitted.

## Native release semantics

If ordinary Codex Native L2 is unavailable upstream, release notes and diagnostics must say so explicitly. IPC remains the authoritative production source for affected domains.

Do not claim zero-overhead Native production merely because controlled App Server L1 qualification passed.

## Public documentation

Update at least:

- README installation/architecture summary;
- CHANGELOG;
- v0.3.1 evidence-source/diagnostics documentation;
- migration notes from v0.3.0;
- privacy/performance contract documentation;
- exact Native-vs-IPC availability statement for the released Codex version.

## Public publication gate

The implementation train may prepare the exact release candidate, but MUST NOT without explicit Owner authorization:

```text
cargo publish
create/push public v0.3.1 tag
create public GitHub Release
```

A dry-run/package artifact and release notes may be prepared.

## Required release receipt

Return/record at minimum:

```text
START_MAIN
FINAL_MAIN
V031_RC_SHA
VERSION
MSRV
TESTS
WINDOWS_CI
LINUX_CI
PACKAGE
PUBLISH_DRY_RUN
UPGRADE_030_TO_031
FRESH_INSTALL_031
REAL_CODEX_DOGFOOD
PERFORMANCE_BUDGET
NATIVE_L2_STATE
PRODUCTION_AUTHORITY
PUBLICATION_AUTHORIZED=false|true
```

## Risk vector

```text
CODE_CHANGED=release_blockers_only
ARCHITECTURE_CHANGED=false
PERSISTENCE_CHANGED=release_blockers_only
CODEX_INTEGRATION_CHANGED=release_blockers_only
SECURITY_OR_PRIVACY_CHANGED=review_only_or_blockers
RELEASE_BOUNDARY=true
```

## Acceptance

```text
V031_ARCHITECTURE_FROZEN=true
V031_TESTS=PASS
WINDOWS_CI=PASS
LINUX_CI=PASS
PACKAGE=PASS
PUBLISH_DRY_RUN=PASS

UPGRADE_030_TO_031=PASS
FRESH_INSTALL_031=PASS
NORMAL_CODEX_LAUNCH=PASS
REAL_CODEX_DOGFOOD=PASS

PERFORMANCE_BUDGET=PASS
HOOKSTAT_INDUCED_TIMEOUTS=0
HOOKSTAT_INDUCED_FAILURES=0

NATIVE_FIRST=true
IPC_ONLY_FALLBACK=true
NO_THIRD_EVIDENCE_PATH=true
LEGACY_EVIDENCE_PRESERVED=true

CRATES_IO_PUBLICATION=OWNER_GATE
GITHUB_TAG=OWNER_GATE
GITHUB_RELEASE=OWNER_GATE
```

## Estimated effort

**4–6 effective engineering hours.**

## Next

Public v0.3.1 only after explicit Owner publication authorization. No automatic continuation into v0.4 or a future-runtime production track.
