# HookStat v0.4 — Hooks Control Center Specification

## Purpose

This document defines the normative Human-facing Hooks experience for HookStat v0.4.

The Hooks Control Center is not primarily an analytics table. It is the current runtime hook catalog plus HookStat reliability intelligence.

Core principle:

> Runtime Truth First, Reliability Second.

For Codex, HookStat must expose at least the human-readable information available from the official `/hooks` surface, then add historical reliability, coverage, trends, revisions, and diagnosis.

## Product acceptance statement

```text
CODEX_HOOKS_INFORMATION_PARITY=MANDATORY
HOOKSTAT_RELIABILITY_OVERLAY=ADDITIVE
BASIC_RUNTIME_INFORMATION_MAY_NOT_REQUIRE_OPENING_CODEX_HOOKS=true
```

G40 pins one exact Codex source/version baseline and records the parity matrix. Newer runtime fields/events are handled by explicit capability drift, not guessed away.

## Current gap

The v0.3.1 Hooks page is analytics-first. A row is derived from historical HookStat evidence and primarily exposes event, runtime, failure rate, trend, and risk. Hook Detail exposes internal IDs, revisions, run counts, coverage, and reliability intelligence.

That model is insufficient for a user trying to answer:

- What hooks are installed now?
- Which event does each belong to?
- Which are active or disabled?
- Which need review?
- Where did they come from?
- What will they execute?
- Are they managed?
- Are they trusted?
- What matcher, timeout, mode, or context limit applies?

v0.4 reverses the hierarchy.

## Navigation model

The Hooks area has three conceptual levels.

### Level 1 — Event catalog

Required columns/fields when available:

```text
Event
Installed
Active
Review
Health
Description
```

Runtime warnings/errors associated with hook discovery appear in an Issues section and are not silently collapsed into reliability coverage.

Illustrative Human view:

```text
Hooks

Event                  Installed  Active  Review  Health
Before tool executes          2       1       0  Coverage limited
Permission requested          3       1       0  Healthy
After tool executes           2       1       0  Healthy
Before compaction             2       1       0  —
Session starts                3       1       0  Coverage limited
Interrupt                     0       0       0  —
```

The event description remains visible without entering a handler detail.

### Level 2 — Handlers for one event

Each handler row must expose enough status to distinguish current runtime state before reliability:

```text
Enabled marker
Human name / Hook N fallback
Source class
Mode/type summary
Trust/review state
Reliability health summary
```

Examples:

```text
[x] Hook 2   User config   Sync Command   Trusted       Healthy
[ ] Hook 1   User config   Sync Command   Trusted       Not observed
[!] Hook 3   Plugin        MCP Tool       Needs review  Not admitted
[M] Hook 4   Managed       Agent          Managed       Coverage limited
```

### Level 3 — Hook detail

The detail page is divided into ordered sections.

#### A. Runtime configuration

Show, when exposed by the runtime:

```text
Event
Enabled
Managed
Matcher
Source
Handler type
Command
MCP Server
MCP Tool
Prompt / Agent handler classification
Mode
Timeout
Additional context limit
Trust
```

Only fields relevant to the handler type are shown.

Long command/matcher/source values wrap or scroll rather than being replaced with internal fingerprints in the normal Human view.

#### B. Reliability summary

Show:

```text
Coverage
Selected period
Runs
Terminal samples
Failures
Failure rate
P50/P95/P99 duration when supported
Health
Health explanation
Risk
Risk explanation
```

#### C. Observation history

Show:

```text
First seen
Last observed
Latest evidence
Current revision
Revision count
Current vs historical state
```

#### D. Advanced intelligence

Show:

```text
Recent failures
Trend projections
Revision comparison/timeline
Failure fingerprints
Internal identity / full technical identifiers only as advanced metadata
```

## Official Codex `/hooks` parity matrix

The pinned baseline must be audited against the official Codex implementation. At minimum the current parity contract includes:

| Runtime field/capability | HookStat v0.4 |
| --- | --- |
| Event name | MUST show |
| Installed count | MUST show |
| Active count | MUST show |
| Needs-review count | MUST show |
| Event description | MUST show |
| Discovery warnings/errors | MUST show |
| Enabled | MUST show |
| Managed | MUST show |
| Needs review | MUST show |
| Matcher | MUST show when present |
| Source | MUST show |
| Command handler text | MUST show locally when runtime exposes it |
| MCP server/tool | MUST show |
| Prompt handler type | MUST show |
| Agent handler type | MUST show |
| Sync/Async mode | MUST show |
| Timeout | MUST show |
| Additional context limit | MUST show |
| Trust status | MUST show |
| Toggle | P1, only through proven official route |
| Trust selected/all | P1, only through proven official route |

Read parity is mandatory. Write parity is conditional on an externally usable official mutation mechanism.

## Runtime event compatibility

The current HookStat canonical `HookEvent` is a reliability taxonomy and must not become the only way to render the live runtime catalog.

v0.4 presentation should distinguish:

```text
RuntimeEventDescriptor
CanonicalHookEvent (optional mapping)
```

Rules:

1. Every event returned by the runtime catalog is visible.
2. Known events map to canonical analytics semantics.
3. Unknown/new runtime events remain visible with reliability unavailable rather than disappearing.
4. Current Codex `Interrupt` is explicitly audited. If invocation/terminal semantics are provable, add it to the canonical `HookEvent`; otherwise display it catalog-only with an explicit reliability state.

This prevents HookStat from falling behind every time Codex adds one event.

## Runtime catalog × reliability join

The current catalog is authoritative for what exists now.

Conceptually:

```text
CURRENT_RUNTIME_CATALOG
LEFT JOIN
HOOKSTAT_RELIABILITY_HISTORY
```

Join results:

### Installed + observed

Show runtime truth and reliability intelligence.

### Installed + not observed

Show the hook completely. Reliability section says, for example:

```text
Reliability: Not observed in selected period
Coverage: NOT_ADMITTED / insufficient evidence / applicable truthful state
```

Do not hide the row or show `0.00% healthy`.

### Join ambiguous

Show runtime truth. Reliability section says the historical join is ambiguous and does not attribute history optimistically.

### Historical + not currently installed

Do not mix into the current event/handler list. Keep it in Changes/history with explicit `Historical / no longer installed` status.

## Human time specification

Normal TUI must never print Unix milliseconds as primary Human text.

Required formatter outputs local time and optionally relative age:

```text
2026-08-30 09:22
2026-08-30 09:22 (12 minutes ago)
Yesterday 21:04
```

Exact style may vary by locale, but the values must be immediately understandable.

Raw epoch values may remain in machine-readable JSON/debug outputs.

Fields covered include:

```text
first_seen
last_seen
latest_evidence
failure occurrence time
revision timeline boundaries
change occurrence time
```

## Metric scope specification

The existing UI can present numbers from different scopes without making that difference obvious. v0.4 requires explicit scope semantics.

Normative scopes:

```text
Selected window / all revisions
Selected window / current revision
All time / all revisions
Terminal-sample denominator
```

Default Hooks list reliability should use one documented scope consistently. Recommended default:

```text
LIST_SCOPE=selected window / all observed revisions of the joined handler
```

Hook detail may additionally expose current-revision metrics, but must label them.

Example:

```text
Selected 7 days, all revisions
Runs              227
Terminal samples  221
Failures          2
Failure rate      0.90%

Current revision
Runs                5
Terminal samples    0
Failure rate         — (no terminal samples)
```

This is acceptable. Displaying `runs=5`, `failure 0.00% (sample=0)`, and an unlabeled `7-day sample=227` side by side is not.

## Zero-sample and coverage wording

Forbidden Human presentation:

```text
0.00% healthy (sample=0)
```

Required presentation class:

```text
Failure rate: —
Terminal samples: 0
Status: No terminal samples
Explanation: HookStat observed starts/current configuration but cannot calculate a failure rate for this scope.
```

Coverage explanations should answer what is missing, not merely print `Partial`.

## Risk wording

An opaque score alone is insufficient.

Instead of:

```text
Risk 10
```

show a Human category and reason:

```text
Low risk (10/100)
Reason: no observed failures, but terminal coverage is incomplete.
```

The exact risk model remains the analytics authority; presentation adds explanation without changing its semantics.

## Current vs historical status

Current runtime state and historical evidence state must be visibly separate.

Examples:

```text
Current: Installed, enabled, trusted
History: Observed since Aug 24
```

or:

```text
Current: No longer installed
History: 227 historical terminal samples
```

Do not derive current installation state from whether old ledger rows exist.

## Interaction

Retain the shared Human interface conventions already established in v0.3:

- Up/Down or j/k navigation;
- Enter opens/accepts local context;
- Esc returns/cancels;
- `?` Help;
- press-only key handling;
- explicit dirty-state confirmation;
- bilingual locale policy;
- responsive layout.

Hooks Control Center-specific actions should be discoverable in the footer.

## Safe mutations

v0.4 does not gain permission to rewrite Codex configuration merely because `/hooks` supports toggles.

Required order:

```text
prove official externally usable mutation surface
→ bind exact runtime identity/current hash
→ fixture tests
→ bounded real owner proof
→ admit write action
```

If this proof is unavailable:

```text
READ_INFORMATION_PARITY=PASS
WRITE_PARITY=UPSTREAM_UNAVAILABLE
```

Managed hooks are never mutated by HookStat.

## Privacy

Human-readable runtime presentation fields are local ephemeral UI material.

They must not be written to:

- SQLite ledger;
- HookStat receipts;
- performance receipts;
- diagnostics exports;
- committed fixtures containing owner data;
- remote/network telemetry.

Tests use sanitized synthetic values.

## Responsive layout acceptance

At minimum qualify:

- wide Windows Terminal / ZenBook Duo layout;
- narrower terminal layout;
- Chinese and English;
- long command;
- long matcher/source;
- MCP handler;
- Prompt/Agent handler;
- managed handler;
- review-needed handler;
- many handlers requiring scroll;
- no reliability samples;
- partial coverage;
- historical-only hook.

## A/B acceptance

G45 performs an owned A/B comparison against the pinned Codex `/hooks` baseline.

Final Human acceptance question:

> If the user only opens HookStat, do they still need to open Codex `/hooks` to learn what a current hook is and how Codex sees it?

If yes, information parity has not been achieved.

## Non-goals

- persisting raw command/matcher/source text for analytics;
- inventing runtime fields absent from the official surface;
- hiding unknown runtime events;
- implementing DeepSeek/OpenCode adapters inside v0.4;
- rewriting reliability analytics merely for presentation convenience.
