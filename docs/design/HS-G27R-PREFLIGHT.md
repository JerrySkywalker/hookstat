# HS-G27R Release-Readiness Preflight

This is a release-preparation record, not a release candidate, version bump,
publication authorization, tag, or GitHub Release.

```text
PUBLIC_HOOKSTAT_VERSION=0.2.1
PREFLIGHT_HEAD=c007f4f7e38668398d4b51d7d0cbc0c850e73d5e
G27R=PRE_FLIGHT_ONLY
PUBLICATION_AUTHORIZED=false
OWNER_AB_VISUAL_SMOKE=OWNER_ACTION_REQUIRED
```

## Completed unattended evidence

| Area | Evidence |
| --- | --- |
| v0.2.1 behavioral boundary | Existing deterministic analytics, receipt, ledger, proxy/trust, diagnostics, async refresh, period, and terminal tests remain in the settled suite. G24/G25 only project admitted records or persist sanitized HookStat alias metadata. |
| Workbench scale | `workbench::tests::projects_ten_thousand_invocations_many_hooks_and_revision_epochs_deterministically` covers 10,240 invocation records across 64 hooks and multiple revision epochs. Finite reliability reads remain bounded by `ledger::tests::finite_reliability_queries_materialize_only_the_recent_bounded_range`. |
| Privacy and coverage | Canonical records and failure exploration retain bounded status categories/timestamps/counts only; no new stdout, stderr, prompt, tool payload, command, path, credential, or network telemetry field exists. Incomplete/history-only states remain explicit and do not imply healthy, active, removed, or recovered behavior without admitted evidence. |
| Human interface | G26's [automated matrix](HS-G26-AUTOMATED-HARDENING-MATRIX.md) covers shared contract semantics, en-US/zh-CN, CJK width, color-disabled rendering, normal/narrow/minimum layouts, loading/empty/error, Changes, Catalog, alias drafts, failure clusters, press-only input, Help, async requests, diagnostics independence, and terminal restoration. |
| Shared UI package | The preflight's original shared-package namespace is superseded by the public `terminal-ui-contract` 0.1.0 release. Its accepted release SHA is `7fde0040f3135e2772461833846b5525732564ea`; consumers must bind that registry package and rerun their package/install proof. |

## Release dependency inventory

| Component | Current development binding | Requirement before a consumer publish |
| --- | --- | --- |
| HookStat | `hookstat` 0.2.1; no release-version mutation in this train | RC train selects a v0.3 version only after all gates are accepted. |
| Shared Human UI | crates.io `terminal-ui-contract` `=0.1.0` | Verify the registry release, bind each consumer to its exact released semver version, then run consumer package/install proof. |
| TabBeacon | Consumer of the same shared source contract | Verify its accepted consumer binding independently before any TabBeacon publication. |
| Rust toolchain | Rust 1.97.1 | Retain exact toolchain for package/CI and fresh-install checks. |

The required order is therefore:

1. Verify `terminal-ui-contract` 0.1.0 is available from crates.io.
2. In new consumer candidates, bind the released shared crate version, lock
   dependencies, and prove `cargo package`, publish dry-run, and fresh
   installation.
3. Owner completes the HookStat/TabBeacon Windows Terminal A/B smoke.
4. Run the dedicated HookStat v0.3 RC train at one exact head, including the
   final version decision, exact-head Windows/Linux CI, package/install proof,
   and separate publication authorization.

## Draft release notes — not published

### HookStat v0.3 (draft)

- adds a coverage-aware Changes workbench with evidence-backed first/last/latest
  observation times, revision timelines, and conservative regression/recovery
  classifications;
- adds a Hook Catalog with safe local Human aliases, selected-period confidence,
  freshness, compact trend context, and bounded failure-cluster drill-down;
- converges terminal navigation, footer grammar, settings/discard behavior,
  Help, geometry, locale resolution, typography, and no-color semantics with
  the shared TabBeacon Human-interface contract;
- preserves Codex-only production scope, ordinary `codex` launch, explicit
  trust boundaries, local-first privacy, coverage truthfulness, receipt
  semantics, and v0.2.1 period behavior.

## Remaining RC blockers

```text
OWNER_AB_VISUAL_SMOKE=OWNER_ACTION_REQUIRED
SHARED_UI_PUBLICATION=COMPLETE_TERMINAL_UI_CONTRACT_0_1_0
CONSUMER_SEMVER_REBIND_AFTER_SHARED_PUBLICATION=IN_PROGRESS
HOOKSTAT_V03_RC_VERSION_DECISION=NOT_STARTED
HOOKSTAT_CARGO_PACKAGE_WITH_GIT_SHARED_DEPENDENCY=EXPECTED_BLOCKED
HOOKSTAT_CARGO_PUBLISH_DRY_RUN_WITH_GIT_SHARED_DEPENDENCY=EXPECTED_BLOCKED
FINAL_RC_EXACT_HEAD_PACKAGE_INSTALL_SMOKE=NOT_STARTED
FINAL_PUBLICATION_AUTHORIZATION=NOT_GRANTED
```

The publishability entries are intentional ordering constraints, not a reason
to publish a crate or mutate HookStat's version during this preflight.

This preflight record predates the neutral shared-package migration. Its
Git-dependency packaging limitation is superseded by the required registry
binding and fresh package/install evidence for the migrated consumer candidate.
