# ADR 0002 — Runtime and EvidenceSource are separate abstractions

Status: Accepted

A coding-agent Runtime may provide multiple evidence surfaces: durable session
logs, local databases, live event streams, telemetry, or an explicitly enabled
instrumented receipt spool. Therefore an adapter is not hard-coded to one input
mechanism.

Analytics consumes normalized HookInvocation records and does not depend on
whether Codex evidence originated from rollout JSONL, a local database, App
Server, OTel, a passive receipt, or an instrumented receipt. Coverage belongs to
the evidence source and must survive normalization. Passive sources remain
preferred; instrumentation is not encoded as a Codex-only analytics path.

Runtime, hook dialect, and model provider are separate concepts. A DeepSeek Harness runtime using a Codex-compatible hook dialect and an OpenAI model remains `runtime=deepseek-harness`.
