# ADR 0004 — Terminal UI System Strategy

Status: Accepted historical v0.2 design foundation; superseded for new shared
Human-interface primitives by HS-G21 in v0.3. This record is retained so the
v0.2 decision history is not rewritten.

Repository note: `0004-opt-in-instrumented-evidence-fallback.md` already uses the numeric prefix `0004`. This run was explicitly contracted to create the present filename. The full ADR slug is therefore authoritative; the existing v0.1 ADR is not renamed or edited in this design-only train.

## Context

HookStat v0.2 is intended to be the second application adopting the Jerry Terminal UI Design System, with TabBeacon as the reference implementation.

Current repository evidence shows:

- both applications use Ratatui and Crossterm;
- HookStat uses `ratatui 0.30` / `crossterm 0.29`, while TabBeacon uses `ratatui 0.29.0` / `crossterm 0.28.1`;
- HookStat has one compact `tui.rs` with Home/Detail and synchronous manual refresh;
- TabBeacon has a large application-specific `control_center.rs` with a navigation shell, typed screens/drafts/commands, locale catalog, interface preference store, width helpers, and terminal guard;
- TabBeacon's Control Center refresh is deadline-driven but its collector still executes synchronously on the UI event-loop thread;
- TabBeacon's typed management, sessions, workspace, settings, and repair types are not generic UI types;
- there is not yet a standalone shared crate, versioning policy, cross-repository release process, or proven generic API consumed by two applications.

The shared contract must cover framework choice, layout, theme/color semantics, typography, navigation, shortcut footer, refresh/resource states, terminal lifecycle, components, and localization. Extracting too early would turn current application structure into a public API before HookStat proves which parts are actually common.

## Decision

Choose **Option B: internal module first, extract later**.

HookStat G00 will implement the contract in `docs/design/JERRY_TERMINAL_UI_SYSTEM.md` as an internal modular UI subsystem. It will align behavior and semantic interfaces with TabBeacon without copying TabBeacon domain models or changing TabBeacon in the HookStat train.

Conceptual boundary:

```text
HookStat domain/view models
        |
        v
HookStat internal UI system
  shell / navigation / theme / typography
  terminal guard / input commands
  resource states / refresh controller
  generic presentation components
  locale interface
        |
        v
Ratatui + Crossterm
```

The internal implementation must avoid HookStat-specific types in the pieces intended for later reuse. Application views and locale keys remain application-owned.

No `jerry-terminal-ui` crate, workspace split, Git dependency, repository subtree, or publication is introduced by this design train.

## Why

1. **The common behavior is clearer than the common code.** The shared layout, semantic palette, command model, terminal guard, refresh states, and locale rules can be frozen now. A stable Rust API cannot yet be derived from two consumers.
2. **Dependency versions differ.** Immediate extraction would force a framework upgrade/downgrade before a risk-specific compatibility test identifies the right version pair.
3. **TabBeacon is application-specific.** Its state includes management snapshots, mutable drafts, workspace aliases, sessions, repairs, and terminal presentation settings. Moving that file into a crate would export the wrong boundary.
4. **The requested async target is not current shared code.** HookStat needs an off-thread, generation-tagged refresh model, while TabBeacon currently performs bounded synchronous collection. The generic contract should be proven before extraction.
5. **Release/governance cost is real.** A shared external crate introduces independent versioning, compatibility, CI, publication, and coordinated upgrade responsibilities. No current user value requires that cost in G00.
6. **HookStat remains a modular monolith.** ADR 0001 says a workspace split requires real reuse pressure; an internal module satisfies the current architecture.

## Extraction criteria

Reconsider an external `jerry-terminal-ui` crate only after all of the following are true:

```text
CONFORMING_APPLICATIONS>=2
GENERIC_COMPONENTS_PROVEN=true
APPLICATION_DOMAIN_TYPES_IN_SHARED_API=false
RATATUI_CROSSTERM_VERSION_POLICY_DECIDED=true
ASYNC_REFRESH_CONTRACT_PROVEN_IN_BOTH=true
TERMINAL_GUARD_PROVEN_IN_BOTH=true
LOCALE_INTERFACE_PROVEN_IN_BOTH=true
RELEASE_OWNERSHIP_DEFINED=true
BREAKING_CHANGE_POLICY_DEFINED=true
```

Candidate extraction scope:

- semantic theme/typography tokens;
- display-width/grapheme helpers;
- application shell geometry;
- navigation and shortcut components;
- typed key commands;
- loading/empty/error resource presentation;
- generation/coalescing refresh controller interfaces;
- terminal lifecycle guard;
- locale trait/key lookup interface, not application message catalogs.

Excluded from a shared crate:

- HookStat domain, ledger, receipt, analytics, identity, trust, or Codex types;
- TabBeacon management, workspace, sessions, provider, repair, or terminal-title types;
- application navigation enums and message keys;
- application-owned preference schemas;
- mutation authority.

## Alternatives considered

### Option A — Create `jerry-terminal-ui` now

Rejected for G00. It creates a public/release boundary before the second consumer exists, couples mismatched framework versions, and risks exporting TabBeacon application concepts. It also cannot truthfully offer a proven shared asynchronous refresh implementation today.

### Copy TabBeacon `control_center.rs` into HookStat

Rejected. The file is intentionally built around TabBeacon management/domain state. Copying it would produce a HookStat-only fork that looks shared but drifts immediately.

### Keep the v0.1 HookStat TUI and only restyle colors

Rejected. It would not establish the shared layout, typed state, navigation, localization, terminal lifecycle, or non-blocking refresh architecture required by the roadmap.

### Use TabBeacon as a Git/path dependency

Rejected. TabBeacon is an application crate, not a generic UI library. Cross-repository source coupling would blur product and release authority.

### Add Tokio for the refresh model

Rejected as a default. A bounded worker thread/channel can meet the contract without a large dependency or daemon. A future goal may justify an executor only with measured need.

## Consequences

Positive:

- HookStat can start G00 without waiting for cross-repository release infrastructure.
- Shared semantics are frozen before code API compatibility.
- v0.1 runtime/instrumentation/ledger boundaries remain isolated.
- The off-thread refresh model can be proven in a smaller read-only application first.
- Later extraction is based on two implementations and concrete duplication.

Costs:

- Some carefully bounded implementation duplication will exist temporarily.
- TabBeacon will not automatically receive HookStat's async refresh/controller improvements.
- Extraction later will require coordinated dependency and migration work.
- Reviewers must enforce the contract so an internal module does not become a new HookStat-only visual language.

## Implementation constraints

- G00 is infrastructure, not the complete v0.2 UI rewrite.
- No large dependency is added without evidence.
- Render/key handling performs no database work.
- No runtime/Codex mutation authority enters the generic UI boundary.
- Locale catalogs and view models remain application-owned.
- Public crate publication requires a separate explicit decision and release authorization.
- TabBeacon remains read-only during HookStat implementation unless a separate TabBeacon goal authorizes its changes.

## Decision receipt

```text
SELECTED_OPTION=B_INTERNAL_MODULE_FIRST
EXTERNAL_CRATE_CREATED=false
TABBEACON_MODIFIED=false
HOOKSTAT_RUNTIME_BEHAVIOR_CHANGED=false
EXTRACTION_REVISIT=AFTER_TWO_CONFORMING_CONSUMERS
```
