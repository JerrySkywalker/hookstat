# HookStat Roadmap

## Active execution overlay — v0.4

HookStat v0.3.1 is publicly released from accepted main `651620cbc9f204f312fc31efee424c747895927a` with public tag/release `v0.3.1`.

Current product development is governed by [`ROADMAP_V040.md`](ROADMAP_V040.md) and the corresponding `goals/HS-G40*` through `goals/HS-G46R*` contracts.

The v0.4 product theme is **Hooks Control Center / Human Usability**.

Normative product principle:

> **Runtime Truth First, Reliability Second.**

For Codex hooks, HookStat must expose at least the human-readable current-runtime information provided by the pinned official Codex `/hooks` baseline, then add HookStat reliability/history/diagnosis as an additive layer.

v0.4 remains Codex production-first. DeepSeek Harness, OpenCode, Claude Code, Agy, and other runtime work proceeds through independent `exp/*` tracks and does not block v0.4 unless explicitly promoted.

Branch interaction is governed by [`../docs/process/EXPERIMENTAL_BRANCH_AND_PROMOTION_POLICY.md`](../docs/process/EXPERIMENTAL_BRANCH_AND_PROMOTION_POLICY.md):

```text
main      = accepted production truth
agent/*   = planned product work with merge intent
fix/*     = narrow production repair
exp/*     = exploration with no direct merge intent
promote/* = productization from current main
```

Experiments are promoted, not directly merged into `main`.

## Completed execution overlay — v0.3.1

The v0.3.1 Runtime-Neutral Native & IPC High-Performance Evidence Runtime train is complete and publicly released.

Its historical execution contract remains in [`ROADMAP_V031.md`](ROADMAP_V031.md). Historical receipts and goals remain immutable evidence; the `IN PROGRESS` wording inside that historical execution file is superseded by this top-level public-release record and should be closed during G40 housekeeping rather than rewritten as if the old document had always been final.

v0.3.1 established:

- runtime-neutral canonical evidence and authority routing;
- Native-first capability/admission framework;
- bounded local HSIP v1 broker/IPC;
- WAL/recovery and duplicate protection;
- reference producer and conformance kit;
- bounded diagnostics;
- single-repository HookStat release boundary;
- truthful `NOT_ADMITTED` domains;
- risk-aware Fast Lane, immutable candidate freeze, release orchestrator, and performance environment preflight.

An external cooperative producer is not bundled or required by HookStat v0.3.1.

## Historical product milestones

The sections below preserve original project-history context. Their version numbering is historical where later accepted roadmaps changed product sequencing.

### v0.1 — Codex historical reliability — COMPLETE

Critical path:

```text
HS-B00 Repository Foundation
  ↓
HS-G01 Codex Evidence Qualification
  ↓
HS-G02 Codex Vertical Slice
  ↓
HS-G03 Persistent Reliability Ledger
  ↓
HS-G04 Reliability Analytics
  ↓
HS-G05 TUI
  ↓
HS-G06 Real Codex Dogfood & Hardening
  ↓
HS-G07 Usable v0.1 Release Candidate
```

The v0.1 line established the first admitted Codex evidence path, stable handler attribution where proven, incremental/idempotent local ledger, reliability analytics, TUI, privacy boundaries, and public release.

### v0.2 / v0.2.1 — Reliability depth and startup — COMPLETE

The v0.2 line added Human reliability depth, revision comparison, failure clustering/fingerprints, diagnostics, richer Reliability Center behavior, bilingual Human UI, and later startup/period responsiveness with Today/24h/7d/30d/All semantics.

### v0.3.0 — Codex Reliability Workbench & Unified Human Interface — COMPLETE

The accepted post-v0.2 roadmap changed the original historical numbering. v0.3.0 remained Codex-only in production and delivered:

- TabBeacon-compatible shared Human interface contract;
- Changes workbench;
- Hook Catalog;
- revision timeline;
- safe Human aliases;
- bounded failure exploration;
- owner Windows Terminal dogfood;
- public v0.3.0 release.

Detailed history remains in `HOOKSTAT_POST_V02_TO_V03_ROADMAP.md`.

### v0.3.1 — Runtime-neutral evidence substrate — COMPLETE

See the completed overlay above.

## v0.4 — Hooks Control Center / Human Usability — ACTIVE

Critical path:

```text
PUBLIC v0.3.1
      ↓
HS-G40 Rebaseline & /hooks Parity Contract
      ↓
HS-G41 Live Runtime Hook Catalog
      ├───────────────┐
      ↓               ↓
HS-G42              HS-G43
Hooks Center        Human Reliability Presentation
      └───────┬───────┘
              ↓
HS-G44 Safe Hook Management
              ↓
HS-G45 Human UX / Codex /hooks A/B Dogfood
              ↓
HS-G46R v0.4 Hardening & Release
```

Read/information parity with Codex `/hooks` is mandatory. Write parity is admitted only when an official externally usable mutation surface is proven; otherwise it is truthfully `UPSTREAM_UNAVAILABLE` and does not authorize configuration guessing.

## Experimental runtime tracks — independent

The original early roadmap envisioned DeepSeek Harness as v0.3 and OpenCode as v0.4. That numbering is no longer normative. Runtime production versions are not preassigned before experiments prove viability.

Current conceptual tracks may include:

```text
exp/deepseek-hook-surface
exp/opencode-plugin-surface
exp/claude-hook-surface
exp/agy-hook-surface
```

Standard lifecycle:

```text
EXPERIMENT_STARTED
→ SURFACE_DISCOVERED
→ CAPABILITY_MAPPED
→ FIXTURES_PASS
→ REAL_OWNER_PROOF
→ CONFORMANT
→ PROMOTION_READY
→ promote/* from current main
→ production gates
→ main
```

Valid experiment outcomes also include `UPSTREAM_UNSUITABLE`, `DEFERRED`, and `ABANDONED`.

A future runtime release is planned only after an experiment reaches `PROMOTION_READY` and receives an explicit productization decision.

## Later product possibilities

Not part of v0.4 unless separately promoted:

- production DeepSeek Harness adapter;
- production OpenCode adapter;
- Claude Code adapter;
- Agy adapter;
- daemon/watch mode where justified;
- notifications;
- Web UI;
- OTel export;
- additional runtime adapters;
- cloud/distributed aggregation;
- AI-generated diagnosis.

The modular runtime-neutral core should allow those tracks without rewriting ledger, analytics, denominator semantics, or the Reliability Center application shell.
