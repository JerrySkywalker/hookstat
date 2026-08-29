# HookStat Fast Lane

## Rule

```text
validate changed risk once
reuse unchanged-risk evidence
checkpoint meaningful progress
avoid ceremony without new information
no commit after candidate freeze
```

### Docs-only

Governance/diff sanity and classifier fixtures. Do not install the Rust
toolchain or run Rust/real-runtime gates merely because HEAD changed. The
legacy Windows/Ubuntu CI contexts remain lightweight successful checks while
branch-protection contexts are unknown.

### Ordinary code

During iteration: format + focused tests and the classifier-selected platform
gate. At candidate freeze, dispatch the full hosted Windows/Ubuntu matrix for
the exact SHA. Avoid a duplicate complete local gate unless changed risk
requires it.

### Evidence semantics

Run one representative fixture family plus one real read-only Codex proof when the claim depends on live semantics. Reuse unchanged accepted evidence.

### Persistence

Run one idempotence/incremental/corruption-safety family for the changed storage behavior.

### TUI

Use deterministic render tests; one final representative owner smoke is enough after the candidate settles.

### Candidate freeze and release

Set `CANDIDATE_SHA=<exact SHA>` only after implementation and release
documentation settle. Then no code, docs, or receipt commit is allowed. A
defect means a new SHA and a new candidate; it never means adding a receipt
commit to the old one.

Start hosted final CI, a fresh independent mechanical review, Windows
performance preflight/qualification, and the release gate in parallel when
independent. The release gate reuses `verify-package.ps1` rather than creating
a second package implementation. Store exact-SHA results in PR comments,
Actions artifacts, or machine-readable receipts, not in candidate commits.

### Evidence reuse

PR comments and external review receipts do not invalidate an unchanged
candidate. Product code, Cargo/version metadata, or release/package changes do
invalidate the relevant proof. Workflow-only changes require workflow
validation and fresh workflow review but may reuse product-performance evidence
when the product performance surface is demonstrably unchanged. The full
invalidation table is in
[`HOOKSTAT-CI-AUDIT-AND-RELEASE-FASTLANE.md`](../docs/process/HOOKSTAT-CI-AUDIT-AND-RELEASE-FASTLANE.md).

## Unattended work

Push remote branch checkpoints after completed goals or major risk discoveries. Transient GitHub/network failures may receive bounded retry with backoff. A latched architectural blocker is not retried indefinitely; document it and switch to safe non-claiming work or stop as the active goal requires.
