# HS-G38R — v0.3.1 Hardening & Release

## Status

PLANNED after accepted G38D.

## Objective

Freeze the v0.3.1 architecture, close HookStat-owned release blockers, prove package/upgrade/fresh-install behavior, normal Codex non-interference, and prepare the exact public v0.3.1 candidate.

Public publication remains an explicit Owner gate.

A named external cooperative producer is not required for G38R or public v0.3.1 preparation.

## Freeze rule

After G38D acceptance:

```text
NEW_ARCHITECTURE_WORK=false
NEW_PRODUCT_FEATURE=false
NEW_RUNTIME=false
EXTERNAL_INTEGRATION_IMPLEMENTATION=false
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

Do not use release hardening as permission to modify an external producer repository or begin OpenCode/DeepSeek/Claude/Agy work.

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
HSIP reference/conformance smoke
normal `codex` non-interference smoke
reference HSIP performance budget recheck
report/doctor/TUI consistency
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

The migration may add new v0.3.1 broker/diagnostic/conformance state but may not silently rewrite historical semantics.

If no external IPC integration is installed, upgrade validation must not invent one or mutate another repository/configuration to create coverage.

## Fresh-install proof

A fresh user-local installation must be able to:

```text
install HookStat 0.3.1 candidate
run report/doctor/TUI without prior state
run HSIP v1 reference/conformance qualification
inspect Native capability/admission truthfully
inspect IPC integration admission truthfully
show unsupported domains as NOT_ADMITTED
launch normal `codex`
remain non-invasive when no evidence integration is admitted
```

No HookStat launcher wrapper is admitted.

No external repository modification is permitted as part of fresh-install proof.

## Native and IPC release semantics

If ordinary Codex Native L2 is unavailable upstream, release notes and diagnostics must say so explicitly.

IPC is authoritative only for a domain with a separately admitted named integration.

If no such integration exists, the domain remains:

```text
NOT_ADMITTED
```

This is allowed release state when represented truthfully.

The non-admitted transparent shim is never an implicit fallback.

The HookStat reference producer is a conformance instrument only and is never production runtime authority.

## Performance release gate

G38R rechecks HookStat's own substrate:

```text
REFERENCE_HSIP_P95_MS<=1
REFERENCE_HSIP_P99_MS<=2
REFERENCE_HSIP_OBSERVATION_GAPS=0
HOOKSTAT_INDUCED_TIMEOUTS=0
HOOKSTAT_INDUCED_FAILURES=0
```

External integration performance is checked only when that integration seeks admission; it is not a v0.3.1 release prerequisite.

Historical transparent-shim failures remain preserved and do not become release PASS evidence.

## Public documentation

Update at least:

- README installation/architecture summary;
- CHANGELOG;
- v0.3.1 evidence-source/diagnostics documentation;
- HSIP v1 integration/conformance documentation;
- migration notes from v0.3.0;
- privacy/performance contract documentation;
- exact Native availability statement for the released Codex version;
- explicit statement that no external cooperative producer is bundled/required by HookStat v0.3.1;
- instructions for third-party integrations to qualify against the HSIP contract without changing HookStat Core.

Do not claim live runtime coverage for any domain that is `NOT_ADMITTED`.

## Public publication gate

The implementation train may prepare the exact release candidate, but MUST NOT without explicit Owner authorization:

```text
cargo publish
create/push public v0.3.1 tag
create public GitHub Release
```

A dry-run/package artifact and release notes may be prepared.

## Required release receipt

Record at minimum:

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
HSIP_CONFORMANCE
REFERENCE_HSIP_PERFORMANCE
NORMAL_CODEX_SMOKE
NATIVE_L2_STATE
PRODUCTION_AUTHORITY_BY_DOMAIN
NOT_ADMITTED_DOMAINS_EXPLICIT
EXTERNAL_INTEGRATION_REQUIRED=false
PUBLICATION_AUTHORIZED=false|true
```

## Risk vector

```text
CODE_CHANGED=release_blockers_only
ARCHITECTURE_CHANGED=false
PERSISTENCE_CHANGED=release_blockers_only
CODEX_INTEGRATION_CHANGED=none_or_release_blocker_only
EXTERNAL_REPOSITORY_CHANGED=false
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
HSIP_CONFORMANCE=PASS
REFERENCE_HSIP_PERFORMANCE=PASS
NORMAL_CODEX_LAUNCH=PASS

HOOKSTAT_INDUCED_TIMEOUTS=0
HOOKSTAT_INDUCED_FAILURES=0

NATIVE_FIRST=true
IPC_ADMITTED_INTEGRATION_ONLY_FALLBACK=true
NO_THIRD_EVIDENCE_PATH=true
LEGACY_EVIDENCE_PRESERVED=true
COVERAGE_TRUTHFUL=true
EXTERNAL_INTEGRATION_REQUIRED=false

CRATES_IO_PUBLICATION=OWNER_GATE
GITHUB_TAG=OWNER_GATE
GITHUB_RELEASE=OWNER_GATE
```

## Estimated effort

**4–6 effective engineering hours.**

## Next

Public v0.3.1 only after explicit Owner publication authorization. No automatic continuation into v0.4, G36T, or any external runtime-integration track.
