# HS-G22 / HS-G23 — Unified Human Interface Convergence

## Scope and result

```text
GOALS=HS-G22,HS-G23
TABBEACON_BASELINE=2eb39c0a6af363fd4e680ad968ec17e3ffb05f7d
SHARED_UI_REPO=JerrySkywalker/jerry-terminal-ui
SHARED_UI_SHA=5bf1db60ba911c5ea7a01c7f7ef3924f730a0054
```

HookStat retains its reliability model, period semantics, asynchronous refresh,
diagnostic independence, and read-only evidence behavior. The Human shell and
interaction layer now adapt the shared semantic implementation.

## G22 interaction convergence

| Contract | Implementation and deterministic proof |
| --- | --- |
| Direct top-level navigation | `NavigationState` wraps `TopLevelNavigation`; app tests prove Overview → Hooks → Overview directly. |
| No selected/active split | The route adapter has one `current` route only; sidebar renders one `>` marker. |
| Explicit local interaction | Enter on Hooks starts `HooksList`; Esc returns to page navigation. Hook detail is a separate local state. |
| Contextual footer | `FooterState` / `format_footer` replace the ` · ` chain with two-space sentence grammar. |
| Settings edit | Shared `SettingsEditor` owns Enter edit/done, Up/Down field movement, Left/Right draft mutation, Apply, Revert, and dirty quit confirmation. |
| Help | Shared Help overlay state owns input until Esc, `?`, or `q`; renderer provides en-US and zh-CN Help content. |
| Press-only input | Physical key mapping rejects Repeat and Release; the shared input primitive carries the same policy. |

## G23 shell convergence

| Contract | Implementation and deterministic proof |
| --- | --- |
| Header/body/footer | `HUMAN_SHELL` supplies 2 / remaining / 2 rows. |
| Sidebar/minimum | `HUMAN_SHELL` supplies 21 columns and 24x10. The previous HookStat-only 21/12/4 policy is removed. |
| Header grammar | `HookStat Reliability Center — <overall Human status>` uses the shared title/chrome roles. |
| Sidebar grammar | `Sections` / `分区` and one `>` current marker; no dark top-level selection background. |
| Chrome/typography | Shared `ChromeToken`, cyan/default policy, and NO_COLOR behavior are adapted to Ratatui 0.30. Reliability status remains a documented content exception with text/glyph meaning. |
| Locale/width | Shared locale precedence and width helpers are used. HookStat obtains an operating-system locale through `sys-locale`, including Windows, instead of assuming `LANG`. |

## Deterministic parity evidence

- shared crate unit tests: navigation, footer states, Settings/discard,
  Help dismissal, press-only input, geometry, locale precedence, NO_COLOR, and
  CJK grapheme/display width;
- TabBeacon Control Center/Human-presentation adapter tests;
- HookStat application and TestBackend renders for normal, narrow, minimum,
  en-US, zh-CN, Help, direct navigation, dirty-discard, and no selected/active
  split.

## Remaining gate

```text
OWNER_TABBEACON_HOOKSTAT_AB_VISUAL_SMOKE_REQUIRED=true
```

An Owner-attended Windows Terminal A/B comparison is the only evidence that
cannot be truthfully substituted by unattended deterministic tests. It remains
outside this train's unattended mutation authority.
