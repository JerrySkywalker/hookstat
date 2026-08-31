# G45 Owner Visual Finding 001 — Duplicate Runtime Events

## Status

```text
FINDING_ID=G45-OV-001
OWNER_VISUAL_CHECK=FAIL
RELEASE_BLOCKING=true
DISCOVERED_ON_MAIN=c24139842e35f83368db00dbf56d9025817d4a9e
CODEX_BASELINE=0.151.0 / rust-v0.151.0 / 78c290807ce710180111df227df3b7a4fe845452
G46R=HOLD
```

This receipt records the first Owner visual acceptance failure after the G45 automated preparation. It is a durable finding, not a screenshot baseline. No Owner-private hook values are recorded here.

## Observed Human symptom

The Hooks Event catalog displayed the same semantic events twice. One set contained synthetic zero-handler rows with descriptions; another set contained the actual installed handler counts. Both sets localized to the same Human event names.

The zh-CN screen also displayed known event descriptions in English, and a known runtime event such as `Interrupt` could remain as a raw English event name when it had no canonical HookStat reliability event.

Required finding classification:

```text
EVENT_CATALOG_SEMANTIC_DUPLICATES=true
KNOWN_EVENT_LOCALIZATION_DEFECT=true
VISUAL_REGRESSION_CI_GAP=true
```

## Root cause supported by source

At the finding baseline, `RuntimePresentationSnapshot::from_codex_hooks_list` seeds the qualified Codex event surface using PascalCase strings such as:

```text
PreToolUse
PermissionRequest
PostToolUse
...
Interrupt
```

The same parser then inserts events from `hooks/list` using the raw `eventName` string.

The qualified Codex v0.151.0 v2 protocol defines `HookEventName` with camelCase serialization, so the real wire values include:

```text
preToolUse
permissionRequest
postToolUse
preCompact
postCompact
sessionStart
sessionEnd
userPromptSubmit
subagentStart
subagentStop
stop
interrupt
```

The presentation map keys events by:

```text
(runtime_context, runtime_event_name)
```

Therefore a seeded key such as:

```text
(cwd, PreToolUse)
```

and a real wire key such as:

```text
(cwd, preToolUse)
```

remain distinct even though the renderer presents them as the same Human event.

## Why renderer-only deduplication is forbidden

The defect must not be patched by deduplicating localized labels in the renderer.

`runtime_context` exists to prevent different cwd/runtime contexts from being silently collapsed, and unknown future events must remain visible even when HookStat has no canonical reliability semantics for them.

Correct repair belongs in the runtime-presentation identity layer:

```text
raw wire name
   -> known runtime semantic identity when recognized
   -> localized presentation identity
   -> optional reliability HookEvent mapping
```

## Localization defect

Known event descriptions are currently seeded as English strings. Those strings are runtime-presentation values rather than localization keys, so they can leak into zh-CN rendering.

Required correction:

```text
KNOWN_EVENT_NAME_LOCALIZED=true
KNOWN_EVENT_DESCRIPTION_LOCALIZED=true
INTERRUPT_LOCALIZED_IN_ZH_CN=true
RELIABILITY_SUPPORT_NOT_REQUIRED_FOR_LOCALIZATION=true
UNKNOWN_EVENT_RAW_NAME_PRESERVED=true
UNKNOWN_EVENT_DESCRIPTION_GUESSED=false
```

## Test-gap assessment

The repository already contains deterministic TUI tests and a G45 automated fixture matrix. The failure therefore demonstrates a test-shape gap rather than an absence of tests.

Existing tests can construct normalized presentation models directly. The missing mandatory path is:

```text
qualified official-shaped hooks/list JSON
        -> parser
        -> App
        -> complete terminal frame
```

The first Owner pass also demonstrates the need for full-frame golden visual snapshots plus semantic structural invariants. Text-fragment assertions are insufficient to catch duplicated rows or certain localization/layout failures.

## Correction train

The finding creates the following release-blocking DAG:

```text
G45V-A Runtime Event Identity & Localization Repair
   ↓
G45V-B TUI Visual Regression CI Foundation
   ↓
G45V-C Real-Wire End-to-End Visual Matrix
   ↓
G45R Owner Re-Dogfood
   ↓
G46R
```

## Closure criteria

This finding closes only when all of the following are true at accepted main:

```text
SAME_CONTEXT_SEMANTIC_EVENT_DUPLICATES=0
KNOWN_EVENT_LOCALIZATION=PASS
ZH_CN_KNOWN_EVENT_ENGLISH_LEAK=false
REAL_WIRE_TO_FRAME_E2E=PASS
TUI_VISUAL_REGRESSION_CI=PASS
OWNER_REDOGFOOD=PASS
```

The first-pass failure remains historical evidence after closure; do not rewrite it as if the original Owner pass succeeded.
