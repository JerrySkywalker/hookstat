# HookStat Quality Gates

Use the smallest gate set that proves the changed risk. A goal file may
strengthen these requirements. `ROADMAP_V031.md` is the authoritative release
scope for v0.3.1; `RELEASE_CRITERIA.md` is a historical v0.1 record only.

## Candidate and workflow policy

Classify changed paths with `scripts/ci/classify-change.ps1`. An unknown
source, test, build-sensitive, or unrecognized path is never a fast-path
exception:

```text
UNKNOWN_RISK=true
RUN_FULL_WINDOWS=true
RUN_FULL_UBUNTU=true
```

Workflow, CI/review script, Cargo/build metadata, and release-script changes
also require the current full Windows/Ubuntu matrix. Docs/governance/README-only
changes receive lightweight validation while the existing `CI / rust
(windows-latest)` and `CI / rust (ubuntu-latest)` contexts remain satisfiable.
Do not rename or remove those contexts while branch-protection configuration is
unproven.

After code and release documentation settle, record `CANDIDATE_SHA` and freeze
it. No code, documentation, or receipt commit is permitted after the freeze.
A defect creates a new candidate SHA. PR comments, Actions artifacts, local
machine receipts, and fresh review receipts are the permitted non-mutating
acceptance evidence.

## D — Docs/governance

- diff/scope sanity;
- internal links and terminology consistent;
- code CI not required if no code/build metadata changed.

## C — Ordinary Rust code

Iteration: `cargo fmt` and focused tests.
Settled candidate: the hosted final CI owns `cargo fmt --check`, Clippy
warnings-as-errors, all tests, locked build, Windows, and Ubuntu. Do not run a
duplicative complete local Rust gate on the same SHA unless the changed risk
requires it.

## E — Runtime evidence semantics

Required when parser/adapters/status normalization/coverage change:

- sanitized fixture family covering admitted statuses;
- same-event multi-handler identity proof where applicable;
- malformed/unknown record behavior;
- at least one read-only real-runtime owner smoke for claims not provable synthetically, or an explicit owner-activation-required receipt when live instrumentation is prohibited. When a goal expressly authorizes live instrumentation, prove dry-run/reconciliation, exact backup, scoped official trust review when required, real receipts/report/TUI, and restore;
- explicit coverage statement;
- no live runtime mutation unless the active goal explicitly authorizes the
  bounded activation and all of its preflight, trust, privacy, and restore
  gates pass.

## S — Persistent HookStat state

Required for SQLite/schema/migration/cursor/dedup changes:

- repeated ingest is idempotent;
- incremental ingest adds only new records;
- interrupted/bad record does not corrupt prior accepted ledger;
- data directory isolation and privacy/data-minimization checks.

Instrumented receipt spools additionally prove atomic start/completion records,
concurrent writer safety, duplicate ingest, malformed isolation, and explicit
incomplete/unknown terminal coverage. Concurrent hooks must not write the main
SQLite database directly.

## T — TUI/presentation

Required when TUI changes:

- deterministic buffer/render tests at representative widths;
- empty/partial/error/normal states;
- failure rate always paired with sample count;
- incomplete coverage cannot masquerade as healthy zero failure;
- one representative interactive smoke on Windows before v0.1 closure.

## P — Security/privacy

Required when data retained/exported or runtime permissions change. Review raw-content persistence, secret exposure, network egress, path disclosure, and mutation boundaries.

## R — Release

After the immutable candidate freezes, launch these independent gates in
parallel where their prerequisites are independent:

- manual full hosted CI for the exact SHA;
- a fresh independent exact-SHA review receipt;
- Windows performance preflight before any expensive absolute qualification;
- `scripts/release/release-gate.ps1 -CandidateSha <sha>`.

The release gate composes the existing `verify-package.ps1` archive and
fresh-install proof, then owns publish dry-run, v0.3.0-to-v0.3.1 legacy
preservation, report/doctor smoke, and one machine-readable result bound to
the candidate SHA and gate version. Its package/install result is reusable only
when the candidate SHA, gate implementation, and relevant environment contract
are unchanged. Publication remains a separate owner-authorized action.

An independent reviewer must emit `INDEPENDENT_REVIEW_RECEIPT` with
`REVIEWED_SHA`, `REVIEWER_PROCESS=FRESH`, `REVIEWER_PROFILE`, result, and
material-finding count. It is durable non-mutating evidence, not an approval
pretending that the same GitHub account is an external reviewer.

## Stop gate

Failure to prove passive durable Codex evidence is not permission to silently
add a daemon or launcher wrapper. An instrumented source requires an explicit
Owner architecture decision, transparent proxy semantics, fixture-proven
apply/restore, and no live-owner configuration mutation unless the active goal
expressly authorizes that bounded activation.
