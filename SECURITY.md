# Security and Privacy

HookStat processes local coding-agent runtime evidence that may contain sensitive prompts, paths, tool arguments, environment details, or credentials.

## Default boundary

- local-first and offline by default;
- no telemetry or network export in v0.1;
- no mutation of Codex configuration, hooks, trust state, or session records;
- ingest only the minimum fields needed for reliability analytics;
- do not persist raw prompts/tool payloads;
- sanitize or fingerprint error material before durable storage when raw text may contain secrets.

Do not report vulnerabilities with real credentials or private session transcripts. Use a minimal sanitized fixture.
