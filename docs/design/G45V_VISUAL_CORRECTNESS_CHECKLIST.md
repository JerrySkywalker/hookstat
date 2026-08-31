# G45V Visual Correctness Checklist

This checklist is the audit surface for the correction train inserted after the first G45 Owner visual failure.

## G45V-A — runtime event identity and localization

- [ ] Replace duplicated bare-string known-event tables with one typed or equivalently explicit semantic mapping.
- [ ] Parse exact Codex v0.151.0 camelCase wire event names.
- [ ] Preserve raw unknown future event names.
- [ ] Separate known runtime presentation identity from optional reliability `HookEvent` identity.
- [ ] Localize known event names independently from reliability support.
- [ ] Localize known event descriptions independently from runtime wire text.
- [ ] `Interrupt` is localized in zh-CN while reliability remains unavailable/not admitted.
- [ ] Same-context semantic event duplicates are impossible.
- [ ] Cross-context events are not silently merged.
- [ ] Exact requested cwd/current context is explicit in production semantics.
- [ ] Installed/active/review counts remain truthful after normalization.
- [ ] Unknown event reliability remains unavailable rather than fabricated.
- [ ] Owner-observed duplicate case has a deterministic regression test.

Required receipt:

```text
G45V_A=PASS
SAME_CONTEXT_SEMANTIC_EVENT_DUPLICATES=0
KNOWN_EVENT_LOCALIZATION=PASS
CROSS_CONTEXT_SILENT_MERGE=false
UNKNOWN_EVENT_DROPPED=false
```

## G45V-B — visual regression harness and CI

- [ ] Render canonical frames through the real product renderer and Ratatui `TestBackend`.
- [ ] Commit explicit golden visual baselines.
- [ ] Baselines preserve complete visible character grids.
- [ ] Baseline update is an explicit developer action.
- [ ] CI never auto-updates visual baselines.
- [ ] Add a separately identifiable visual CI result (`CI / tui-visual` or equivalent stable name).
- [ ] TUI-sensitive risk classification includes `src/tui/**`.
- [ ] TUI-sensitive risk classification includes `src/runtime_presentation.rs`.
- [ ] Presentation-relevant Codex adapter/localization/baseline changes run the visual gate.
- [ ] Add structural invariant `EVENT_DISPLAY_IDENTITY_DUPLICATES=0`.
- [ ] Add structural invariant `SELECTED_ROW_COUNT<=1` where meaningful.
- [ ] Add structural invariant `FOOTER_VISIBLE=true` for normal canonical frames.
- [ ] Add structural invariant `KNOWN_EVENT_ENGLISH_LEAK_IN_ZH_CN=false`.
- [ ] Add structural invariant `RAW_UNIX_MS_VISIBLE=false`.
- [ ] Add structural invariant `ZERO_SAMPLE_HEALTHY_PERCENT_VISIBLE=false`.
- [ ] Add section-order invariant for runtime truth before reliability.
- [ ] Failure diagnostics identify baseline/geometry/locale and provide a bounded frame diff.
- [ ] No Owner-private runtime values enter visual fixtures/artifacts.

Initial baseline set covers representative:

- [ ] 140x58 wide;
- [ ] ~100x32 standard;
- [ ] ~60x30 narrow;
- [ ] 44x44 very narrow/tall;
- [ ] en-US;
- [ ] zh-CN;
- [ ] Overview;
- [ ] Hooks Events;
- [ ] Hooks Handlers;
- [ ] Hook Detail;
- [ ] Changes;
- [ ] Diagnostics;
- [ ] Settings;
- [ ] loading/ready/stale/error classes where visually distinct;
- [ ] long command/matcher/source stress cases.

Required receipt:

```text
G45V_B=PASS
TUI_VISUAL_HARNESS=PASS
TUI_GOLDEN_BASELINES=PASS
TUI_STRUCTURAL_INVARIANTS=PASS
TUI_VISUAL_CI_SEPARATELY_VISIBLE=true
BASELINE_AUTO_UPDATE=false
OWNER_PRIVATE_VISUAL_DATA=0
```

## G45V-C — real-wire parser-to-frame E2E

- [ ] Add a sanitized official-shaped Codex v0.151.0 `hooks/list` fixture.
- [ ] Fixture uses `preToolUse`, not `PreToolUse`.
- [ ] Fixture includes representative handler types.
- [ ] Fixture includes `interrupt`.
- [ ] Fixture includes one future unknown event.
- [ ] Fixture may include sanitized warning/error context.
- [ ] Test begins at `RuntimePresentationSnapshot::from_codex_hooks_list` or the exact product-equivalent parser entry point.
- [ ] Test applies the parsed snapshot to App state.
- [ ] Test renders Events frame.
- [ ] Test renders Handlers frame.
- [ ] Test renders Detail frame.
- [ ] `preToolUse` appears exactly once in the Event frame.
- [ ] `interrupt` appears exactly once.
- [ ] unknown future event appears exactly once.
- [ ] installed count matches fixture handlers.
- [ ] known description is localized in zh-CN and en-US.
- [ ] unknown event remains visible without fabricated reliability.
- [ ] wide and narrow representative E2E frames are covered.

Required receipt:

```text
G45V_C=PASS
REAL_WIRE_TO_FRAME_E2E=PASS
PRE_TOOL_USE_DISPLAY_COUNT=1
INTERRUPT_DISPLAY_COUNT=1
UNKNOWN_EVENT_DISPLAY_COUNT=1
KNOWN_DESCRIPTION_LOCALIZATION=PASS
```

## G45R — Owner re-dogfood

- [ ] Build exact accepted main after G45V-C.
- [ ] Record exact tested main SHA.
- [ ] Record current Codex version; requalify if it has moved beyond the pinned baseline in a material way.
- [ ] Compare Codex `/hooks` to HookStat Events/Handlers/Detail.
- [ ] Confirm duplicate Event rows are gone.
- [ ] Confirm zh-CN known event names/descriptions are localized.
- [ ] Confirm installed/active/review counts are understandable.
- [ ] Confirm long command/matcher/source are usable.
- [ ] Confirm current vs history is obvious.
- [ ] Confirm metrics require no machine-value decoding.
- [ ] Confirm narrow layout is usable.
- [ ] Do not mutate Owner hooks/configuration.

Required primary answer:

```text
DO_I_STILL_NEED_CODEX_HOOKS_FOR_BASIC_INFO=NO
```

Only after G45R passes may G46R begin.
