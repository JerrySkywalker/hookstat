//! G44 locks the truthful read-only outcome when Codex exposes no bounded
//! external enable/disable route.

#[test]
fn unavailable_management_has_no_unsupported_tui_write_route() {
    let app = include_str!("../src/tui/app.rs");
    let keymap = include_str!("../src/tui/keymap.rs");
    let rendering = include_str!("../src/tui/rendering.rs");

    for forbidden in [
        "SetHookEnabled",
        "ConfigBatchWrite",
        "hooks.state",
        "trusted_hash",
    ] {
        assert!(
            !app.contains(forbidden) && !keymap.contains(forbidden),
            "unavailable G44 management must not expose {forbidden} through the TUI"
        );
    }
    assert!(rendering.contains("StateHookManagementUnavailable"));
}
