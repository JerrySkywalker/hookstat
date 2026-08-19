# HookStat

**Reliability analytics for hooks across coding-agent runtimes.**

HookStat is a local-first Rust CLI/TUI for turning real hook executions into reliability history: invocation counts, failures, failure rates, latency, coverage, and later revision-aware comparisons.

## Status

**HS-G01 is blocked: `BLOCKED_DATA_SOURCE_DECISION_REQUIRED`.** On 2026-08-20, the current Codex installation exposed durable session JSONL and local state surfaces, but none proved the required combination of per-handler identity, invocation denominator, terminal status, and timestamp. The public App Server protocol exposes hook notifications live, not a retrospective durable evidence contract. See the sanitized [HS-G01 qualification receipt](runs/HS-V01-UNATTENDED-12H-TRAIN-001-G01-qualification.md).

Accordingly, this development build does **not** ingest the owner's Codex history, create a default ledger, or present an empty history as `0.00% healthy`. The v0.1 implementation target remains **Codex-first** once a durable source is admitted; the architecture is explicitly multi-runtime-ready for DeepSeek Harness, OpenCode, and future runtimes.

The v0.1 user contract is intentionally narrow:

```text
use codex normally
      ↓
Codex leaves local evidence
      ↓
run hookstat
      ↓
see per-hook historical runs / failures / failure rate
```

HookStat v0.1 must not require a launcher wrapper, daemon, mutation of Codex configuration, or mutation of Codex trust state.

### Current development commands

```powershell
# Explicitly reports the evidence-admission blocker (exit code 3).
hookstat status

# Deterministic development fixture only. It never reads Codex data.
hookstat preview-fixture
hookstat preview-fixture --json
```

`preview-fixture` is a synthetic test/report path, not a claim about local Codex history. It exists to exercise canonical records, reliability aggregation, frozen-style text rendering, and the HookStat-owned SQLite ledger tests while the data-source decision is unresolved.

## Development

```powershell
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo build --locked
```

The roadmap and execution rules live under `dev_governance_files/` and `goals/`. The normative TUI baseline is `docs/design/TUI_SPEC.md`.

## License

MIT.
