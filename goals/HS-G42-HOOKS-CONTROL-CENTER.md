# HS-G42 — Hooks Control Center

## Objective

Replace the analytics-first Hooks list with the v0.4 runtime-catalog-first Hooks Control Center while preserving the existing shared Human interface contract.

## Preconditions

```text
G41=PASS
LIVE_RUNTIME_HOOK_CATALOG=PASS
```

## Navigation

Implement the three-level mental model:

```text
Events
  ↓ Enter
Handlers for selected event
  ↓ Enter
Hook Detail
```

Do not require users to infer current runtime configuration from historical reliability rows.

## Event page

Show at minimum:

```text
Event
Installed
Active
Review
Health
Description
Issues where applicable
```

Installed/Active/Review come from the current runtime snapshot. Health comes from joined HookStat reliability and must not overwrite runtime truth.

## Handler page

Show current handler status before reliability:

```text
Enabled state
Human label/fallback
Source class
Handler type/mode
Trust/review/managed state
Reliability health summary
```

The list must include handlers with no HookStat history.

## Detail page

Order sections:

1. Runtime configuration;
2. Reliability summary;
3. Observation history;
4. Advanced intelligence.

Runtime configuration must cover the pinned Codex parity fields from `docs/design/HOOKS_CONTROL_CENTER_SPEC.md`.

Long command/matcher/source values must wrap/scroll safely. Do not substitute fingerprints for the Human runtime value when the runtime value is available in the ephemeral snapshot.

## Current/historical separation

The UI must visually distinguish:

```text
current installed state
from
historical HookStat observations
```

Do not mix historical-only hooks into the current handler list.

## Interaction

Preserve established v0.3 Human interface behavior:

- Up/Down and j/k;
- Enter/Esc local navigation;
- `?` Help;
- press-only key policy;
- responsive layout;
- bilingual UI;
- footer grammar.

Runtime-catalog refresh must have an explicit discoverable action.

## No write parity yet

G42 is read/control-center presentation. It does not gain permission to mutate Codex hooks. G44 owns safe write parity.

## Deterministic tests

Cover:

- event list with all current events;
- zero handlers for an event;
- review-needed counts;
- runtime issues;
- installed/observed;
- installed/unobserved;
- ambiguous join;
- historical-only exclusion;
- long command/matcher/source;
- command/MCP/Prompt/Agent;
- managed handler;
- Chinese/English;
- wide and narrow terminal sizes;
- scroll/selection persistence across refresh.

## Acceptance

```text
HOOKS_CONTROL_CENTER=PASS
CODEX_HOOKS_INFORMATION_PARITY=PASS
CURRENT_RUNTIME_TRUTH_PRIMARY=true
INSTALLED_UNOBSERVED_VISIBLE=true
HISTORICAL_ONLY_NOT_IN_CURRENT_LIST=true
LONG_PRESENTATION_FIELDS_USABLE=true
SHARED_HUMAN_INTERFACE_CONTRACT=PASS
PRODUCT_WRITE_MUTATIONS_ADDED=false
CI=PASS
INDEPENDENT_REVIEW=PASS
```

## Next

Converge with G43 Human Reliability Presentation, then proceed to G44.
