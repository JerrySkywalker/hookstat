# HS-V01-CODEX-INSTRUMENTATION-RECOVERY-12H-001 checkpoint

## Scope and boundary

The accepted prior train was merged to `main` as `5c4270721240f85384474a9378ab65bfeae59ee9`.
This recovery train begins from that commit on
`agent/v01-codex-instrumentation-recovery-001`.

The Owner admitted an opt-in instrumented source while retaining passive
evidence as preferred. No Owner-live Codex configuration, hook definition,
trust state, session history, credentials, or private stream content was
modified or committed.

## Current evidence decision

- Passive Codex historical per-handler source: unavailable in the prior
  0.147.0 qualification.
- Instrumented source: admitted as an opt-in metadata receipt spool.
- Handler identity: `handler_key`, `handler_revision`, source kind, event,
  matcher fingerprint, structural identity, and execution mode; no raw command
  is in the ledger or report.
- Runtime discovery: the documented `hooks/list` App Server contract can expose
  enabled/trust/managed/plugin state, but this checkpoint's lightweight local
  dry-run safely reads user/project `hooks.json` and inline TOML only. It does
  not claim complete plugin/managed or trust-state coverage.

## Fixture evidence

- proxy preserves fixture stdin/stdout/stderr/exit code through inherited OS
  streams;
- nonzero execution failure, exit-2 unknown control/failure ambiguity,
  start-only incomplete coverage, malformed and duplicate receipts are distinct;
- atomic receipt start/completion records support concurrent processes and
  fail-open telemetry storage failure;
- fixture apply/restore proves exact backup, atomic replacement, idempotence,
  no double wrapping, drift refusal, and restore;
- SQLite upgrades an incomplete receipt only when a terminal completion later
  arrives; repeated refresh is idempotent;
- deterministic JSON and Ratatui normal/empty/partial/error/small/detail tests
  keep failure rate sample counts visible.

## Official behavior consulted

- https://learn.chatgpt.com/docs/hooks — command fields, concurrent matching
  handlers, async behavior, exit-2 control semantics, and trust review.
- https://learn.chatgpt.com/docs/app-server — `hooks/list` effective runtime
  discovery and synchronous-only hook run notifications.
- https://github.com/openai/codex/blob/main/codex-rs/hooks/src/engine/discovery.rs
  and `command_runner.rs` — current handler normalization/trust and shell/process
  cleanup behavior.

## Validation at checkpoint

```text
cargo fmt --check                                       PASS
cargo clippy --all-targets --all-features -- -D warnings PASS
cargo test --all-targets --locked                        PASS (26 tests)
cargo build --locked                                    PASS
```

The read-only dry-run discovered twelve locally configured command handlers and
reported only fingerprinted identities/counts. It performed no apply.
