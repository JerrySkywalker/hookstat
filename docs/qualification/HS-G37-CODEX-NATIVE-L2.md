# HS-G37 Codex Native L2 Qualification

## Draft result

| Field | Value |
| --- | --- |
| Target platform | Windows |
| Codex CLI | `codex-cli 0.149.0` |
| Release tag | `rust-v0.149.0` |
| Tested source commit | `758ef40f50c1a458425c7cfbf1eb12cbc07af0b0` |
| Ordinary user launch | `codex` |
| Ordinary-session Native L2 | `UPSTREAM_UNAVAILABLE` |
| HookStat launcher/wrapper | `false` |
| Owner configuration changed | `false` |
| Production fallback | admitted cooperative IPC for that domain, otherwise `NOT_ADMITTED` |

This draft qualification does not downgrade the controlled Native L1 proof.
It distinguishes protocol availability from acquisition: `hook/started` and
`hook/completed` exist in the App Server, but HookStat still needs a supported
way to receive those notifications from the App Server that owns an ordinary
user-launched session.

## Exact upstream evidence

The installed CLI and the already-qualified protocol pin are both 0.149.0. The
annotated `rust-v0.149.0` tag peels to source commit
`758ef40f50c1a458425c7cfbf1eb12cbc07af0b0`.

At that immutable source:

- the app-server daemon contract says its current implementation is Unix-only;
- the ordinary TUI supports either an embedded App Server or a local/remote
  App Server target;
- the default local-daemon probe is compiled only on Unix; its non-Unix
  implementation returns `None`;
- without an explicit remote endpoint or a reusable default daemon, the TUI
  selects its embedded App Server;
- App Server clients receive turn/item events for threads they start, resume,
  or fork. The published surface provides `thread/unsubscribe` but no passive
  `thread/subscribe` operation for an external observer to join an already
  owned ordinary session.

Primary immutable sources:

- <https://github.com/openai/codex/blob/758ef40f50c1a458425c7cfbf1eb12cbc07af0b0/codex-rs/app-server-daemon/README.md>
- <https://github.com/openai/codex/blob/758ef40f50c1a458425c7cfbf1eb12cbc07af0b0/codex-rs/tui/src/lib.rs>
- <https://github.com/openai/codex/blob/758ef40f50c1a458425c7cfbf1eb12cbc07af0b0/codex-rs/app-server/README.md>

Therefore the ordinary Windows CLI provides no supported passive attachment
surface that satisfies the G37 Native L2 contract. HookStat must not launch,
wrap, proxy, or resume the Owner's session to manufacture one.

## Routing consequence

The source representation is fail-closed:

```text
Native L2 UPSTREAM_UNAVAILABLE
  + admitted cooperative IPC integration -> IPC
  + non-admitted transparent shim         -> NOT_ADMITTED
  + no admitted IPC integration           -> NOT_ADMITTED
```

`NOT_ADMITTED` remains a coverage/authority state. It is not serialized as an
evidence transport, and it contributes no denominator evidence.

## Safety boundary

No ordinary Codex process was launched for this audit. No App Server daemon or
remote-control process was started. No live Owner Codex configuration, auth
store, prompt, command, tool payload, or hook output was read or changed.

Unix acquisition remains `NOT_QUALIFIED` by this checkpoint. The existence of
the Unix daemon is not itself a proof that a passive external observer can join
an already-owned ordinary thread without affecting session ownership.

## v0.3.1 release interpretation

For v0.3.1, this is the exact Native availability statement for the qualified
`codex-cli 0.149.0` Windows audit: `UPSTREAM_UNAVAILABLE`. It does not infer an
identical result for newer Codex versions. Until a newer exact version receives
its own Native qualification, HookStat continues to select an admitted named
IPC integration for a domain when one exists, otherwise `NOT_ADMITTED`.
