# HookStat

**Local-first reliability analytics for hooks across coding-agent runtimes.**

HookStat v0.2 is the Reliability Center: a TabBeacon-aligned terminal UI that
turns admitted local hook receipts into a bilingual, human-readable operational
view. It is Codex-first today, while its canonical ledger, analytics, JSON,
and TUI remain evidence-source-neutral. Passive durable receipts remain the
preferred architecture. When passive per-handler terminal evidence is not
available, HookStat supports an **opt-in transparent instrumented receipt
source**.

Normal daily launch is unchanged:

```text
codex
```

HookStat never requires `hookstat codex` as a launcher and does not require a
daemon.

## Install and update

Install the authorized stable v0.2.0 package from crates.io with Cargo:

```powershell
cargo install hookstat --version 0.2.0 --locked
cargo install hookstat --version 0.2.0 --locked --force
```

For local development from an owned checkout instead:

```powershell
cargo install --path . --locked
hookstat --version
```

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

For Codex 0.147 Windows command execution, HookStat keeps the portable quoted
`command` and writes a separate `commandWindows`. Its private manifest pathname
is URL-safe encoded there, so after the optional leading quoted executable the
Windows command contains no embedded quotes. The manifest token itself has no
whitespace or `cmd.exe` metacharacters. This keeps ordinary installations and
data directories with spaces or non-ASCII characters compatible without
changing the original handler's command or process-containment behavior.

## Reliability Center, reports, and diagnostics

```powershell
hookstat report
hookstat report --json
hookstat report --read-only --json
hookstat doctor
hookstat doctor --json
hookstat diagnostics export --output .\hookstat-diagnostics.json --apply
hookstat
```

`hookstat` opens the full-screen Reliability Center. Its Overview highlights
coverage and the most meaningful risks; Hooks supports search, failed-only
filtering, and sorting; Hook Detail keeps the human display identity primary
and shows the internal key only as metadata. Diagnostics is observational and
read-only. It can report HookStat/Codex presence, effective runtime visibility,
instrumentation/trust state, spool and SQLite health, receipt integrity,
coverage, PATH identity, and evidence freshness. It never applies
instrumentation, repairs hooks, writes trust, or mutates Codex.

For local development performance evidence, `hookstat --timing-output` emits
sanitized phase timing and work counters after the terminal restores. The
output contains no paths, receipt bodies, commands, prompts, payloads, or
standard-stream content; it is not telemetry and is never sent remotely.

Reports and the UI provide deterministic Today/24h/7d/30d/All selections.
Today is the local civil-calendar interval from midnight to now and is never
an alias for rolling 24h. Finite-period views use bounded SQLite working sets;
All remains the explicit full-history view. Startup first draws a loading shell
while reliability reconciliation and diagnostics refresh on independent
background paths.

They also provide
previous-period comparisons, per-handler counts, terminal states, and
coverage warnings. Trend, regression, and revision panels explicitly say when
history, samples, coverage, or a prior revision are unavailable; they never
invent evidence. Risk ranking is not percentage-only: it presents failure rate
with its terminal sample count and combines sample confidence, coverage,
recency/trend, and impact so a 1/1 result does not automatically outrank a
meaningful mature failure history. Bounded fingerprints use only admitted
status categories such as non-zero exit, timeout, protocol failure, or
execution failure—never raw error streams.

Every failure percentage includes its terminal sample count. Partial,
incomplete, unsupported, or unknown coverage never appears as `0.00% healthy`.
`report --read-only` is useful for inspecting an existing HookStat data root
without creating a ledger or spool.

### Reliability Center keyboard reference

| Key | Action |
| --- | --- |
| `Tab` | Move focus between navigation and content. |
| `↑`/`↓` or `k`/`j` | Move the focused route or selected hook; scroll Detail. |
| `PgUp`/`PgDn` | Page a long Hooks list or Detail. |
| `Enter` / `Esc` | Open the selected hook / return to Hooks. |
| `/`, `f`, `s` | Search, toggle failed-only, or change Hooks sort. |
| `t`, `1`, `7`, `3`, `a` | Request Today, 24h, 7d, 30d, or All history (or apply Settings with `a`). |
| `r`, `q` | Request asynchronous refresh / cleanly quit. |

Settings stages `auto`, `en-US`, or `zh-CN` and a color preference. Applying
the setting changes the next frame and persists the choice without losing the
current route, selection, search, filter, sort, or requested window. Locale
precedence follows explicit `--lang`, `HOOKSTAT_LANG`, the saved preference,
system locale, then English. Machine JSON keys and stable handler keys remain
locale-neutral.

`diagnostics export` previews by default and writes only with `--apply`. Its
sanitized JSON excludes prompts, payloads, credentials, raw hook commands,
stdout/stderr, and private session content.

`hookstat preview-fixture [--json]` is sanitized deterministic development data,
not Owner Codex history.

## Development and release checks

```powershell
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --locked
cargo build --locked
cargo package --locked
cargo publish --dry-run --locked
```

Actual crates.io publication, a `v0.2.0` tag, and GitHub Release creation remain
separate Owner-authorized actions.

See the architecture, ADRs, and execution contracts under `docs/`,
`dev_governance_files/`, and `goals/`.

## License

MIT.
