# HookStat

**Local-first reliability analytics for hooks across coding-agent runtimes.**

HookStat v0.1.0 is Codex-first, while its canonical ledger, analytics, JSON,
and TUI remain evidence-source-neutral. Passive durable receipts remain the
preferred architecture. Current Codex 0.147.0 does not provide enough passive
retrospective per-handler terminal evidence, so v0.1 also supports an **opt-in
transparent instrumented receipt source**.

Normal daily launch is unchanged:

```text
codex
```

HookStat never requires `hookstat codex` as a launcher and does not require a
daemon.

## Opt-in Codex instrumentation

Start with read-only discovery. It reports only hashed handler identities and
coverage/trust consequences, never raw command strings:

```powershell
hookstat codex instrument --dry-run
```

On a supported Codex installation, this dry-run combines local static
`hooks.json`/inline-TOML discovery with the read-only App Server `hooks/list`
effective view. It reports source-class counts, enabled/trusted state where the
runtime exposes it, reconciliation counts, and explicit unsupported coverage.
The short-lived App Server request is stopped after its response; HookStat does
not create a daemon, session, launcher wrapper, or trust change. Command text,
source paths, matchers, and plugin identifiers are reduced to fingerprints
before output. If the App Server is unavailable, the dry-run says so and
retains the static view instead of inventing effective coverage.

For an explicit authorized activation, select the configuration root explicitly.
This is deliberate: `--apply` has no implicit live-default target.

```powershell
hookstat codex instrument --apply --config-root $env:USERPROFILE\.codex
hookstat codex instrument --trust --config-root $env:USERPROFILE\.codex
hookstat codex instrument --restore --config-root $env:USERPROFILE\.codex
```

Apply is atomic, creates an exact local prestate backup and rollback journal,
is idempotent, refuses configuration drift during restore, and will not wrap a
handler twice. It supports safe `hooks.json` command handlers. Inline TOML,
plugin, and managed sources are shown as unsupported coverage rather than
modified optimistically. Changing a hook command can require Codex trust review.
`--apply` never approves trust. The separate explicit `--trust` action uses
Codex's official App Server `hooks/list` and `config/batchWrite` route only
after it proves the exact current HookStat manifest, journal, source path,
supported user-handler identity, and current hash. It writes only selected
`trusted_hash` values, reloads user config, and requires every selected handler
to return `trusted`. Plugin, managed, disabled, stale, duplicate, and unrelated
hooks are rejected or left unchanged; it never bypasses trust enforcement.

Effective plugin or managed handlers can be visible in discovery even when
HookStat cannot mutate them. That is `PASS_WITH_EXPLICIT_UNSUPPORTED_COVERAGE`,
not a healthy zero-rate claim. v0.1 instruments only enabled, trusted,
unmanaged command handlers in safely supported user/project `hooks.json`
layers. Inline TOML and any source that cannot be restored byte-exactly remain
read-only coverage limitations.

The proxy executes the original handler with the same stdin/stdout/stderr data
flow and returns its exit code. It does not inspect or persist prompt text, tool
arguments, assistant messages, arbitrary standard-stream content, or raw hook
commands in the reliability ledger. It writes atomic HookStat-owned metadata
receipts under the platform user-data directory, then later ingests them into a
SQLite ledger. Start-only receipts are explicitly `incomplete`, not success or
failure. Exit code `2` is `unknown` because Codex control semantics depend on
stderr content that HookStat intentionally does not inspect.

On Windows, the proxy joins a Job Object with kill-on-close before spawning the
original handler. Forced proxy termination therefore closes the active handler
tree without broad process killing. After normal root-handler completion it
clears that limit, allowing legitimate background descendants to survive.
Unix keeps its native shell cancellation behavior in v0.1 and does not claim
the Windows Job Object containment guarantee.

## Reports and TUI

```powershell
hookstat report
hookstat report --json
hookstat
```

Reports and the Ratatui TUI provide 24h/7d/30d/All selections, per-handler
counts, distinct terminal states, coverage warnings, and latency only when every
terminal observation proves duration. Every failure percentage includes its
terminal sample count. Partial/incomplete coverage never appears as `0.00%
healthy`.

`hookstat preview-fixture [--json]` is sanitized deterministic development data,
not Owner Codex history.

## Development and release checks

```powershell
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --locked
cargo build --locked
cargo package
cargo publish --dry-run
```

Actual crates.io publication, a `v0.1.0` tag, and GitHub Release creation remain
separate Owner-authorized actions.

See the architecture, ADRs, and execution contracts under `docs/`,
`dev_governance_files/`, and `goals/`.

## License

MIT.
