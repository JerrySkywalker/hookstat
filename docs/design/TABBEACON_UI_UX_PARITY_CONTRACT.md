# TabBeacon UI/UX Parity Contract for HookStat

## Status

NORMATIVE for HookStat v0.3 planning and implementation.

## Purpose

HookStat v0.2 intentionally adopted TabBeacon design ideas but retained an independent TUI state machine. Owned dogfood showed that this produced real interaction drift: navigation required a different mental model, the sidebar showed different selected/active semantics, Settings used different keys, the footer used a different grammar, and the shell/chrome style diverged.

This document replaces the vague phrase `TabBeacon-aligned` with an enforceable rule:

> **TabBeacon-compatible by contract and shared implementation.**

TabBeacon is the normative Human-interface reference. HookStat owns reliability-domain content, not a separate application-shell UX language.

## Reference baseline

The initial parity baseline was audited against the accepted TabBeacon Control Center implementation available during HookStat post-v0.2 planning, including:

- `JerrySkywalker/tabbeacon/src/control_center.rs`;
- `JerrySkywalker/tabbeacon/src/human_presentation.rs`.

The observed source baseline at planning time includes TabBeacon commit `8dfa7f35978ced067f2b13dffbfd375d933c5bda` in search/index references. G20 MUST resolve and record the exact accepted TabBeacon baseline actually used for implementation before parity work begins.

A future TabBeacon change does not silently redefine an in-flight HookStat release. Moving the parity baseline requires an explicit decision and updated parity evidence.

# 1. Scope of exact parity

The following are shared Human-interface behavior and MUST conform unless a documented domain exception is approved:

- application shell geometry;
- header grammar;
- sidebar/navigation grammar;
- top-level screen switching behavior;
- selected/current marker behavior;
- Settings edit-state behavior;
- Apply/Revert/dirty-quit behavior;
- footer grammar and state replacement;
- help-overlay ownership and dismissal;
- press-only key-event policy;
- Human chrome color policy;
- Human typography hierarchy;
- locale source precedence;
- OS-locale behavior;
- minimum terminal size;
- resize handling;
- terminal enter/restore behavior;
- display-cell-safe CJK/truncation primitives.

The following remain HookStat domain-owned and may differ:

- reliability periods;
- Hook list content;
- Hook Detail content;
- risk/trend/revision semantics;
- Changes workbench;
- diagnostics facts;
- alias operations;
- failure-cluster content.

Domain ownership does not authorize a different shell or control grammar.

# 2. Top-level navigation contract

## Reference behavior

TabBeacon uses one current `screen`. In normal non-editing mode:

```text
Up / Down / j / k
        ↓
switch current top-level screen immediately
```

There is no required global Navigation-vs-Content focus step before switching pages.

## HookStat v0.3 requirement

HookStat MUST converge to the same top-level mental model.

Forbidden baseline after convergence:

```text
Tab
  ↓
Navigation focus
  ↓
Up/Down moves selected route
  ↓
Enter activates route
```

Top-level state MUST NOT require both `selected_route` and `active_route`.

### Required

```text
TOP_LEVEL_GLOBAL_FOCUS_MODEL=false
TOP_LEVEL_SELECTED_ACTIVE_DIVERGENCE=false
DIRECT_PAGE_NAVIGATION=true
```

Local interaction modes are allowed. For example, a Hook list may have a selected row, and Hook Detail may scroll. Those are local content states, not a second global page-navigation system.

# 3. Sidebar/current-screen grammar

## Reference

TabBeacon current-screen grammar is conceptually:

```text
Sections
> Overview
  Appearance
  Workspace
  ...
```

Only the current top-level screen receives the `>` marker.

## HookStat requirement

Use the same single-marker model:

```text
Sections / 分区
> Overview / 概览
  Hooks
  Changes
  Diagnostics / 诊断
  Interface / 界面
```

Do not retain the v0.2 combination:

```text
> selected
• active
```

Do not use a separate dark selected background to communicate a navigation state that TabBeacon does not have.

# 4. Shell geometry

Normal-terminal reference geometry:

```text
header  = 2 rows
body    = remaining rows
footer  = 2 rows
sidebar = 21 columns
content = remaining columns
```

Minimum terminal contract:

```text
MIN_WIDTH=24
MIN_HEIGHT=10
```

HookStat MUST use the same normal geometry after convergence.

A shared narrow-mode implementation may adapt sidebar/content geometry, but HookStat MUST NOT maintain independent 21/12/4 or other breakpoints once a shared implementation exists unless the shared contract itself defines them.

# 5. Header grammar

TabBeacon header combines product Human title and overall Human state, with the title emphasized.

Shared grammar:

```text
<Product Human Title> — <overall Human state>
```

HookStat examples:

```text
HookStat Reliability Center — Healthy
HookStat Reliability Center — Coverage limited
HookStat 可靠性中心 — 健康
HookStat 可靠性中心 — 覆盖受限
```

The precise reliability state is HookStat-owned, but punctuation, spacing, emphasis, and chrome structure follow the shared header renderer.

# 6. Footer grammar and state machine

## Reference grammar

TabBeacon uses Human sentence-style shortcut hints:

```text
<Key> action  <Key> action  <Key> action
```

Groups are separated by spacing, not by a generated bullet/dot token chain.

Examples from the reference interaction language:

```text
↑↓ navigate  Enter edit selected screen  a Apply  r Revert  q Quit
↑↓ select setting  ←→ change draft  Enter done  a Apply  r Revert
```

Localized Chinese follows the same grammar:

```text
↑↓ 导航  Enter 编辑当前分区  a 应用  r 还原  q 退出
↑↓ 选择设置  ←→ 调整草稿  Enter 完成  a 应用  r 还原
```

## HookStat requirement

Retire the v0.2 footer approach that dynamically concatenates every possible command with ` · ` separators.

Footer MUST be a context state machine. At minimum define:

1. normal top-level navigation;
2. Hook list interaction;
3. Hook Detail interaction;
4. period interaction where needed;
5. Settings editing;
6. dirty/unsaved state;
7. discard confirmation;
8. overlay dismissal;
9. conflict/warning state where applicable.

The footer should show only the most important current actions. Complete shortcut documentation belongs in the Help overlay.

Required:

```text
FOOTER_GRAMMAR_PARITY=true
FOOTER_DOT_TOKEN_CHAIN=false
FOOTER_CONTEXT_STATE_MACHINE=true
```

# 7. Settings edit model

Reference interaction:

```text
Enter       enter editing / finish editing
Up/Down     select field while editing
Left/Right  change draft value
a           Apply
r           Revert
q           Quit; if dirty, ask before discarding
```

HookStat v0.2 behavior differs and MUST converge in v0.3.

Contextual key reuse is allowed:

- on reliability pages, `r` may mean Refresh;
- in Settings editing, `r` means Revert.

The footer and Help overlay must always make the current meaning explicit.

Dirty Settings MUST NOT disappear silently on quit.

Required:

```text
SETTINGS_EDIT_MODEL_PARITY=true
DIRTY_QUIT_CONFIRMATION=true
```

# 8. Help overlay

TabBeacon uses `?` for a focused Help overlay.

HookStat MUST provide the same generic overlay behavior:

```text
?        open Help
Esc/?/q  dismiss Help
```

While Help is open:

- overlay owns normal key events;
- underlying screen state does not accidentally change;
- footer becomes overlay-dismiss guidance;
- active locale is respected;
- terminal resize remains safe.

HookStat Help content should cover:

- top-level navigation;
- Today/24h/7d/30d/All selection;
- Hooks row selection;
- search/filter/sort;
- Hook Detail navigation;
- Changes workbench;
- Settings editing/apply/revert;
- reliability truthfulness concepts where concise.

# 9. Input event policy

Both applications already use a press-only policy. This is normative:

```text
KeyEventKind::Press   admitted
KeyEventKind::Repeat  ignored
KeyEventKind::Release ignored
```

Do not replace this with timing-based debounce merely because a platform emits repeat events.

If a future accessibility requirement needs held-key repeat, it must be added through a shared explicit policy rather than HookStat-only behavior.

# 10. Human chrome color policy

TabBeacon Human chrome is the reference.

At the planning baseline, enabled Human color uses a cyan-oriented application chrome; disabled/NO_COLOR falls back to terminal defaults.

HookStat MUST NOT preserve a separate application-wide chrome palette for header/sidebar/footer/control selection merely because v0.2 introduced one.

Shared chrome includes:

- header;
- sidebar;
- border treatment;
- footer;
- normal settings controls;
- overlay chrome.

Reliability-domain content may use additional semantic text/glyphs. Additional domain color is allowed only as a documented content-level exception and MUST NOT:

- redefine shared navigation/selection color language;
- become the only carrier of meaning;
- create a visibly separate application family.

NO_COLOR and explicit no-color settings must behave consistently across TabBeacon and HookStat.

# 11. Typography

Shared chrome typography follows TabBeacon's Human hierarchy.

At minimum:

- Human application title emphasized/bold;
- shared sidebar/footer normal Human style;
- no independent HookStat-only dim/bold/color hierarchy for shared chrome.

Reliability-domain tables may use restrained content formatting after chrome parity is satisfied.

# 12. Locale and Human wording policy

Both applications support:

```text
en-US
zh-CN
```

Shared locale precedence:

```text
explicit CLI
  ↓
product environment variable
  ↓
persisted Human-interface preference
  ↓
operating-system locale
  ↓
en-US fallback
```

Windows operating-system locale MUST be resolved from Windows user-locale APIs or the shared accepted mechanism, not by assuming Unix `LANG` is present.

Shared wording should use the same concepts where the actions are the same:

```text
Sections / 分区
Help / 帮助
Apply / 应用
Revert / 还原
Quit / 退出
```

HookStat domain terms remain HookStat-owned.

# 13. Terminal lifecycle and resize

Shared terminal primitives must preserve:

- alternate-screen entry/exit correctness;
- raw-mode restoration;
- cursor restoration where applicable;
- clean quit;
- Ctrl+C policy consistent with dirty-state safety;
- resize without panic/corruption;
- minimum-size fallback at 24x10;
- display-cell-aware truncation and CJK handling.

HookStat MUST NOT regress terminal restoration while adopting asynchronous startup or new workbench pages.

# 14. Period selector as a domain-specific control

The v0.2.1 period selector is a HookStat domain control:

```text
Today | 24h | 7d | 30d | All
```

Its data semantics are not a TabBeacon concern.

However, its Human interaction MUST use shared control conventions:

- same chrome style;
- same selection marker grammar;
- same footer/help wording style;
- no reintroduction of a global Navigation/Content focus model;
- immediate visual feedback while background data refresh is pending.

# 15. Shared implementation boundary

Real dogfood has proved that two independently maintained TUI state machines drift.

v0.3 therefore MUST select an enforceable reuse strategy.

Preferred conceptual package:

```text
terminal-ui-contract
  ├ shell/layout
  ├ header/sidebar/footer chrome
  ├ top-level navigation
  ├ footer state grammar
  ├ generic Help overlay
  ├ edit-mode primitives
  ├ press-only key policy
  ├ terminal guard
  ├ display-width/CJK helpers
  └ Human color/typography primitives
```

The exact repository/crate packaging is decided in HS-G21.

Unacceptable final state:

> TabBeacon and HookStat each retain independent shell/navigation/footer/edit state machines and rely only on documentation to remain synchronized.

# 16. Parity verification

v0.3 acceptance requires three proof classes.

## Source/contract proof

Automated tests assert the interaction contract independently of screenshots.

Examples:

```text
Down from Overview -> next screen immediately
no top-level selected/active split
Settings Enter -> editing
Settings r -> revert
Dirty q -> confirm discard
? -> Help overlay
Repeat/Release -> ignored
```

## Deterministic render proof

Ratatui/TestBackend fixtures cover:

- normal en-US;
- normal zh-CN;
- narrow/minimum terminal;
- Help overlay;
- dirty Settings/discard state;
- loading/error/empty;
- Changes page;
- no-color.

## Owned Windows Terminal A/B dogfood

Open current accepted TabBeacon and HookStat in the same Windows Terminal environment and verify the shared Human interface directly.

Acceptance questions:

```text
Does Up/Down switch pages the same way?
Does the sidebar mark the current page the same way?
Does the header occupy the same geometry and grammar?
Does the footer use the same spacing and state model?
Does Settings edit/apply/revert/quit feel identical?
Does ? Help behave identically?
Does no-color behave consistently?
Does resize/quit restore the terminal consistently?
```

Domain content does not need to be identical.

# 17. Parity completion contract

```text
TABBEACON_UI_REFERENCE=normative
TABBEACON_BASELINE_PINNED=true
SHARED_UI_BOUNDARY=PASS
TOP_LEVEL_GLOBAL_FOCUS_MODEL=false
TOP_LEVEL_SELECTED_ACTIVE_DIVERGENCE=false
DIRECT_PAGE_NAVIGATION=true
CURRENT_SCREEN_MARKER_PARITY=true
HEADER_GEOMETRY_PARITY=true
SIDEBAR_GEOMETRY_PARITY=true
FOOTER_GEOMETRY_PARITY=true
FOOTER_GRAMMAR_PARITY=true
SETTINGS_EDIT_MODEL_PARITY=true
DIRTY_QUIT_CONFIRMATION=true
HELP_OVERLAY_PARITY=true
PRESS_ONLY_INPUT_PARITY=true
HUMAN_CHROME_STYLE_PARITY=true
TYPOGRAPHY_PARITY=true
LOCALE_POLICY_PARITY=true
WINDOWS_OS_LOCALE_PARITY=true
MINIMUM_SIZE_PARITY=true
TERMINAL_RESTORATION_PARITY=true
OWNER_WINDOWS_TERMINAL_A_B_DOGFOOD=PASS
```

A passing v0.3 may contain different reliability-domain pages and controls, but opening and operating HookStat and TabBeacon must no longer feel like two separately designed terminal applications.
