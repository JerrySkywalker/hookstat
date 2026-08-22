# HS-G26 Automated Human Interface Hardening Matrix

This matrix records deterministic evidence for the v0.3 Human-interface
surface. It is intentionally separate from the Owner-attended terminal A/B
comparison: an automated terminal buffer can prove semantic parity and safe
fallbacks, but cannot attest that TabBeacon and HookStat feel identical in an
owned Windows Terminal session.

```text
TABBEACON_BASELINE=2eb39c0a6af363fd4e680ad968ec17e3ffb05f7d
SHARED_UI_REVISION=5bf1db60ba911c5ea7a01c7f7ef3924f730a0054
OWNER_AB_VISUAL_SMOKE=OWNER_ACTION_REQUIRED
```

| Verification family | Deterministic evidence | Required property |
| --- | --- | --- |
| Shared source contract | `tui::conformance::tests::source_contract_parity_uses_the_shared_shell_navigation_footer_and_editor` | shared 2/21/2 shell, one-current navigation, footer grammar, shared dirty-discard transitions |
| Key admission and overlay | `tui::conformance::tests::source_contract_admits_only_press_events_and_help_owns_input`; `tui::app::tests::help_overlay_owns_normal_keys_until_dismissed` | repeat/release rejected; Help consumes normal input and dismisses safely |
| No-color accessibility | `tui::conformance::tests::no_color_preserves_text_and_selection_semantics_without_terminal_colors`; `tui::theme::tests::monochrome_preserves_selection_without_color` | no ANSI color dependence; selection and emphasis retain text/glyph/modifier semantics |
| CJK and narrow rendering | `tui::widgets::tests::width_primitives_preserve_combining_and_cjk_graphemes`; `tui::rendering::tests::catalog_alias_and_failure_cluster_surfaces_are_bilingual_and_narrow_safe` | grapheme-safe width, en-US/zh-CN, narrow Catalog/failure pages |
| Minimum and resize geometry | `tui::conformance::tests::minimum_size_has_no_partial_shared_shell`; `tui::layout::*`; `tui::rendering::tests::every_top_level_view_reflows_at_normal_narrow_and_minimum_sizes` | 24x10 minimum, no partial widgets below minimum, deterministic resize layout |
| Loading, empty, and error | `tui::rendering::tests::loading_shell_draws_before_data_and_marks_pending_today_immediately`; `tui::rendering::tests::representative_viewports_cover_empty_populated_and_degraded_states`; `tui::state::*` | first frame is usable; empty/error does not fabricate a healthy claim |
| Changes workbench | `workbench::*`; `tui::rendering::tests::changes_render_populated_narrow_and_drill_down_in_both_locales` | history/coverage/revision boundaries, period reclassification, safe narrow drill-down |
| Catalog, aliases, exploration | `ledger::tests::alias_apply_is_conflict_safe_and_rejects_unsafe_text`; `tui::app::tests::alias_edit_is_draft_only_until_conflict_safe_apply_result`; `tui::app::tests::dirty_alias_quit_uses_the_shared_discard_confirmation`; `tui::view_model::tests::failure_clusters_aggregate_safe_taxonomy_across_affected_hooks` | HookStat-only aliases, explicit Apply/Revert/Cancel, no raw error stream, affected hook evidence |
| Scale and periods | `workbench::tests::projects_ten_thousand_invocations_many_hooks_and_revision_epochs_deterministically`; `ledger::tests::finite_reliability_queries_materialize_only_the_recent_bounded_range`; `tui::app::tests::latest_requested_period_rejects_out_of_order_snapshot_completion` | >=10,000 deterministic history, many hooks/revisions, bounded ordinary reads, latest-request-wins |
| Diagnostics and terminal cleanup | `tui::app::tests::period_switches_only_request_reliability_and_leave_diagnostics_independent`; `tui::terminal::*` | diagnostics stays independent; terminal modes restore after complete or partial entry |

The exact-head Windows and Linux CI workflow runs formatting, Clippy with
warnings denied, tests, and build for every candidate. The remaining visual
gate must be executed by the Owner without claiming success from deterministic
fixtures alone.
