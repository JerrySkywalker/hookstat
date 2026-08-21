# HookStat v0.2 View Model

Status: design only. This document does not change the v0.1 TUI or runtime behavior.

## Purpose

The v0.2 Reliability Center needs a typed boundary between HookStat's ledger/analytics/domain model and Ratatui widgets. A view model is a privacy-safe, locale-neutral projection prepared outside rendering. It contains semantic values and stable IDs, not terminal `Style`, translated prose, raw hook commands, prompts, tool payloads, or database handles.

## Application navigation

Top-level routes:

```text
Overview
Hooks
Diagnostics
Settings
```

`Hook Detail` is a child route of `Hooks`:

```text
Hooks { selection, search, filters }
  -> Hook Detail { handler_key }
  -> Back restores Hooks selection/search/filters
```

The title/navigation/content/footer shell follows `JERRY_TERMINAL_UI_SYSTEM.md`.

## Common application state

Conceptual state:

```text
ReliabilityCenterState {
  route: Route
  nav_selection: TopLevelRoute
  locale: ResolvedLocale
  theme: ThemeChoice
  refresh: RefreshState
  overview: Resource<OverviewViewModel>
  hooks: Resource<HooksViewModel>
  diagnostics: Resource<DiagnosticsViewModel>
  settings: SettingsViewModel
  notice: Option<UiNotice>
}
```

`Resource<T>` is one of Loading, Empty, Ready, or Error with an optional last-good value, as defined by the shared UI contract. A refresh result is generation-tagged and merged on the UI thread. Renderers never open the ledger.

Stable selection rule:

- Select handlers by `(runtime, handler_key)`, not vector index.
- After refresh, preserve that identity if it remains visible.
- If it disappears because of a filter or new data, select the nearest visible row deterministically and announce the changed result count in metadata.

## Existing and future data boundary

HookStat v0.1 already provides:

- `HookInvocation` with runtime, evidence, coverage, internal handler identity, terminal status, duration, and bounded error fingerprint;
- per-handler `HandlerAggregate` with runs, failure samples, failures, rate, previous-window delta, terminal breakdown, and supported latency percentiles;
- `MachineReport` with qualification, handlers, recent failures, malformed receipts, and incomplete receipts;
- 24h, 7d, 30d, and All windows.

The following v0.2 projections do not yet exist and must not be invented by rendering:

- human `DisplayIdentity` separated from the internal handler key;
- cross-handler/runtime summary cards;
- risk score and trend series;
- diagnostics snapshot;
- interface preference store;
- revision comparison history beyond the current per-invocation revision field.

Until a goal implements and tests one of these projections, the corresponding field is `Unavailable` or omitted with a reason key.

## Overview

### User goal

Answer quickly: Is the observed hook system reliable, how complete is the evidence, what changed, and which hooks deserve attention?

### Required data

```text
OverviewViewModel {
  window
  runtime_summaries[] {
    runtime
    evidence_source_class
    coverage
    total_runs
    terminal_sample_count
    failed_runs
    failure_rate
    previous_window_delta?
    health
  }
  highest_risk_hooks[] {
    internal_ref
    display_identity
    event
    failed_runs
    sample_count
    failure_rate
    trend?
    risk?
    coverage
  }
  incomplete_receipts
  malformed_receipts
  generated_at
}
```

`health` is a semantic interpretation that must consider coverage. Zero terminal samples or incomplete coverage cannot become `Healthy` merely because the mathematical failure percentage is zero.

### Output

A Reliability Center dashboard:

- runtime and evidence status;
- coverage;
- total runs;
- failures and failure rate with the sample count;
- health/degraded state in text and color;
- highest-risk hooks;
- refresh age and stale/error status.

### View state

- Window: 24h / 7d / 30d / All.
- Selected risk hook by stable identity.
- Loading/empty/ready/stale-error resource state.
- Optional information overlay explaining coverage and denominators.

### Navigation behavior

- Up/Down selects a risk hook when the content list has focus.
- Enter opens that Hook Detail.
- Window shortcuts change the projection and request an asynchronous refresh if the needed snapshot is not cached.
- `r` requests refresh; existing data remains visible as stale/loading.
- Back returns focus to navigation; quit follows the shared contract.

## Hooks

### User goal

Find a human-readable hook, compare its reliability to peers, and open a detailed explanation without handling opaque IDs first.

### Required data

```text
HooksViewModel {
  window
  rows[] {
    internal_ref
    display_identity
    runtime
    event
    coverage
    runs
    failed_runs
    sample_count
    failure_rate
    trend?
    risk?
    latest_revision
  }
  search
  filters { runtime?, event?, coverage?, risk?, failures_only? }
  sort { field, direction }
  total_before_filter
}
```

### Output

A selectable hook list whose primary label is `display_identity.display_name`. Event, runtime, rate with sample count, trend, risk, and coverage are separate typed columns/metadata. The internal key appears only as secondary metadata or in detail.

### View state

- Stable selected handler reference.
- Search input and cursor state.
- Typed filters and deterministic sort.
- Horizontal/compact column policy derived from terminal width.
- Loading/empty/ready/stale-error resource state.

Empty search results and an empty ledger are different states with different explanation keys.

### Navigation behavior

- Up/Down selects a row.
- Enter opens Hook Detail.
- `/` edits case-insensitive display-name/event search; Esc cancels edits without discarding the accepted query until explicitly defined by G01.
- `f` opens/cycles filters.
- Back closes search/filter overlays first, then returns focus to top-level navigation.
- Refresh preserves selection by internal reference.

## Hook Detail

### User goal

Understand one handler's identity, evidence quality, reliability history, failure modes, latency support, and revision context.

### Required data

```text
HookDetailViewModel {
  internal_ref { runtime, handler_key }
  display_identity
  revision
  source_label
  event
  execution_mode
  coverage
  qualification
  windows[] { window, runs, failed_runs, sample_count, failure_rate, delta? }
  terminal_breakdown
  latency? { p50, p95, p99, support_complete }
  recent_failures[] { occurred_at, status, bounded_fingerprint? }
  trend_7d?
  trend_30d?
  revision_comparison?
}
```

Raw commands, source paths, prompt/tool content, stdout/stderr, credentials, and native private payloads are prohibited.

### Output

- Human display name first.
- Internal identity and revision as secondary metadata.
- Runtime/event/source/coverage facts.
- All supported time-window rates with sample counts.
- Terminal-status breakdown that keeps Blocked/Stopped distinct from failures.
- Latency only when duration coverage is complete under the existing analytics rule.
- Recent failures using bounded taxonomy.
- Trend and revision comparison only after G05 proves them.

### View state

- Selected section or scroll offset for constrained terminals.
- Loading/empty/ready/stale-error state for the exact handler reference.
- Optional coverage or identity explanation overlay.

### Navigation behavior

- Back returns to Hooks with its search, filters, and selected identity intact.
- Up/Down scrolls or selects within the active detail section.
- `r` refreshes the exact detail plus affected summary/list projections.
- Search/filter commands are inactive unless a detail-local list explicitly supports them.

## Diagnostics

### User goal

Understand whether HookStat can observe the runtime safely and what non-destructive next action is available.

### Required data

```text
DiagnosticsViewModel {
  overall_health
  checks[] {
    id
    title_key
    status
    explanation_key
    safe_metadata[]
    next_action_key?
  }
  installation_status
  runtime_detection[]
  trust_status
  instrumentation_status
  receipt_storage_status
  ledger_health
  coverage
  refreshed_at
}
```

Check IDs and machine statuses remain locale-neutral. Human titles, explanations, and next actions are locale keys.

The view model must be assembled from read-only or already-admitted bounded operations. Merely opening or refreshing Diagnostics never applies instrumentation, changes trust, edits `hooks.json`, restores configuration, or exposes private backup material.

### Output

A truthful operational checklist grouped by installation, runtime, evidence source, storage, and coverage. Each warning/failure includes why it matters and a safe next step. Unsupported or owner-required actions are explicit, not silently executed.

### View state

- Stable selected check ID.
- Optional detail/help overlay.
- Loading/empty/ready/stale-error resource state.
- Sanitized export preview state belongs to G04, not the initial TUI foundation.

### Navigation behavior

- Up/Down selects checks.
- Enter opens explanation/details, not mutation.
- Back closes detail.
- `r` requests a read-only asynchronous diagnostic refresh.
- Any future action key must go through a separately typed safety/preview/apply contract; G04 does not inherit authorization to mutate Codex.

## Settings

### User goal

Choose the Human interface language and presentation preferences safely, see changes immediately, and persist only after explicit Apply.

### Required data

```text
SettingsViewModel {
  accepted { language, color, reduced_motion, theme }
  draft { language, color, reduced_motion, theme }
  locale_resolution { locale, source }
  dirty
  conflict
  save_state
}
```

Only settings implemented and governed by G00/G03 may appear. Unsupported future settings are omitted rather than shown as non-functional controls.

### Output

- Language: auto / en-US / zh-CN.
- Color policy: auto / always / never.
- Reduced motion if the shared UI introduces motion.
- Theme choice after G00 establishes supported themes.
- Local-only persistence explanation.
- Dirty/conflict/save status.

### View state

- Accepted persisted baseline separate from in-memory draft.
- Focused field.
- Dirty state.
- Concurrent-preference conflict.
- Apply/revert/discard confirmation state.

Language changes update the next frame from the draft. No file is written until Apply.

### Navigation behavior

- Up/Down selects fields.
- Enter focuses/unfocuses a field.
- Left/Right changes the focused value.
- Apply uses an explicit context key defined in the footer; Revert restores the accepted baseline.
- Quit with a dirty draft requires explicit discard or cancel.
- Back exits field edit before leaving the view.

## View-model construction

The refresh worker should build one immutable snapshot per generation:

1. scan and ingest admitted HookStat-owned receipts using existing semantics;
2. read accepted ledger records or bounded queries;
3. run analytics outside the UI thread;
4. resolve sanitized display identity through the G02 boundary;
5. collect read-only diagnostics through the G04 boundary when requested;
6. return view models with generated time, coverage, and safe error classification.

The implementation may optimize ledger queries later, but v0.2 renderers must not receive a `Ledger`, `Connection`, `ReceiptSpool`, raw Codex configuration, or proxy manifest.

## Required invariants

```text
FAILURE_RATE_WITH_SAMPLE_COUNT=true
INCOMPLETE_COVERAGE_HEALTHY_ZERO=false
BLOCKED_STOPPED_AUTO_FAILURE=false
UNSUPPORTED_DATA_INVENTED=false
SELECTION_USES_INTERNAL_STABLE_ID=true
DISPLAY_ID_NOT_USED_FOR_DEDUP_OR_TRUST=true
RAW_PRIVATE_CONTENT_IN_VIEW_MODEL=false
DATABASE_IN_RENDER=false
CODEX_MUTATION_FROM_VIEW=false
```
