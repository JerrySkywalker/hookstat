# HS-G34 Codex Native L1 Qualification

## Result

| Field | Value |
| --- | --- |
| Qualification status | `PASS` |
| Native admission state | `Qualified` |
| Source coverage | `IdentityLimited` |
| Production activation | `false` |
| Ordinary-session attach tested | `false` (HS-G37 scope) |
| HookStat as Codex launcher | `false` |

This qualification used a disposable `CODEX_HOME`, a disposable workspace, and
the official Codex App Server. HookStat did not proxy, wrap, launch, or join
the observed hook execution path. The Owner's Codex configuration was not read
as a fixture or changed.

## Tested implementation and protocol pin

| Item | Exact evidence |
| --- | --- |
| Codex CLI | `codex-cli 0.149.0` (`@openai/codex` 0.149.0) |
| Release tag | `rust-v0.149.0`, annotated tag object `a4e15bf371341b067c8278d3b70b1a8c7b3d793e` |
| Tested source commit | `758ef40f50c1a458425c7cfbf1eb12cbc07af0b0` |
| App Server v2 schema SHA-256 | `9B3DE71A5A2FFC980B792A18AA8F8DEC3F85F48829560222A0264FE494B679A9` |
| Protocol hook source blob | `41e34a3f8f11f34c12c8f880f029e734442f0400` (`codex-rs/app-server-protocol/src/protocol/v2/hook.rs`) |
| Dispatcher source blob | `a024f75939becc3c0bbe2d17b161ed8c85949905` (`codex-rs/hooks/src/engine/dispatcher.rs`) |
| Core lifecycle source blob | `af2a31e9445a745dd14ab643116166451669554b` (`codex-rs/core/src/hook_runtime.rs`) |

At qualification time, upstream `main` was
`7b5b3bd5a2418a5e142449c9ab95e057d14bc98a`, diverged from the tested release
by 119 commits. Its protocol hook source blob was unchanged, while its
dispatcher and core lifecycle blobs differed. This implementation deliberately
supports the exact tested version only; it does not claim forward compatibility.

## Capability result

| Capability | Result | Qualification meaning |
| --- | --- | --- |
| Invocation start | Proven | `hook/started` is observed and normalized. |
| Terminal result | Proven | `hook/completed` terminal status is observed and normalized. |
| Stable handler attribution | Not proven | Codex run IDs include source path and display order. |
| Duration | Proven | `durationMs` is retained in canonical evidence. |
| Source scope | Proven | Hook scope is normalized without retaining source paths. |
| Revision attribution | Proven | Current `hooks/list.currentHash` joins ephemerally to a revision reference. |
| Ordering/correlation | Proven | Thread, turn, and runtime run identity correlate the lifecycle. |
| Replay/delivery | Not proven | The protocol gives no replay/delivery guarantee. |
| Event-surface completeness | Not proven | The tested lifecycle surface is synchronous-hook-only. |
| Privacy boundary | Proven | Wire private fields never enter canonical evidence or persistence. |
| Version compatibility | Proven | Exact `0.149.0` source/schema pin only; integration rejects incompatible versions. |

`Qualified` therefore does not mean `Admitted`: the G29 admission model remains
truthful while stable handler identity is unproven.

## Identity and terminal semantics

The exact dispatcher builds a run ID from event name, declaration display order,
and source path. It is per run and sensitive to source location and declaration
ordering; it is not a proved HookStat stable handler key. The integration hashes
this runtime identifier only for ephemeral location correlation. The normalizer
derives an opaque current-configuration revision reference from the active
`hooks/list.currentHash`, also without persisting the source path or command.

On same-configuration App Server restart, the location and revision references
matched. After a controlled handler configuration change, the positional
location reference remained while the revision reference changed. This proves
revision attribution for the current active catalog, not stable handler identity
or historical catalog replay.

Codex terminal values map exactly as follows: `completed` to Completed, `failed`
to Failed, `blocked` to Blocked, and `stopped` to Stopped. No completion is
invented for missing evidence; TimedOut, ProtocolFailure, Incomplete, and
Unknown remain unavailable unless separately evidenced. The existing failure-rate
denominator is unchanged.

## Controlled protocol proof

The ignored integration test constructs a temporary configuration with two
synchronous handlers and invokes a one-shot App Server. It records only bounded
lifecycle fields in memory, then immediately normalizes them. It proved:

- 2 `HookStarted` notifications;
- 2 matching `HookCompleted` notifications;
- both Completed and Failed outcomes;
- duration on every completed run;
- distinct runtime location references for two handlers on one event;
- correlation through the accepted G29 `EvidenceCorrelator` to two
  qualification-only `HookInvocation` values;
- restart and config-change behavior described above.

The test command uses only temporary, generic `cmd /c exit <code>` handlers.
It persists no prompts, assistant text, tool input/output, standard streams,
commands, credentials, tokens, or full private paths.

## Code boundaries

- [`src/native.rs`](../../src/native.rs) contains runtime-neutral contracts:
  capability probe, reader, normalizer, and identity resolver.
- [`src/runtime/codex.rs`](../../src/runtime/codex.rs) owns private Codex wire
  types, opaque reader cursor state, source qualification, and normalization.
- [`src/evidence.rs`](../../src/evidence.rs) remains wire-type-free and retains
  only the accepted Native and IPC evidence transports.
- [`tests/codex_native_evidence.rs`](../../tests/codex_native_evidence.rs)
  prevents private wire fields and Codex type names from entering the evidence
  core. [`tests/codex_native_controlled_protocol_e2e.rs`](../../tests/codex_native_controlled_protocol_e2e.rs)
  supplies the live disposable App Server proof.

The separate sanitized machine receipt is
[`runs/HS-V031-G34-NATIVE-EVIDENCE-QUALIFICATION-001/qualification.json`](../../runs/HS-V031-G34-NATIVE-EVIDENCE-QUALIFICATION-001/qualification.json).
