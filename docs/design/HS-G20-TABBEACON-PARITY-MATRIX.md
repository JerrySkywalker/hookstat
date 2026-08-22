# HS-G20 — TabBeacon UI/UX Parity Matrix

## Evidence identity

```text
GOAL_ID=HS-G20
TABBEACON_BASELINE=2eb39c0a6af363fd4e680ad968ec17e3ffb05f7d
HOOKSTAT_START_MAIN=edba3f5d208b6fc52e644a4f530bd825d9c0e12b
TABBEACON_BASELINE_PINNED=true
```

The TabBeacon baseline was inspected directly at the pinned commit.  The
reference sources are `src/control_center.rs`, `src/human_presentation.rs`,
and `src/interface_preferences.rs`, including their in-file deterministic
tests and terminal-smoke fixture.  HookStat sources were inspected at the
admitted start main, including `src/tui/{app,keymap,layout,rendering,terminal,
theme,widgets}.rs`, localization, and their deterministic tests.

Classification is deliberately descriptive, not a waiver.  `DRIFT` rows are
the G22/G23 implementation checklist. `DOMAIN_EXCEPTION` is allowed only for
reliability content, never for shared chrome or controls.

| Required G20 row | TabBeacon pinned behavior | HookStat start behavior | Classification | G22/G23 disposition |
| --- | --- | --- | --- | --- |
| header geometry | two rows | three rows | DRIFT | shared 2-row shell |
| header grammar | emphasized title, `Title — overall state` | title only in distinct chrome | DRIFT | shared header model |
| sidebar width | 21 columns in normal shell | 21/12/4 independent policy | DRIFT | shared shell policy |
| sidebar title | `Sections` / `分区` | `Navigation` / localized equivalent | DRIFT | shared wording/token |
| screen marker grammar | one `> current` marker | selected `>` plus active `•` | DRIFT | one current route |
| page switching | Up/Down/j/k change screen immediately | navigation focus then activation | DRIFT | direct navigation |
| selected vs active state | one `screen` state | independently stored selected and active routes | DRIFT | remove divergence |
| navigation/content focus | no global focus | global Navigation/Content focus | DRIFT | remove global focus |
| settings edit entry | Enter enters/finishes explicit edit state | no comparable staged edit-state admission | DRIFT | shared draft interaction |
| settings field navigation | Up/Down while editing | arrows have unrelated/global handling | DRIFT | edit-scoped field navigation |
| settings value change | Left/Right while editing | left/right can change without the reference edit model | DRIFT | edit-scoped draft change |
| apply key | `a` requests staged Apply | `a` applies from Settings through a reused period command | MATCH | bind explicitly to shared semantic action |
| revert key | `r` reverts staged draft | `x` reverts Settings; `r` refreshes | DRIFT | edit-scoped `r` Revert |
| dirty quit confirmation | `q` opens explicit discard confirmation | quit is not dirty-draft guarded | DRIFT | shared discard state |
| help overlay | `?` opens a keyboard-owning Help overlay | absent | DRIFT | shared overlay state |
| footer navigation text | contextual sentence grammar | complete shortcut inventory | DRIFT | normal-navigation footer state |
| footer editing text | edit-state-specific grammar | no explicit edit state | DRIFT | settings-edit footer state |
| footer conflict/discard text | explicit conflict/discard states | no discard state and no equivalent grammar | DRIFT | warning/discard footer states |
| footer spacing grammar | actions separated by two spaces | generated ` · ` chain | DRIFT | shared formatter |
| color/chrome policy | cyan Human chrome or default NO_COLOR chrome | independent palette and selected background | DRIFT | shared chrome tokens |
| typography | bold title; ordinary shared chrome | independent bold/dim/color hierarchy | DRIFT | shared typography tokens |
| locale source precedence | CLI, environment, preference, OS, fallback | same precedence is represented in `LanguageState` | MATCH | move resolution primitive to shared core |
| Windows OS locale lookup | operating-system locale is queried on Windows | locale plumbing has no proved Windows user-locale query | DRIFT | consumer Windows locale adapter |
| minimum size | 24x10 | 24x10 | MATCH | shared geometry constant |
| resize behavior | one 21-column normal shell with bounded fallback | HookStat-only 21/12/4 transition | DRIFT | shared narrow policy |
| key repeat/release filtering | Press only | Press only | MATCH | shared input admission |
| terminal cleanup | RAII restores alternate screen/raw mode/cursor | RAII restores alternate screen/raw mode/cursor | MATCH | shared terminal lifecycle contract |
| CJK display width/truncation | grapheme/display-cell safe fit and pad | grapheme/display-cell safe truncation | MATCH | shared display-width primitives |

## Domain exceptions

| Surface | Classification | Justification |
| --- | --- | --- |
| reliability status glyphs and content colors | DOMAIN_EXCEPTION | They communicate HookStat evidence/coverage states and remain text/glyph-complete; they do not control chrome, navigation, or selection. |
| Today/24h/7d/30d/All selector and statistics | DOMAIN_EXCEPTION | They are HookStat-owned data semantics. Their chrome, marker, footer, and help language must still use the shared Human interface. |
| TabBeacon-only management, workspace, session, integration, and Hook-trust pages | NOT_APPLICABLE | They are product-specific TabBeacon content, not a HookStat shell requirement. |

## G20 decision

```text
PARITY_MATRIX_COMPLETE=true
UNJUSTIFIED_DRIFT=0
DOMAIN_EXCEPTIONS_DOCUMENTED=true
G20_ACCEPTED=true
```

The declared drifts are not accepted end-state differences. They are all
assigned to G22/G23.  A shared implementation boundary is mandatory because
the start state contains two independent implementations of exactly the state
machines that the parity contract marks as shared.
