# HS-G45V-A — Runtime Event Identity & Localization Repair

## Objective

Repair the release-blocking G45 Owner visual defect where one semantic Codex event can appear twice and known event descriptions can leak English text into zh-CN.

This is a product-correctness goal, not a renderer-only cosmetic patch.

## Starting point

```text
BASE_MAIN=c24139842e35f83368db00dbf56d9025817d4a9e
OWNER_FINDING=G45-OV-001
G45_OWNER_VISUAL_CHECK=FAIL
G46R=HOLD
CODEX_PIN=rust-v0.151.0@78c290807ce710180111df227df3b7a4fe845452
```

Read first:

- `dev_governance_files/ROADMAP_V040.md`;
- `docs/qualification/G45_OWNER_VISUAL_FINDING_001.md`;
- `docs/architecture/RUNTIME_PRESENTATION_SNAPSHOT.md`;
- `docs/design/G45V_VISUAL_CORRECTNESS_CHECKLIST.md`.

## Root-cause constraint

Do not treat the Owner symptom as an arbitrary duplicate-render bug.

The known root cause is a mismatch between synthetic PascalCase event strings and the qualified Codex v0.151.0 camelCase wire representation.

Forbidden primary fix:

```text
dedup localized labels in renderer
```

That would hide identity errors and risk collapsing different runtime contexts.

## Required identity model

Separate:

```text
RAW_RUNTIME_WIRE_IDENTITY
KNOWN_RUNTIME_PRESENTATION_IDENTITY
OPTIONAL_RELIABILITY_IDENTITY
```

Implement a typed known-runtime-event mapping or an equally explicit equivalent.

For known Codex v0.151.0 events, map exact camelCase wire names:

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

Do not require reliability `HookEvent` support merely to recognize/localize a runtime event.

## Interrupt

Required semantics:

```text
INTERRUPT_KNOWN_RUNTIME_EVENT=true
INTERRUPT_LOCALIZED=true
INTERRUPT_RELIABILITY_EVENT=UNPROVEN_OR_NONE
INTERRUPT_RELIABILITY_STATE=UNAVAILABLE_OR_NOT_ADMITTED
```

Do not fabricate reliability because the runtime event is known.

## Unknown future events

Unknown runtime event names remain visible verbatim.

Required:

```text
UNKNOWN_EVENT_DROPPED=false
UNKNOWN_EVENT_RAW_NAME_PRESERVED=true
UNKNOWN_EVENT_DESCRIPTION_GUESSED=false
```

## Runtime context

Do not remove `runtime_context` to solve duplicates.

Production discovery requests one exact cwd. If multiple contexts are returned, use an explicit exact-current-context selection or a visibly disambiguated representation.

Required:

```text
SAME_CONTEXT_SEMANTIC_EVENT_DUPLICATES=0
CROSS_CONTEXT_SILENT_MERGE=false
CURRENT_CONTEXT_EXPLICIT=true
```

## Localization

Known event display names and descriptions must use localization semantics.

Required:

```text
KNOWN_EVENT_NAME_LOCALIZED=true
KNOWN_EVENT_DESCRIPTION_LOCALIZED=true
ZH_CN_KNOWN_EVENT_ENGLISH_LEAK=false
```

Do not carry English description strings through the runtime-presentation model as the authoritative Human value when the event is a known semantic event.

## Counts and reliability join

After normalization, preserve truthful:

- installed count;
- active count;
- review count;
- handler ownership;
- reliability join state.

Do not merge handlers from distinct contexts merely because events share a known semantic identity.

## Regression tests

At minimum cover:

1. synthetic completion + real `preToolUse` response produces one semantic event;
2. installed count is the real handler count, not zero;
3. `interrupt` appears once and localizes in zh-CN/en-US;
4. known descriptions localize in both supported locales;
5. unknown future event remains visible exactly once;
6. same-context casing variants cannot create duplicate known events;
7. multiple contexts do not silently merge;
8. existing conservative reliability joins remain intact;
9. raw runtime presentation remains ephemeral/private.

Use official-shaped wire casing in parser fixtures.

## Scope boundaries

Do not:

- begin G45V-B before G45V-A acceptance;
- add write parity;
- mutate Owner hooks/config;
- redesign reliability analytics;
- start experimental runtimes;
- begin G46R.

## Quality gates

During iteration use focused parser/runtime-presentation/localization tests.

At settled candidate:

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --locked
cargo build --locked
git diff --check
```

Use risk-aware CI and an exact-head fresh read-only Supervisor review.

## Acceptance

```text
G45V_A=PASS
EVENT_WIRE_MAPPING=PASS
SAME_CONTEXT_SEMANTIC_EVENT_DUPLICATES=0
KNOWN_EVENT_LOCALIZATION=PASS
ZH_CN_KNOWN_EVENT_ENGLISH_LEAK=false
INTERRUPT_LOCALIZED=true
UNKNOWN_EVENT_DROPPED=false
CROSS_CONTEXT_SILENT_MERGE=false
INSTALLED_ACTIVE_REVIEW_COUNTS=PASS
RAW_RUNTIME_PRESENTATION_PERSISTED=false
CI=PASS
INDEPENDENT_REVIEW=PASS
```

## Next

Begin G45V-B from accepted main.
