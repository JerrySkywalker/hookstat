# ADR 0002 — Runtime and EvidenceSource are separate abstractions

Status: Accepted

A coding-agent Runtime may provide multiple evidence surfaces: durable session logs, local databases, live event streams, or telemetry. Therefore an adapter is not hard-coded to one input mechanism.

Analytics consumes normalized HookInvocation records and does not depend on whether Codex evidence originated from rollout JSONL, a local database, App Server, OTel, or another admitted source. Coverage belongs to the evidence source and must survive normalization.

Runtime, hook dialect, and model provider are separate concepts. A DeepSeek Harness runtime using a Codex-compatible hook dialect and an OpenAI model remains `runtime=deepseek-harness`.
