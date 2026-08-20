# Security and Privacy

HookStat processes local coding-agent runtime evidence that may contain sensitive prompts, paths, tool arguments, environment details, or credentials.

## Default boundary

- local-first and offline by default;
- no telemetry or network export in v0.1;
- normal operation never mutates Codex configuration, hooks, trust state, or
  session records; an explicitly authorized `hookstat codex instrument --apply`
  may atomically update only the selected supported configuration root and
  creates a local exact backup for restore. A separate explicitly authorized
  `--trust` may write only exact HookStat-generated user-handler hashes through
  Codex's official App Server mechanism;
- ingest only the minimum fields needed for reliability analytics;
- do not persist raw prompts/tool payloads;
- sanitize or fingerprint error material before durable storage when raw text may contain secrets.

## Opt-in instrumentation boundary

The proxy inherits original stdin, stdout, stderr, working directory, and
environment without decoding or persisting those contents. Durable reliability
receipts contain only opaque invocation metadata, hashed handler identity,
event/source/mode, timestamps/duration, normalized terminal result, and
coverage. Private original command strings and exact configuration backups are
local restore-control material only; they are never placed in the SQLite
ledger, JSON report, test evidence, or repository.

`--apply` can trigger a Codex trust review because it changes a hook command;
it never approves trust itself. `--trust` is separate and fails closed unless
the current journal, backup, manifest, config hash, runtime source path,
handler identity, current hash, and effective state prove only the exact
supported HookStat targets. It uses `hooks/list` followed by the official
`config/batchWrite` `hooks.state` upsert, reloads user config, and verifies
`trusted` afterward. It never bypasses enforcement or edits a trust database.
Managed and plugin sources are not rewritten or trusted. Use
`--restore --config-root <root>` to recover the exact recorded prestate after
HookStat verifies no configuration drift.

Do not report vulnerabilities with real credentials or private session transcripts. Use a minimal sanitized fixture.
