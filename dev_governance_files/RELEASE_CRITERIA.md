# HookStat v0.1 Release Criteria

A v0.1.0 release-ready candidate requires:

1. ordinary `codex` usage; no HookStat launcher wrapper or mandatory daemon;
2. admitted passive or explicitly opt-in instrumented Codex evidence source with explicit coverage semantics;
3. stable per-handler identity for the rows HookStat reports;
4. trustworthy invocation denominator and failed-run classification;
5. local incremental/idempotent ledger;
6. 24h/7d/30d/All reliability views;
7. frozen-baseline TUI with sample counts and visible coverage limitations;
8. sanitized fixture tests and read-only real owner Codex dogfood;
9. Windows first-class behavior; Linux CI must also pass;
10. no implicit mutation of Codex config/hooks/trust/history; opt-in instrumentation must be transparent, reversible, drift-safe, and never alter trust;
11. no raw prompts/tool payloads durably stored by default;
12. format, Clippy, tests, locked build and exact candidate CI pass;
13. `cargo package` and `cargo publish --dry-run` pass after versioning to 0.1.0.

Actual crates.io publication, tag, or public GitHub Release is not authorized by repository development goals alone.
