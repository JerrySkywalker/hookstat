# G45 automated fixture matrix

This matrix prepares the Owner-only visual dogfood. Every listed fixture is
sanitized and deterministic; no Owner hook configuration, command, matcher,
source path, prompt, tool payload, or screenshot is used as repository
evidence.

```text
GOAL=G45
PRODUCT_ACCEPTED_MAIN=cf5be21c134fbfa83b9c6a0adf5cc68800d8e4dd
CODEX_BASELINE=rust-v0.151.0 / 78c290807ce710180111df227df3b7a4fe845452
WIDE=140x58
NARROW=44x44
LOCALES=en-US,zh-CN
SAFE_WRITE_UX=UPSTREAM_UNAVAILABLE
RAW_PRESENTATION_PERSISTENCE=0
G45_OWNER_VISUAL_CHECK=PENDING
```

## Sanitized catalog and state coverage

| Required state | Sanitized fixture / deterministic test |
| --- | --- |
| Event, Handler, Detail routes; wide and narrow; en-US and zh-CN | `tui::rendering::tests::hooks_control_center_renders_runtime_truth_before_reliability_in_both_locales` |
| Long command, matcher, and source | `control_center_catalog` inside the same rendering test module |
| Command, MCP Tool, Prompt, and Agent | `control_center_catalog` entries `fixture:0:0` through `fixture:0:3` |
| Managed, needs-review/trust state, disabled, installed-unobserved | `hooks_control_center_renders_joined_health_errors_unknown_types_and_selected_rows` |
| Partial coverage, zero terminal samples, explicit metric scope and risk reason | `hooks_control_center_renders_joined_health_errors_unknown_types_and_selected_rows` |
| Interrupt and future unknown event; runtime warnings/errors | `hooks_control_center_renders_runtime_truth_before_reliability_in_both_locales` |
| Historical-only versus current detail ownership | `historical_detail_rendering_ignores_stale_runtime_selection` |
| Unavailable write path and no misleading control | `runtime_detail_truthfully_explains_unavailable_hook_management_in_both_locales` and `g44_safe_hook_management` |
| No raw epoch fallback; localized human time | `relative_age_boundaries_are_localized_without_epoch_fallback` and `narrow_tall_change_detail_scrolls_into_human_timeline` |

## Deterministic interaction coverage

| Interaction | Test evidence |
| --- | --- |
| Events → Handlers → Detail | `hooks_control_center_navigates_events_handlers_and_detail_without_mutating_history` |
| Esc/backtracking and local selection stability | `historical_detail_rendering_ignores_stale_runtime_selection`; `failure_cluster_navigation_cancels_an_open_historical_alias_draft` |
| Non-destructive runtime refresh | `runtime_catalog_refresh_is_explicit_and_selection_survives_non_destructive_update` |
| Loading and error state preserves accepted content | `runtime_and_reliability_failures_do_not_erase_the_other_accepted_resource`; `loading_shell_draws_before_data_and_marks_pending_today_immediately` |
| Bilingual footer/help and narrow reflow | `catalog_alias_and_failure_cluster_surfaces_are_bilingual_and_narrow_safe`; `every_top_level_view_reflows_at_normal_narrow_and_minimum_sizes` |

## Automated commands

```text
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets --locked
cargo build --locked
git diff --check
```

The Owner packet is intentionally the next acceptance boundary. Automated
coverage cannot answer the five perception/usability questions or substitute
for an owned Windows Terminal A/B session.
