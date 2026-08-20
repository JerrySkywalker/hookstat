# HS-V01-PROCESS-CLEANUP-AND-SCOPED-TRUST-RECOVERY-12H-001 checkpoint

## Recovery candidate

This recovery starts from accepted main
`76294634ac3ace6f86879d901961b66a33a46785` and preserves the prior train's
sanitized governance and blocker records. The recovery implementation is
checkpointed on `agent/v01-live-activation-release-001` at
`1db9ace973efffc17ff0544a14410319f199229e` before any new Owner-live mutation.

## Scoped official trust proof

Codex CLI `0.147.0` generated an App Server schema that exposes
`hooks/list`, `config/batchWrite`, `hooks.state`, upsert merge semantics, and
user-config reload. Upstream Codex TUI source uses that exact App Server write
sequence for hook trust.

HookStat now exposes an explicit `hookstat codex instrument --trust` action;
`--apply` cannot grant trust. Trust loads the current private backup, journal,
and manifest; requires the exact applied configuration hash; checks only
supported user `hooks.json` HookStat targets; rejects mismatch, disabled,
managed, plugin, stale, missing, and duplicate targets; upserts only exact
current hashes; reloads; and re-lists until every selected target is trusted.
Its output contains counts only.

Unit fixtures cover twelve managed supported handlers, four plugin handlers,
trusted and untrusted unrelated user hooks, partial prior trust, all-trusted
idempotence, disabled targets, duplicate identities, stale configuration, and
missing manifest state. An isolated temporary-CODEX_HOME integration fixture
against the installed 0.147.0 App Server passed dry-run, write, reload,
verification, idempotent repeat, and exact restore. The Owner `config.toml`
fingerprint was unchanged before and after that isolated fixture.

## Windows process containment proof

The proxy now joins a Windows Job Object with kill-on-close before spawning the
original shell. A normal root-handler completion clears that limit before the
job handle closes, so legitimate background descendants are preserved.

The direct proxy-only termination regression was run five consecutive times.
Each run killed only the proxy PID, proved a handler child/grandchild did not
complete, and proved an unrelated process did complete. The paired normal-exit
fixture proves a deliberate background descendant survives. Existing tests
continue to prove streams, stdin, cwd, environment, full Windows exit codes,
concurrent proxies, and receipt incompleteness on cancelled execution.

## Local candidate gates

Formatting, Clippy with warnings denied, all targets tests, locked build,
package verification, and publish dry-run passed on the recovery candidate.
The installed-App-Server fixture is intentionally ignored in ordinary CI but
was run explicitly and passed in this environment.

An Ubuntu WSL distribution is available but has no Rust toolchain; a Docker
Linux image pull did not complete within its bounded tool timeout. No Linux
process-containment guarantee is claimed by v0.1; exact-head hosted Linux CI
remains required before release.

No Owner-live hook configuration, trust state, session history, prompt, tool
payload, stream content, credential, or raw hook command was committed in this
checkpoint.
