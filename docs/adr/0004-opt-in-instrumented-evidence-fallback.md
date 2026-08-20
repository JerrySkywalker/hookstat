# ADR 0004 — Opt-in instrumented evidence fallback

Status: Accepted by Owner architecture decision

## Context

Codex 0.147.0 did not expose a sufficient retrospective durable per-handler
hook-run ledger for passive qualification. Passive evidence remains preferred,
but a release candidate needs a truthful evidence plane without requiring users
to launch `hookstat codex` or run a daemon.

## Decision

Introduce an evidence-source-neutral `InstrumentedEvidenceSource`. It may be
enabled only through explicit dry-run/apply against a selected `hooks.json`
configuration. It replaces one command handler with a HookStat proxy command;
normal daily startup remains `codex`. The proxy preserves stdin, stdout, stderr,
and the original exit code without reading or retaining those payloads. It writes
atomic HookStat-owned start/completion metadata receipts and the ordinary
refresh path ingests those receipts into the existing SQLite ledger.

Exact prestate backups and a manifest with original command strings are private
local control-plane data needed for restore. They are not reliability records,
reports, telemetry, committed fixtures, or network data. Apply is atomic and
idempotent; restore is byte-exact and refuses drift. Inline TOML, plugin, and
managed sources are reported as unsupported coverage rather than mutated.

## Consequences

Coverage is explicitly partial when active sources are unsupported or an
invocation has only a start receipt. Exit code 2 remains `unknown` because Codex
uses stderr content to distinguish several control outcomes and HookStat is
prohibited from inspecting it. Apply never auto-approves trust. A separate,
explicit trust action may use Codex's official App Server only after proving the
exact current HookStat manifest, journal, supported user-handler identity, and
hash; it preserves unrelated state, verifies the reload, and never bypasses
trust enforcement. Passive durable receipts from future runtimes flow through
the same canonical model.

## Effective discovery refinement

Codex App Server `hooks/list` is used only as a short-lived, read-only
effective-configuration discovery plane. HookStat reconciles its positionally
identified handlers with local static discovery using private in-memory source
location data and emits only hashed handler identities and aggregate source
coverage. Plugin and managed handlers may therefore be visible but remain
explicit unsupported mutation coverage. `hooks/list` does not turn into a
passive historical invocation ledger or a HookStat daemon dependency.
