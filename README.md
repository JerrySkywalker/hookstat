# HookStat

**Reliability analytics for hooks across coding-agent runtimes.**

HookStat is a local-first Rust CLI/TUI for turning real hook executions into reliability history: invocation counts, failures, failure rates, latency, coverage, and later revision-aware comparisons.

## Status

HookStat is in repository foundation. The v0.1 implementation target is **Codex-first**, while the architecture is explicitly multi-runtime-ready for DeepSeek Harness, OpenCode, and future runtimes.

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
