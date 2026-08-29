# HookStat CI Audit and Release Fast Lane

## Status and scope

This document records the `HS-WORKFLOW-FASTLANE-AUDIT-AND-OPTIMIZATION-014`
baseline and the workflow architecture adopted by
`HS-WORKFLOW-FASTLANE-IMPLEMENTATION-015`. It changes release process only;
it does not alter HookStat product, HSIP, IPC, ledger, analytics, runtime, or
TUI behavior.

```text
HOSTED_CI_IS_PRIMARY_BOTTLENECK=false
CANDIDATE_CHURN_IS_PRIMARY_PROCESS_BOTTLENECK=true
BRANCH_PROTECTION_AUDIT=UNPROVEN_HTTP_401
BRANCH_PROTECTION_CONTEXTS=UNKNOWN
```

Because required branch-protection contexts were not observable, the workflow
retains its externally visible `CI` workflow and its `rust (windows-latest)`
and `rust (ubuntu-latest)` matrix job identities. A later Owner/admin audit
may simplify those compatibility contexts only after it proves the actual
required-check configuration.

## Frozen audit observations

| Concern | Observation | Meaning |
| --- | ---: | --- |
| CI compute time | median `186 s`; range `161–240 s` | Hosted execution is short relative to the overall train. |
| Matrix churn | `36` full matrices across PRs 36–41 | A full matrix often ran before a candidate was settled. |
| Superseded CI | `17` successful results | Later commits invalidated otherwise useful exact-SHA evidence. |
| Docs/governance matrices | `8` | Process-only changes paid code-toolchain cost. |
| Receipt commits on PR 40 | `4` commits; `3` invalidations | Committing acceptance receipts changed the candidate and restarted proof. |
| Potentially avoidable work | `22` matrices; `66 m 26 s` Actions wall time | Avoidable CI was material, but not the whole end-to-end delay. |
| Final PR 40 head to merge | about `14 m 25 s` | Review and qualification scheduling remained significant. |

The observed primary delay was candidate churn plus SHA invalidation, with
serialized review, Windows qualification, release packaging, and a Windows
benchmark environment rejection discovered too late. This is not a prediction
of a precise future time reduction. A future settled release candidate must
pilot this flow and report its measured wall time without treating this audit
as a time guarantee.

## Risk-aware PR validation

`scripts/ci/classify-change.ps1` consumes a base SHA, head SHA, or an explicit
changed-file list and emits conservative risk flags:

```text
RISK_D RISK_C RISK_E RISK_S RISK_T RISK_P RISK_R
WINDOWS_SENSITIVE UNIX_SENSITIVE
PACKAGE_SURFACE_CHANGED PERFORMANCE_SURFACE_CHANGED WORKFLOW_CHANGED
UNKNOWN_RISK
```

| Changed surface | Iteration path | Candidate/final consequence |
| --- | --- | --- |
| `docs/**`, `dev_governance_files/**`, README only | lightweight diff and classifier fixtures; no Rust toolchain | both compatibility contexts still complete successfully |
| recognized platform-neutral Rust | Ubuntu Rust gate during iteration | manual candidate dispatch runs both platforms |
| IPC/Windows process or Windows test code | Windows Rust gate | candidate runs both platforms; Windows qualification follows freeze when required |
| Unix/UDS/socket code | Ubuntu Rust gate | candidate runs both platforms |
| ledger, receipt, migration, or storage | Rust plus state-risk classification | candidate full matrix and state-specific proof |
| TUI | Rust plus TUI-risk classification | candidate full matrix and representative presentation proof when required |
| Cargo/build metadata, release scripts, CI/review scripts, workflows | current full Windows and Ubuntu matrix | release/package or workflow policy review is required |
| unknown `src/**`, `tests/**`, build-sensitive, or unrecognized path | full matrix | `UNKNOWN_RISK=true`; no under-test exception |

The classifier is intentionally a widening mechanism. `UNKNOWN_RISK=true`
always produces:

```text
RUN_FULL_WINDOWS=true
RUN_FULL_UBUNTU=true
```

Deleted and Git type-change paths are classified too. Renames are deliberately
evaluated as an old path deletion plus a new path addition, so a source-to-docs
rename or file/symlink/submodule transition cannot turn into a documentation-
only fast path. The legacy matrix jobs fetch full history because their
lightweight validation compares the same base and head commits as the
classifier. They also always start after a classifier failure and explicitly
fail, rather than becoming skipped contexts that could leave unknown required
checks unsafely satisfied or pending.

The `workflow_dispatch` `full_matrix=true` input is also widening-only and is
the explicit candidate-freeze final CI path. It never makes an unsafe change
look safe.

No target cache is added by this train:

```text
CI_CACHE_IMPLEMENTED=false
CI_CACHE_DECISION=LOW_EXPECTED_VALUE
```

Toolchain setup was observed at roughly 12–24 seconds. Cache complexity would
not address the dominant churn and invalidation problem, and cached artifacts
must never substitute for compilation or testing.

## Immutable candidate policy

Finish implementation and release documentation before freezing. Then record
the exact commit in non-mutating evidence:

```text
CANDIDATE_SHA=<exact SHA>
CANDIDATE_FROZEN=true
POST_FREEZE_CODE_COMMITS=0
POST_FREEZE_DOC_COMMITS=0
POST_FREEZE_RECEIPT_COMMITS=0
NO_COMMIT_AFTER_FREEZE=true
```

A real defect invalidates that candidate: fix it, create a new SHA, and start
a new candidate. Acceptance evidence never edits the frozen candidate.

Allowed durable post-freeze evidence is bound to the exact SHA and lives
outside the candidate tree:

- a PR comment bound to the exact SHA;
- a GitHub Actions artifact;
- a machine-readable local receipt bound to the SHA;
- a fresh reviewer receipt/comment bound to the SHA.

The forbidden churn loop is:

```text
candidate passes -> commit README receipt -> SHA changes -> CI invalid -> review invalid -> repeat
```

Historical committed receipts remain historical; this policy does not rewrite
them.

## Independent review and parallel acceptance

The audit found zero GitHub review objects across PRs 36–41. A single GitHub
account is not represented as external human approval. The process instead
requires a fresh independent reviewer process to emit an exact-SHA, non-mutating
receipt such as:

```text
INDEPENDENT_REVIEW_RECEIPT
REVIEWED_SHA=<sha>
REVIEWER_PROCESS=FRESH
REVIEWER_PROFILE=<profile>
RESULT=PASS|FINDINGS
MATERIAL_FINDINGS=<n>
```

The receipt is a PR comment or equivalent durable artifact. The reviewer may
start immediately after candidate freeze and does not wait for hosted CI.

```text
                   CANDIDATE FREEZE
                         |
       +-----------------+-----------------+
       v                 v                 v                 v
  Hosted final CI   Independent review   Windows preflight   Release gate
                                              -> qualification
       |                 |                 |                 |
       +-----------------+-----------------+-----------------+
                         |
                         v
                 Acceptance decision
                         |
                         v
                       Merge
```

The release-specific package/install/upgrade gate may run in parallel with
Windows qualification when its own conditions are independent. A failed
Windows preflight is an environment rejection, not a product performance
result, and it must not start expensive performance qualification.

## Windows qualification preflight

Run `scripts/qualification/windows-performance-preflight.ps1` before an
absolute performance run. It is observation-only and never kills or reprioritizes
processes. Its sanitized receipt contains aggregate counts for unrelated
`cargo`, unrelated `rustc`, other HookStat qualification/performance work, and
a small series of total-CPU samples.

The preflight rejects an environment when process interference exists or when
multiple samples show sustained extreme CPU. The default is a deliberately
coarse `90%` extreme signal over at least two samples; it is a conservative
admission rule motivated by the previously rejected `93.7–99.4%` observation,
not a claimed universal performance threshold.

```text
ENVIRONMENT_BUSY=true
MUTATION=false
PERFORMANCE_CLAIM=NONE
DISPOSITION=ENVIRONMENT_BUSY
```

## Release gate responsibilities

`scripts/release/release-gate.ps1 -CandidateSha <sha>` first requires a clean
tree, `HEAD == CANDIDATE_SHA`, and an expected manifest version (default
`0.3.1`). It composes, rather than duplicates,
[`verify-package.ps1`](../../scripts/release/verify-package.ps1): the existing
archive, path-dependency, development-proof exclusion, package build, isolated
fresh install, packaged binaries, and transparent-shim admission checks remain
the package authority.

The wrapper adds a locked publish dry-run, the isolated public-v0.3.0 binary
and receipt-spool/journal upgrade proof, the ledger and interface-preference
preservation fixtures, and disposable report/doctor smoke coverage. It
produces a single result:

```text
RELEASE_GATE_VERSION=1
CANDIDATE_SHA=<sha>
VERSION=<version>
PACKAGE=PASS|FAIL
PUBLISH_DRY_RUN=PASS|FAIL
FRESH_INSTALL=PASS|FAIL
UPGRADE_030_TO_031=PASS|FAIL
LEGACY_EVIDENCE_PRESERVED=PASS|FAIL
REPORT_SMOKE=PASS|FAIL
DOCTOR_SMOKE=PASS|FAIL
OVERALL=PASS|FAIL
```

The result can be reused only for the same candidate SHA, unchanged release
gate implementation, and unchanged environment-specific requirements.

## Evidence reuse and invalidation

| Change | Hosted CI | Independent review | Windows performance | Package/install |
| --- | --- | --- | --- | --- |
| PR comment only | reuse | reuse | reuse | reuse |
| external review receipt | reuse | N/A | reuse | reuse |
| README after freeze | new candidate prohibited | new candidate | performance code may be technically unchanged, but candidate reset | package may change |
| IPC code | rerun | rerun | rerun | rerun if package changes |
| analytics-only code | rerun | rerun | reuse IPC performance only where policy permits | rerun package |
| workflow-only | workflow validation | review workflow | reuse product performance | reuse product package only when policy proves it unaffected |
| candidate version/Cargo metadata | rerun relevant CI | rerun | reuse only if binary behavior is provably unchanged and governance allows | rerun |

The preferred outcome is not to debate ambiguous reuse: `NO_COMMIT_AFTER_FREEZE`
makes most of those rows inapplicable.

## Operating model

```text
ITERATION
    -> focused local tests
    -> PR fast CI based on changed risk
    -> fix until implementation and release docs settle
    -> CANDIDATE FREEZE
       -> full hosted CI
       -> fresh independent review
       -> Windows performance preflight then qualification
       -> release gate
    -> acceptance decision
    -> merge
```

The final hosted matrix owns format, Clippy, full tests, locked build, Windows,
and Ubuntu. The release gate owns candidate/version assertions, package,
publish dry-run, archive/fresh-install validation, upgrade preservation, report
and doctor smoke, legacy preservation, and release metadata. Do not run a
second full ordinary local Rust gate on the same exact candidate unless the
changed risk specifically requires it.
