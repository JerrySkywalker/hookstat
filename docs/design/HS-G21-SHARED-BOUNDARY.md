# HS-G21 — Shared Human Interface Boundary

## Decision

```text
SHARED_UI_BOUNDARY_DECIDED=true
COPY_PASTE_STATE_MACHINE_BASELINE=false
TABBEACON_REMAINS_REFERENCE=true
HOOKSTAT_DOMAIN_BOUNDARY_EXPLICIT=true
SHARED_UI_STRATEGY=dedicated dependency-neutral crate with product adapters
SHARED_UI_REPO=JerrySkywalker/jerry-terminal-ui
SHARED_UI_SHA=5bf1db60ba911c5ea7a01c7f7ef3924f730a0054
SHARED_UI_CRATES_IO_PUBLISHED=false
```

`jerry-terminal-ui` is a Rust 2024 / MSRV 1.97.1 MIT infrastructure crate.
It exposes no Ratatui, Crossterm, HookStat, or TabBeacon type. Both consumers
pin the exact Git revision above while the crate is unpublished.

## Shared ownership

The crate is the single implementation of:

- normal 2/21/2 shell geometry and the 24x10 admission rule;
- direct one-current-screen navigation;
- semantic footer state and two-space Human grammar;
- staged-settings edit/draft/discard state transitions;
- generic Help overlay state and dismiss keys;
- press-only input admission;
- Human chrome tokens and NO_COLOR policy;
- locale source precedence; and
- grapheme/display-cell-safe truncation and padding.

Ratatui/Crossterm rendering and terminal side effects remain adapter-owned so
TabBeacon can stay on Ratatui 0.29/Crossterm 0.28.1 while HookStat uses
Ratatui 0.30/Crossterm 0.29. Product catalogs and domain content remain
consumer-owned.

## Domain boundary

HookStat owns reliability periods, Hook rows/details, coverage/risk/trend
facts, diagnostics, and any reliability semantic color. TabBeacon owns its
management, workspace, session, integration, repair, and Hook-trust content.
Neither product owns a competing shell, footer, top-level navigation, Help,
input-admission, locale-precedence, or chrome implementation.

## Future publication ordering

The Git pin is for reproducible development only. Before any crates.io
consumer release: publish and verify `jerry-terminal-ui` first; update each
consumer from the Git revision to the released semver dependency; then run
that consumer's package/install gates before publishing it. This train creates
no tag, GitHub Release, or crates.io publication.
