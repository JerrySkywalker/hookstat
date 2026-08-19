# HookStat Project Charter

## Mission

HookStat is a local-first reliability analytics tool for hooks across coding-agent runtimes. It recovers or observes real hook invocations, normalizes runtime-native outcomes, and answers: how often did each hook run, how often did it fail, how reliable is it over time, and how confident are we in the coverage?

## Terminology

- **Runtime**: coding-agent host such as Codex, DeepSeek Harness, OpenCode, or future Claude Code.
- **Hook dialect**: runtime-native hook protocol/semantics. It is not the model provider.
- **Model provider**: OpenAI, DeepSeek, Anthropic, Google, etc.; orthogonal to Runtime.
- **EvidenceSource**: durable/live/telemetry surface from which hook execution evidence is recovered.
- **HookInvocation**: canonical normalized record representing one handler invocation.
- **Coverage**: confidence class describing what the source can and cannot observe.

## v0.1 product promise

Codex-first. The user continues to launch Codex normally. Later, running `hookstat` ingests available local durable evidence and presents per-handler invocation count, failed invocation count, failure rate, basic terminal-state breakdown, time windows, recent failures, and latency only when supported by evidence.

## Long-term runtimes

1. Codex (v0.1 implementation)
2. DeepSeek Harness (planned second runtime)
3. OpenCode (planned third runtime)
4. Future runtimes without changing the analytics model

## Safety/privacy

HookStat v0.1 is read-only toward coding-agent runtimes, offline by default, and does not persist raw prompts/tool payloads. Incomplete evidence must remain visibly incomplete.

## Non-goals for v0.1

No daemon, launcher wrapper, system tray, Web UI, notifications, automatic repair, active probe, hook doctor, live Codex mutation, DeepSeek Harness implementation, OpenCode implementation, error clustering, handler revision analytics, or crates.io publication without a separate release authorization.
