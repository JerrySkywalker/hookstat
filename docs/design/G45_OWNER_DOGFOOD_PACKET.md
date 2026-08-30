# G45 Owner Dogfood Packet — Hooks Control Center

This packet prepares, but does not perform or record, the Owner-only G45
visual acceptance. It contains no Owner hook values, screenshots, commands,
matchers, source paths, prompts, or tool payloads.

```text
G45_OWNER_VISUAL_CHECK=PENDING
CODEX_BASELINE=rust-v0.151.0 / 78c290807ce710180111df227df3b7a4fe845452
PRODUCT_ACCEPTED_MAIN=cf5be21c134fbfa83b9c6a0adf5cc68800d8e4dd
BUILD_COMMAND=cargo build --locked
LAUNCH_EN_US=.\target\debug\hookstat.exe tui --lang en-US
LAUNCH_ZH_CN=.\target\debug\hookstat.exe tui --lang zh-CN
SAFE_WRITE_UX=UPSTREAM_UNAVAILABLE
OWNER_CONFIGURATION_MUTATION=NOT_AUTHORIZED
```

Budget approximately 20–45 minutes. The Owner performs visual judgment only;
the deterministic fixture matrix and interaction checks are already prepared
separately and contain only sanitized values.

## Setup

1. Run `cargo build --locked` at accepted main. Open Windows Terminal at a
   wide ZenBook Duo-sized viewport (at least 140x42), then run
   `.\target\debug\hookstat.exe tui --lang en-US`.
2. Repeat the same sequence at 44x44 and with
   `.\target\debug\hookstat.exe tui --lang zh-CN`.
3. Press `r` on Hooks and wait for the explicit current-runtime catalog state
   to settle. A failed refresh must retain a visibly stale catalog; it must not
   erase separately accepted reliability history.

## Codex `/hooks` A/B sequence

1. Record `codex --version`; this dogfood packet applies only when it is the
   pinned `0.151.0` baseline. Launch normal `codex`, open `/hooks`, and inspect
   one representative current hook without mutating it.
2. Exit Codex normally. Launch HookStat with the command above, refresh Hooks,
   and find the same current hook.
3. Compare only information hierarchy: current event/state, enabled/disabled,
   managed/review/trust state, handler type, command or MCP/prompt/agent
   details, matcher, and source. Do not compare or exercise mutation controls.
4. Keep no screenshot, export, or committed note containing Owner hook values.

## Event catalog

1. Use Down to select Hooks, then Enter to focus Events.
2. Check the columns Event, Installed, Active, Review, Health, and Description.
3. Confirm `Interrupt` remains present with reliability unavailable/not
   admitted unless a separately qualified evidence mapping exists.
4. Confirm any runtime-added unknown event remains present rather than being
   hidden.
5. Select a zero-handler pinned event and confirm its installed, active, and
   review counts are all zero without being presented as healthy.
6. Confirm catalog warnings/errors are visible as runtime issues rather than
   reliability failures.

## Handlers and detail

1. Enter an event, verify installed-but-unobserved handlers are listed, then
   Enter a handler.
2. Verify the list exposes enabled/disabled, Human fallback label, source,
   type/mode, trust/review/managed state, and compact reliability state before
   any historical detail.
3. In Hook Detail, verify this section order:
   Runtime Configuration → Reliability Summary → Observation History →
   Advanced Intelligence / Technical Metadata.
4. For representative Command, MCP Tool, Prompt, and Agent handlers, verify
   the runtime-facing fields appropriate to the type. Verify the Hook
   Management section says read-only because no verified external
   enable/disable route exists; managed handlers remain visibly read-only; do
   not attempt a mutation.
5. On the long Command, Matcher, and Source cases, resize at both target
   widths. Confirm wrapping/scrolling preserves the values and the navigation
   footer stays usable.

## Interaction and boundaries

1. Verify Up/Down and j/k navigate local rows, Enter descends, Esc returns
   exactly one local level, and `?` opens Help.
2. Switch time periods and verify no runtime catalog rediscovery occurs.
3. Refresh a non-destructive catalog and verify the selected event/handler
   remains selected when its identity still exists.
4. Confirm historical-only handlers remain in Changes/history and never
   appear as currently installed.
5. Do not retain screenshots or exports with Owner-private presentation text.
   Use only the sanitized deterministic fixture coverage for repository
   evidence.

## Required Owner answers

Record short answers to all five questions:

1. Do I still need Codex `/hooks` for basic current-hook information?
2. Is source, command, and matcher understandable?
3. Is current versus history obvious?
4. Are metrics understandable without decoding?
5. Is narrow layout usable?

Do not self-certify these answers. `G45_OWNER_VISUAL_CHECK` remains `PENDING`
until the Owner has completed this packet.
