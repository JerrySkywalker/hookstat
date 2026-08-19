# HookStat Fast Lane

## Rule

```text
validate changed risk once
reuse unchanged-risk evidence
checkpoint meaningful progress
avoid ceremony without new information
```

### Docs-only

Governance/diff sanity. Do not run Rust/real-runtime gates merely because HEAD changed.

### Ordinary code

During iteration: format + focused tests. At the settled candidate: one full local gate and one hosted CI.

### Evidence semantics

Run one representative fixture family plus one real read-only Codex proof when the claim depends on live semantics. Reuse unchanged accepted evidence.

### Persistence

Run one idempotence/incremental/corruption-safety family for the changed storage behavior.

### TUI

Use deterministic render tests; one final representative owner smoke is enough after the candidate settles.

### Release

One release closure train. Do not repeatedly rebuild/package for metadata-only follow-up commits when the relevant package diff is empty.

## Unattended work

Push remote branch checkpoints after completed goals or major risk discoveries. Transient GitHub/network failures may receive bounded retry with backoff. A latched architectural blocker is not retried indefinitely; document it and switch to safe non-claiming work or stop as the active goal requires.
