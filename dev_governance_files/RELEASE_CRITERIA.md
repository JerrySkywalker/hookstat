# Historical HookStat v0.1 Release Criteria

> Historical record only. These v0.1 criteria are not the current v0.3.1
> release gate and must not be interpreted as requiring v0.1 live
> instrumentation or dogfood work for a future candidate. For v0.3.1 use
> `ROADMAP_V031.md`, `QUALITY_GATES.md`, and the immutable-candidate process in
> `docs/process/HOOKSTAT-CI-AUDIT-AND-RELEASE-FASTLANE.md`.

A v0.1.0 release-ready candidate requires:

1. ordinary `codex` usage; no HookStat launcher wrapper or mandatory daemon;
2. admitted passive or explicitly opt-in instrumented Codex evidence source with explicit coverage semantics;
3. stable per-handler identity for the rows HookStat reports;
4. trustworthy invocation denominator and failed-run classification;
5. local incremental/idempotent ledger;
6. 24h/7d/30d/All reliability views;
7. frozen-baseline TUI with sample counts and visible coverage limitations;
8. sanitized fixture tests and real ordinary-Codex dogfood. When passive
   durable evidence is unavailable, dogfood requires explicitly authorized
   opt-in live instrumentation, real metadata receipts, report/TUI validation,
   visible unsupported coverage, and a working exact restore;
9. Windows first-class behavior, including Job Object proof that proxy-only
   termination cleans up active handler descendants without killing unrelated
   processes or legitimate descendants after normal root completion; Linux CI
   must also pass and must not claim the Windows containment guarantee;
10. no implicit mutation of Codex config/hooks/trust/history; opt-in
    instrumentation must be transparent, reversible, drift-safe, backed up
    exactly, and restricted to supported handlers. `--apply` never alters
    trust. A separate explicit trust action may use only Codex's official
    scoped App Server mechanism after proving the current manifest, journal,
    configuration, source, identity, and hash; it must preserve unrelated
    state, verify `trusted`, and never bypass trust enforcement;
11. no raw prompts/tool payloads durably stored by default;
12. format, Clippy, tests, locked build and exact candidate CI pass;
13. `cargo package` and `cargo publish --dry-run` pass after versioning to 0.1.0.

Actual crates.io publication, tag, or public GitHub Release is not authorized by repository development goals alone; it requires separate explicit Owner authorization for the exact version.
