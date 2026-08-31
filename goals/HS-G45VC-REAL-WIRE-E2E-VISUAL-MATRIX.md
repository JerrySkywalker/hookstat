# HS-G45V-C — Real-Wire End-to-End Visual Matrix

## Objective

Prove that an official-shaped Codex v0.151.0 `hooks/list` response produces the correct final HookStat TUI through the complete parser → App → Ratatui frame path.

This goal closes the fixture-realism gap exposed by the first G45 Owner visual failure.

## Preconditions

```text
G45V_A=PASS
G45V_B=PASS
TUI_VISUAL_REGRESSION_CI=PASS
G46R=HOLD
```

Read:

- `docs/qualification/G45_OWNER_VISUAL_FINDING_001.md`;
- `docs/architecture/TUI_VISUAL_REGRESSION_CI.md`;
- accepted G45V-A/B receipts.

## Official-shaped fixture

Add a sanitized fixture pinned to:

```text
CODEX_VERSION=0.151.0
CODEX_TAG=rust-v0.151.0
CODEX_SOURCE_COMMIT=78c290807ce710180111df227df3b7a4fe845452
```

The fixture must preserve qualified API field names/casing and relevant structure.

Known event wire names must use exact v2 camelCase values such as:

```json
{"eventName":"preToolUse"}
```

Never replace real wire shape with presentation-friendly PascalCase in this fixture.

## Fixture privacy

The fixture is synthetic and sanitized.

Forbidden committed data:

- Owner command strings;
- Owner source paths;
- Owner matcher expressions;
- real trust hashes;
- private plugin names where identifying;
- prompts/tool payloads;
- project paths.

Use clearly artificial values.

## Required fixture coverage

Include enough data to prove the complete path for:

- a known event with real handlers (`preToolUse` preferred);
- multiple handler types where practical;
- enabled/disabled state;
- trust/review/managed state;
- `interrupt`;
- one future unknown event;
- warning/error context where useful;
- long sanitized command/matcher/source presentation fields;
- at least one installed-but-unobserved join case when combined with synthetic reliability history.

The fixture need not model every Codex capability if separate deterministic fixtures already cover it. It must model the wire characteristics that component-only tests can miss.

## End-to-end test path

The test must begin at the product parser boundary or an exact equivalent:

```text
fixture JSON
  ↓
RuntimePresentationSnapshot::from_codex_hooks_list
  ↓
App apply_runtime_catalog
  ↓
optional synthetic reliability join
  ↓
production rendering::draw
  ↓
TestBackend cell grid
  ↓
golden snapshot + invariants
```

Do not bypass parser normalization by manually constructing the final presentation snapshot for this E2E class.

## Event-frame requirements

Prove at minimum:

```text
PRE_TOOL_USE_DISPLAY_COUNT=1
INTERRUPT_DISPLAY_COUNT=1
UNKNOWN_EVENT_DISPLAY_COUNT=1
EVENT_DISPLAY_IDENTITY_DUPLICATES=0
INSTALLED_COUNT_MATCHES_FIXTURE=true
ACTIVE_COUNT_MATCHES_FIXTURE=true
REVIEW_COUNT_MATCHES_FIXTURE=true
```

Known event descriptions must be localized by the HookStat locale layer.

Unknown future events remain visible exactly once and do not receive fabricated descriptions/reliability semantics.

## Handler-frame requirements

Prove:

- handler count corresponds to the selected event/context;
- enabled/disabled state is visible;
- source/type/mode/trust/review/managed state is visible;
- installed-but-unobserved remains visible;
- no handler from a different context is silently merged into the current one;
- long presentation fields remain bounded/usable in the chosen canonical geometries.

## Detail-frame requirements

Prove section order remains:

```text
Runtime Configuration
Reliability Summary
Observation History
Advanced Intelligence / Technical Metadata
```

Where raw runtime presentation values are shown, they remain local-only and do not enter snapshots as Owner-private data because fixtures are synthetic.

## Geometry and locale

At minimum run the real-wire E2E class at:

```text
wide + en-US
wide + zh-CN
narrow + zh-CN
```

Add another geometry/locale only if it exercises a distinct layout branch.

## Protocol drift contract

Create one durable qualification marker or fixture metadata block recording the upstream pin.

Future Codex source-pin changes must intentionally requalify this fixture rather than silently reusing the v0.151.0 claim.

If a later Codex version changes event names/handler fields, the fixture should fail or be explicitly superseded.

## Visual CI integration

These E2E frames must run under the G45V-B dedicated visual gate.

A parser/runtime-presentation change that breaks the official-shaped fixture must block the PR.

## Scope boundaries

Do not:

- contact or mutate real Owner Codex configuration;
- fetch live private hooks as test fixtures;
- expand write parity;
- start G46R;
- start experimental runtime adapters.

## Acceptance

```text
G45V_C=PASS
REAL_WIRE_FIXTURE=PASS
REAL_WIRE_TO_FRAME_E2E=PASS
PRE_TOOL_USE_DISPLAY_COUNT=1
INTERRUPT_DISPLAY_COUNT=1
UNKNOWN_EVENT_DISPLAY_COUNT=1
EVENT_DISPLAY_IDENTITY_DUPLICATES=0
KNOWN_DESCRIPTION_LOCALIZATION=PASS
INSTALLED_ACTIVE_REVIEW_COUNTS=PASS
CROSS_CONTEXT_SILENT_MERGE=false
OWNER_PRIVATE_FIXTURE_DATA=0
TUI_VISUAL_CI=PASS
CI=PASS
INDEPENDENT_REVIEW=PASS
```

## Next

Prepare and execute HS-G45R Owner Re-Dogfood from accepted main.
