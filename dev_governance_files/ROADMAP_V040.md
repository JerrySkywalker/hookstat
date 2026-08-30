# HookStat v0.4 execution roadmap

## Status

IN_PROGRESS from public HookStat v0.3.1.

```text
G40=ACCEPTED
G41=ACCEPTED
G42=ACCEPTED
G43=ACCEPTED
G44=QUALIFIED_UPSTREAM_UNAVAILABLE
G45=WAITING_G44_ACCEPTANCE
G46R=WAITING
```

```text
PUBLIC_BASELINE=v0.3.1
PUBLIC_MAIN=651620cbc9f204f312fc31efee424c747895927a
PUBLIC_TAG=v0.3.1
V040_PRODUCT_THEME=Hooks Control Center / Human Usability
PRODUCTION_RUNTIME=Codex
EXPERIMENTAL_RUNTIME_REQUIRED_FOR_V040=false
```

v0.4 is a product-usability release, not the productionization of a second runtime. DeepSeek Harness, OpenCode, Claude Code, Agy, and other runtime work proceeds in independent `exp/*` tracks and cannot block v0.4.

## Product thesis

**Runtime Truth First, Reliability Second.**

For Codex hooks, HookStat must first answer the same human questions as Codex `/hooks`:

- which hook events exist;
- how many handlers are installed and active;
- which handlers need review;
- what each handler is, where it came from, and how it is configured;
- whether it is enabled, managed, or trusted;
- which runtime warnings/errors affect the hook catalog.

Only after that complete runtime truth is visible should HookStat add its unique reliability layer: observed runs, terminal samples, failures, latency, coverage, revisions, trends, fingerprints, history, and health explanations.

Normative rule:

```text
RUNTIME_TRUTH_FIRST=true
CODEX_HOOKS_INFORMATION_PARITY_IS_FLOOR=true
CODEX_HOOKS_HUMAN_INFORMATION_PARITY=MANDATORY
HOOKSTAT_RELIABILITY_OVERLAY=ADDITIVE
RUNTIME_TRUTH_MAY_NOT_BE_REPLACED_BY_ANALYTICS=true
```

The official Codex `/hooks` information surface is the v0.4 information-completeness floor. G40 pins the exact Codex source/version baseline used for parity review; the roadmap does not assume one unqualified future Codex version forever.

## Information architecture

The Hooks area becomes runtime-catalog-first:

```text
Runtime hooks/list
      │
      ▼
Ephemeral Runtime Presentation Snapshot
      │
      ├───────────────┐
      ▼               ▼
Current runtime   Historical reliability
truth             ledger/analytics
      │               │
      └───────┬───────┘
              ▼
       Hooks Control Center
```

Current installed hooks are a LEFT JOIN over reliability history, never the reverse.

Required states:

```text
INSTALLED_AND_OBSERVED
INSTALLED_NOT_OBSERVED
INSTALLED_JOIN_AMBIGUOUS
HISTORICAL_NOT_INSTALLED
```

A current hook is never hidden merely because HookStat has no admitted observations for it. Historical hooks that are no longer installed remain available in Changes/History but must not masquerade as current runtime configuration.

## Privacy boundary

Runtime presentation metadata may contain human-readable values that HookStat historically refuses to persist, including source paths, matcher text, command text, MCP server/tool names, and other runtime-owned presentation fields.

v0.4 may show those values locally in the interactive TUI only through an ephemeral presentation snapshot.

```text
RUNTIME_PRESENTATION_IN_MEMORY_ONLY=true
RUNTIME_PRESENTATION_LEDGER=false
RUNTIME_PRESENTATION_RECEIPT=false
RUNTIME_PRESENTATION_DIAGNOSTICS_EXPORT=false
RUNTIME_PRESENTATION_REMOTE_TELEMETRY=false
RAW_PROMPT_CONTENT_PERSISTED=false
RAW_TOOL_CONTENT_PERSISTED=false
RAW_COMMAND_PERSISTED_IN_LEDGER=false
RAW_COMMAND_LEDGER_PERSISTENCE=false
RAW_MATCHER_LEDGER_PERSISTENCE=false
RAW_SOURCE_PATH_LEDGER_PERSISTENCE=false
```

The persistence/privacy contract remains unchanged. See `docs/architecture/RUNTIME_PRESENTATION_SNAPSHOT.md`.

## Codex `/hooks` parity floor

The v0.4 presentation must represent, when the pinned runtime exposes them:

### Event level

```text
Event
Installed
Active
Review
Description
Warnings
Errors
```

### Handler level

```text
Enabled / Disabled
Needs Review
Managed
Event
Matcher
Source
Handler type
Command OR MCP Server/Tool OR Prompt OR Agent
Mode
Timeout
Additional Context limit
Trust
```

Unknown/new runtime event names must remain visible in the current catalog even when HookStat analytics does not yet have canonical reliability semantics for them. Known canonical events are mapped explicitly. Current Codex `Interrupt` support must be audited and, where reliability semantics are provable, added to the canonical event model.

## Human-readable reliability contract

Machine-oriented values may not leak into the normal Human interface merely because they are easy to render.

Required:

```text
RAW_UNIX_MILLISECONDS_IN_NORMAL_TUI=false
RAW_FULL_INTERNAL_HASH_PRIMARY=false
ZERO_SAMPLE_HEALTHY_PERCENT=false
METRIC_SCOPE_EXPLICIT=true
SAME_SCOPE_SAME_METRIC_CONSISTENT=true
```

Human time examples:

```text
First seen     2026-08-24 21:17
Last observed  2026-08-29 20:03 (13 hours ago)
Latest evidence 2026-08-29 20:03
```

Raw epoch values remain available only in machine/debug surfaces where justified.

A failure rate with zero terminal samples renders as an unavailable metric, not `0.00% healthy`.

Risk must include an explanation, not only an opaque number.

## Metric scope

Every reliability metric shown in Hook Detail must make its scope unambiguous. At minimum distinguish:

```text
SELECTED_WINDOW_ALL_REVISIONS
CURRENT_REVISION_SELECTED_WINDOW
HISTORICAL_ALL_TIME
TERMINAL_SAMPLE_DENOMINATOR
```

If two sections use different scopes, the UI labels them. A value such as `runs=5` may not sit beside a trend with `samples=227` without explaining the different scope.

## v0.4 dependency DAG

```text
PUBLIC v0.3.1
      │
      ▼
HS-G40 — Post-release Rebaseline & /hooks Parity Contract
      │
      ▼
HS-G41 — Live Runtime Hook Catalog
      │
      ├─────────────────────────┐
      ▼                         ▼
HS-G42                      HS-G43
Hooks Control Center        Human Reliability Presentation
      │                         │
      └──────────────┬──────────┘
                     ▼
HS-G44 — Safe Hook Management
                     │
                     ▼
HS-G45 — Human UX / ZenBook Duo Dogfood
                     │
                     ▼
HS-G46R — v0.4 Hardening & Release
                     │
                     ▼
                PUBLIC v0.4
```

G42 and G43 may proceed in isolated branches after G41 if their interfaces are frozen. G44 depends on the runtime catalog but write parity is conditional on an official, externally usable Codex mutation surface. Read/information parity is mandatory; unsupported writes must be represented truthfully rather than implemented by filesystem guessing.

## Goal index and estimated effort

| Goal | Scope | Estimated effort |
| --- | --- | ---: |
| G40 | release rebaseline, parity matrix, v0.4 contracts, experiment governance | 2–3 h |
| G41 | ephemeral live runtime hook catalog, event compatibility, join semantics | 4–6 h |
| G42 | event → handlers → detail Hooks Control Center | 4–7 h |
| G43 | human time, metric scope, health/risk explanations, consistency | 4–6 h |
| G44 | safe runtime-owned enable/trust operations where officially supported | 3–5 h |
| G45 | visual/interaction dogfood and Codex `/hooks` A/B parity | 3–5 h |
| G46R | fast-lane release closeout and publication candidate | 2–4 h |
| **Total** | **v0.4 product track** | **22–36 h** |

## G40 — Post-release rebaseline

G40 establishes the new authoritative product contract before implementation. It must:

- record v0.3.1 as public/closed;
- update stale public-version/release-candidate wording;
- pin the exact Codex `/hooks` source/version used for parity;
- freeze the parity matrix;
- audit current HookStat metric-scope anomalies;
- freeze the runtime presentation privacy boundary;
- establish `agent/*`, `fix/*`, `exp/*`, and `promote/*` branch semantics.

If the current `runs/sample/trend` mismatch is a genuine correctness defect rather than a presentation-scope issue, repair it through a dedicated `fix/*` train and consider a maintenance release independently of v0.4.

## G41 — Live runtime hook catalog

Implement an in-memory current-runtime catalog using the official Codex `hooks/list` surface. Required concepts include event descriptors, handler presentation, source presentation, trust/state presentation, runtime issues, and conservative reliability join state.

Do not persist raw runtime presentation fields.

Current/new events that are unknown to HookStat analytics remain visible with reliability `UNAVAILABLE`/`NOT_ADMITTED` rather than disappearing.

## G42 — Hooks Control Center

Replace the analytics-first Hook list mental model with:

```text
Events
  ↓
Handlers for selected event
  ↓
Current Hook detail + Reliability overlay
```

The runtime detail must be usable without opening Codex `/hooks` for missing basic information.

## G43 — Human reliability presentation

Humanize timestamps, hashes, coverage, risk, terminal-sample availability, revision history, and metric scopes. Establish deterministic tests for time formatting, zero-sample states, mixed revision/history scopes, narrow layouts, and bilingual presentation.

## G44 — Safe hook management

Audit Codex's official mutation surfaces for external clients. If a stable, bounded route is proven, support safe enable/disable and trust operations with exact identity/current-hash preconditions. Managed hooks are always read-only.

If an official external write route is unavailable or cannot be proved safe:

```text
READ_PARITY=PASS
WRITE_PARITY=UPSTREAM_UNAVAILABLE
```

and v0.4 may still release. Never guess or directly rewrite plugin/managed configuration to imitate the Codex TUI.

## G45 — Human UX dogfood

Run owned Windows Terminal A/B evaluation of Codex `/hooks` versus HookStat on representative width, locale, and handler cases.

Acceptance question:

> If the user only opens HookStat, do they still need Codex `/hooks` to understand what a current hook is and how the runtime sees it?

If yes, G45 fails.

## G46R — v0.4 release

Use the merged Fast Lane process:

```text
settle code + docs
→ freeze exact candidate SHA
→ hosted CI / independent review / owner dogfood / release gate in parallel
→ no post-freeze commits
→ merge
→ separately owner-authorized publication
```

Do not recreate pre-v0.3.1 candidate churn.

## Exploration tracks are not version commitments

DeepSeek Harness, OpenCode, Claude Code, Agy, and future runtimes are exploration tracks. Do not assign a production version merely because an experiment starts.

```text
exp/runtime
  ↓
SURFACE_DISCOVERED
  ↓
CAPABILITY_MAPPED
  ↓
FIXTURES_PASS
  ↓
REAL_OWNER_PROOF
  ↓
CONFORMANT
  ↓
PROMOTION_READY
  ↓
promote/runtime-* from current main
  ↓
production gates
  ↓
main
```

See `docs/process/EXPERIMENTAL_BRANCH_AND_PROMOTION_POLICY.md`.

## Explicit non-goals for v0.4

- production DeepSeek Harness adapter;
- production OpenCode adapter;
- production Claude Code adapter;
- production Agy adapter;
- broad Web UI/dashboard work;
- network broker or cloud aggregation;
- global mandatory daemon;
- remote telemetry;
- AI-generated root-cause diagnosis;
- weakening HookStat's privacy or truthful-coverage contracts.

## Completion definition

```text
PUBLIC_BASELINE=v0.3.1
PRODUCTION_RUNTIME=Codex

CODEX_HOOKS_HUMAN_INFORMATION_PARITY=PASS
LIVE_RUNTIME_HOOK_CATALOG=PASS
INSTALLED_UNOBSERVED_HOOKS_VISIBLE=true
HISTORICAL_NOT_INSTALLED_DISTINCT=true
RUNTIME_ISSUES_VISIBLE=true
UNKNOWN_RUNTIME_EVENTS_VISIBLE=true

RAW_UNIX_MILLISECONDS_IN_NORMAL_TUI=false
ZERO_SAMPLE_HEALTHY_PERCENT=false
METRIC_SCOPE_EXPLICIT=true
METRIC_SCOPE_CONSISTENCY=PASS
HUMAN_RISK_EXPLANATION=PASS
HUMAN_COVERAGE_EXPLANATION=PASS

RUNTIME_PRESENTATION_IN_MEMORY_ONLY=true
RAW_RUNTIME_PRESENTATION_PERSISTED=false

SAFE_WRITE_PARITY=PASS_OR_TRUTHFULLY_UPSTREAM_UNAVAILABLE
MANAGED_HOOK_MUTATION=false

WINDOWS=PASS
UBUNTU=PASS
OWNER_CODEX_HOOKS_AB_DOGFOOD=PASS
PACKAGE=PASS
PUBLICATION=OWNER_GATE
```
