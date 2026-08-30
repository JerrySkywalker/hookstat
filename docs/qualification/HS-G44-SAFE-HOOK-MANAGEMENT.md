# HS-G44 safe hook management qualification

## Decision

```text
GOAL=G44
CODEX_BASELINE=0.151.0
CODEX_SOURCE_REF=rust-v0.151.0
CODEX_SOURCE_SHA=78c290807ce710180111df227df3b7a4fe845452
READ_PARITY=PASS
WRITE_PARITY=UPSTREAM_UNAVAILABLE
EXTERNAL_ROUTE_PROVEN=false
MANAGED_HOOK_MUTATIONS=0
TRUST_BYPASS=false
UNOFFICIAL_CONFIG_GUESSING=false
CODEX_OWNER_CONFIG_MUTATIONS=0
```

HookStat keeps the current runtime catalog and runtime-detail information
read-only. The runtime detail explicitly explains that no verified external
enable/disable route is available; it exposes no enable, disable, or trust
control.

## Pinned upstream evidence

The audited Codex source exposes enable/disable only on its internal TUI path:

```text
codex-rs/tui/src/bottom_pane/hooks_browser_view.rs:206-223
  managed/review checks -> local enabled flip -> AppEvent::SetHookEnabled

codex-rs/tui/src/app_event.rs:1091-1112
  AppEvent::SetHookEnabled is an internal TUI event
```

No bounded, externally callable App Server operation that is equivalent to
that event was proven. In particular, no route was proven with all of exact
handler identity, stale-state protection, managed-hook refusal, and refreshed
`hooks/list` confirmation.

The official App Server trust-related configuration route remains distinct:

```text
codex-rs/tui/src/hooks_rpc.rs:52-85
  ClientRequest::ConfigBatchWrite
  key_path=hooks.state
  trusted_hash keyed update
  reload_user_config=true
  expected_version=None

codex-rs/tui/src/config_processor.rs:141-164
  generic configuration reload
```

It does not prove safe external enable/disable parity. Its use is therefore
not admitted for a Trust-only HookStat management surface, and this
qualification performs no App Server request.

## Product contract

```text
ENABLE_DISABLE_ROUTE=TUI_INTERNAL_ONLY
TRUST_ROUTE=OFFICIAL_CONFIG_BATCH_WRITE_BUT_NOT_ADMITTED_FOR_ASYMMETRIC_G44
STALE_GUARD=UNPROVEN_FOR_ENABLE_DISABLE
MANAGED_GUARD=PROVEN_ONLY_IN_TUI
POST_WRITE_VERIFY=UNPROVEN_FOR_ENABLE_DISABLE
WRITE_CAPABILITY=UPSTREAM_UNAVAILABLE
```

This is a complete G44 result under the v0.4 contract: read parity remains
available, no unsupported mutation is guessed or attempted, and future writes
require a newly proven official upstream route.

## Regression evidence

`tests/g44_safe_hook_management.rs` locks the unavailable-route surface:

- no HookStat TUI command or app path references the internal Codex event or
  configuration-write primitives;
- the runtime renderer is required to show the read-only unavailable reason;
- the bilingual runtime-detail rendering test asserts that no enable or trust
  control hint is presented.
