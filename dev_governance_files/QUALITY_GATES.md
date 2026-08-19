# HookStat Quality Gates

Use the smallest gate set that proves the changed risk. A goal file may strengthen these requirements.

## D — Docs/governance

- diff/scope sanity;
- internal links and terminology consistent;
- code CI not required if no code/build metadata changed.

## C — Ordinary Rust code

Iteration: `cargo fmt` and focused tests.
Settled candidate: `cargo fmt --check`, Clippy warnings-as-errors, all tests, locked build, one hosted CI on the settled candidate.

## E — Runtime evidence semantics

Required when parser/adapters/status normalization/coverage change:

- sanitized fixture family covering admitted statuses;
- same-event multi-handler identity proof where applicable;
- malformed/unknown record behavior;
- at least one read-only real-runtime owner smoke for claims not provable synthetically;
- explicit coverage statement;
- no live runtime mutation.

## S — Persistent HookStat state

Required for SQLite/schema/migration/cursor/dedup changes:

- repeated ingest is idempotent;
- incremental ingest adds only new records;
- interrupted/bad record does not corrupt prior accepted ledger;
- data directory isolation and privacy/data-minimization checks.

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

Release candidate must freshly pass package metadata, `cargo package`, `cargo publish --dry-run`, locked build/test/Clippy/format, README/help smoke, and one representative Codex dogfood. Publication is a separate owner-authorized action.

## Stop gate

Failure to prove a durable v0.1 Codex data source with per-handler identity and terminal result is not permission to add a wrapper/daemon/live mutation. Record `BLOCKED_DATA_SOURCE_DECISION_REQUIRED` and preserve all evidence.
