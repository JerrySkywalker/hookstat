# HS-G40 Codex `/hooks` parity baseline

## Scope and immutable source pin

This is the v0.4 parity floor for the current-runtime Hooks Control Center. It
records a read-only source audit; it does not authorize a HookStat write, a
Codex configuration change, or a Codex update.

```text
AUDIT_DATE=2026-08-30
CODEX_CLI_VERSION=0.151.0
CODEX_SOURCE_TAG=rust-v0.151.0
CODEX_SOURCE_COMMIT=78c290807ce710180111df227df3b7a4fe845452
CODEX_HOOKS_SOURCE_PINNED=true
CODEX_OWNER_CONFIG_MUTATIONS=0
CODEX_SELF_UPDATE=false
```

The installed executable was only version-queried (`codex --version`). The
source is the peeled commit for the official annotated `rust-v0.151.0` tag.
Future Codex versions require a new qualification audit; this document does
not claim protocol stability beyond this pin.

| Surface | Pinned official source |
| --- | --- |
| `/hooks` browser event/handler/detail presentation | [`codex-rs/tui/src/bottom_pane/hooks_browser_view.rs`](https://github.com/openai/codex/blob/78c290807ce710180111df227df3b7a4fe845452/codex-rs/tui/src/bottom_pane/hooks_browser_view.rs) |
| TUI read RPC | [`codex-rs/tui/src/hooks_rpc.rs`](https://github.com/openai/codex/blob/78c290807ce710180111df227df3b7a4fe845452/codex-rs/tui/src/hooks_rpc.rs) |
| `hooks/list` request and common types | [`codex-rs/app-server-protocol/src/protocol/common.rs`](https://github.com/openai/codex/blob/78c290807ce710180111df227df3b7a4fe845452/codex-rs/app-server-protocol/src/protocol/common.rs) |
| `HooksListEntry` and `HookMetadata` protocol fields | [`codex-rs/app-server-protocol/src/protocol/v2/plugin.rs`](https://github.com/openai/codex/blob/78c290807ce710180111df227df3b7a4fe845452/codex-rs/app-server-protocol/src/protocol/v2/plugin.rs) |
| Hook event/handler enum exports | [`codex-rs/app-server-protocol/src/protocol/v2/hook.rs`](https://github.com/openai/codex/blob/78c290807ce710180111df227df3b7a4fe845452/codex-rs/app-server-protocol/src/protocol/v2/hook.rs) |
| App Server catalog projection | [`codex-rs/app-server/src/request_processors/catalog_processor.rs`](https://github.com/openai/codex/blob/78c290807ce710180111df227df3b7a4fe845452/codex-rs/app-server/src/request_processors/catalog_processor.rs) |

`hooks/list` accepts `HooksListParams { cwds }` and returns
`HooksListResponse { data: Vec<HooksListEntry> }`. Each catalog entry carries
its `cwd`, `hooks`, `warnings`, and `errors`. G41 consumes that official
read-only surface into the in-memory presentation snapshot defined in
[`RUNTIME_PRESENTATION_SNAPSHOT.md`](../architecture/RUNTIME_PRESENTATION_SNAPSHOT.md).

## Event-level information parity floor

The pinned Codex browser presents an event table, and reports catalog warnings
and errors separately. HookStat G41/G42 must make the following runtime-owned
facts available whenever the pin exposes them:

| Codex `/hooks` fact | Pinned meaning | HookStat v0.4 disposition |
| --- | --- | --- |
| Event | Runtime event name | Visible even if it has no canonical reliability mapping |
| Installed | Number of handlers for the event | Runtime catalog fact |
| Active | Handler is enabled and either managed or trusted | Runtime catalog fact |
| Review | Handler needs trust/review (`Untrusted` or `Modified`) | Runtime catalog fact |
| Description | Runtime event description | Runtime catalog fact |
| warnings | Catalog warning text | Runtime issue, not a reliability failure |
| errors | Catalog error text | Runtime issue, not a reliability failure |

```text
CODEX_HOOKS_INFORMATION_PARITY_IS_FLOOR=true
ALL_RUNTIME_EVENTS_VISIBLE=true
UNKNOWN_EVENT_DROPPED=false
```

The pinned `HookEventName` surface is `PreToolUse`, `PermissionRequest`,
`PostToolUse`, `PreCompact`, `PostCompact`, `SessionStart`, `SessionEnd`,
`UserPromptSubmit`, `SubagentStart`, `SubagentStop`, `Stop`, and `Interrupt`.
G41 must use a runtime-owned event name rather than use `HookEvent` as the sole
representation, so that future names remain visible.

## Handler-level information parity floor

`HookMetadata` exposes a stable-enough snapshot for this pinned source audit.
The current snapshot must preserve the presentation values only in memory.

| Required fact | Pinned `hooks/list` field or derivation | G41 read parity requirement |
| --- | --- | --- |
| Event | `event_name` | Show runtime name |
| Matcher | `matcher` | Show when supplied |
| Source | `source`, `source_path`, and plugin/source context | Show runtime-facing source context; never persist raw path |
| Handler type | `HookHandlerMetadata` variant | Command, MCP tool, Prompt, Agent, or future/unknown label |
| Command | `Command { command, async }` | Show command for Command handler only, in memory |
| MCP Server | `McpTool { server, tool }` | Show server in memory |
| MCP Tool | `McpTool { server, tool }` | Show tool in memory |
| Prompt | `Prompt` variant | Show handler kind without copying prompt content |
| Agent | `Agent` variant | Show handler kind without copying agent content |
| Mode | Command `async` maps to Sync/Async | Show when relevant |
| Timeout | `timeout_sec` | Show when supplied |
| Context | `additional_context_limit` | Show when supplied |
| Trust | `trust_status` | Managed, Untrusted, Trusted, or Modified |
| Enabled | `enabled` | Show explicit state |
| Managed | `is_managed` / trust status | Show explicit state |
| Needs Review | `Untrusted` or `Modified` | Derived current-runtime state |
| Warnings/errors | enclosing `HooksListEntry` | Keep separate from terminal reliability outcomes |

The protocol also carries `key`, `status_message`, `plugin_id`,
`display_order`, and `current_hash`. They may support an internal ephemeral
identity/reconciliation operation but must not be added to a durable HookStat
ledger, receipt, diagnostics export, or network payload merely for display.

## Read and write capability boundary

| Capability | Pinned Codex behavior | G40/G41 disposition |
| --- | --- | --- |
| List catalog | App Server `hooks/list` read RPC | Allowed read-only discovery |
| Toggle | TUI sends `SetHookEnabled`; managed handlers cannot be toggled | Documented only; no HookStat write in G40/G41/G42 |
| Trust selected | TUI trust action records the selected handler key/current hash through App Server configuration write | Documented only; no HookStat write in this train |
| Trust all | TUI batches review-needed handlers through the same configuration-write route | Documented only; no HookStat write in this train |

```text
READ_CAPABILITIES=hooks/list
WRITE_CAPABILITIES=toggle,trust_selected,trust_all
G40_PRODUCT_WRITES=false
G41_PRODUCT_WRITES=false
G42_PRODUCT_WRITES=false
G44_ONLY_FUTURE_WRITE_GATE=true
CODEX_HOOK_ENABLE_DISABLE_MUTATION=false
CODEX_HOOK_TRUST_MUTATION=false
CODEX_CONFIG_MUTATION=false
```

The pinned TUI's internal implementation must not be treated as an external
HookStat authorization. If G44 is separately admitted, it must prove the exact
official write contract, exact current manifest/journal/target preconditions,
and response verification before any owner-authorized action. G44 is not in
scope for this train.

## Interrupt and reliability boundary

`Interrupt` is exposed by the pinned runtime catalog, but the v0.3.1 canonical
`HookEvent` taxonomy does not contain it. This audit found no admitted evidence
mapping that proves its invocation and terminal semantics.

```text
INTERRUPT_RUNTIME_CATALOG_VISIBLE=true
INTERRUPT_CANONICAL_RELIABILITY=UNPROVEN
INTERRUPT_PRESENTATION_ONLY=true
UNKNOWN_EVENT_DROPPED=false
```

G41 must therefore display `Interrupt` and a future unknown event in the
current catalog, with reliability shown as unavailable/not admitted until a
separate reliability-semantics qualification proves a canonical mapping. It
must not manufacture a denominator from catalog visibility.

## Privacy and evidence boundary

This parity matrix is a Human current-state contract, not a third evidence
transport. The current v0.3.1 ledger stores privacy-safe identity/structural
facts rather than raw runtime presentation text. G41 must mechanically retain:

```text
RUNTIME_PRESENTATION_IN_MEMORY_ONLY=true
RAW_RUNTIME_PRESENTATION_LEDGER_WRITES=0
RAW_RUNTIME_PRESENTATION_RECEIPT_WRITES=0
RAW_RUNTIME_PRESENTATION_DIAGNOSTICS_EXPORT=0
RAW_RUNTIME_PRESENTATION_NETWORK_EGRESS=0
SNAPSHOT_DEBUG_LOG_FULL_CONTENT=false
NO_THIRD_EVIDENCE_PATH=true
```

Reliability outcomes continue to come only from separately governed admitted
evidence sources. Catalog warning/error state is current runtime truth and is
not automatically an invocation failure.
