//! Deterministic, dependency-neutral conformance checks for the shared Human
//! interface. Product rendering tests retain their HookStat content fixtures;
//! these tests make the common shell and interaction contract explicit.

#[cfg(test)]
mod tests {
    use super::super::{
        app::{App, Screen},
        keymap::{Command, command_for},
        layout::{ApplicationShell, ShellLayout},
        navigation::{NavigationState, Route},
        theme::{ColorRole, Theme, TypographyRole},
    };
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
    use ratatui::{layout::Rect, style::Modifier};
    use terminal_ui_contract::{
        footer::{FooterAction, FooterState, format_footer},
        interaction::{DiscardDecision, QuitDisposition, SettingsEditor},
        layout::HUMAN_SHELL,
    };

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum FixtureField {
        One,
        Two,
    }

    fn action_name(action: FooterAction) -> &'static str {
        match action {
            FooterAction::Navigate => "navigate",
            FooterAction::Open => "open",
            FooterAction::Help => "help",
            FooterAction::Refresh => "refresh",
            FooterAction::Quit => "quit",
            FooterAction::Select => "select",
            FooterAction::Back => "back",
            FooterAction::Page => "page",
            FooterAction::Search => "search",
            FooterAction::Filter => "filter",
            FooterAction::Sort => "sort",
            FooterAction::Edit => "edit",
            FooterAction::Change => "change",
            FooterAction::Apply => "apply",
            FooterAction::Revert => "revert",
            FooterAction::Cancel => "cancel",
            FooterAction::Discard => "discard",
            FooterAction::Dismiss => "dismiss",
        }
    }

    #[test]
    fn source_contract_parity_uses_the_shared_shell_navigation_footer_and_editor() {
        let ShellLayout::Ready(areas) = ApplicationShell::new().layout(Rect::new(
            0,
            0,
            HUMAN_SHELL.minimum_width,
            HUMAN_SHELL.minimum_height,
        )) else {
            panic!("the published minimum shell must be renderable");
        };
        assert_eq!(areas.title.height, HUMAN_SHELL.header_rows);
        assert_eq!(areas.footer.height, HUMAN_SHELL.footer_rows);
        assert_eq!(areas.navigation.width, HUMAN_SHELL.sidebar_columns);

        let mut navigation = NavigationState::new();
        navigation.move_by(-1);
        assert_eq!(navigation.current(), Route::Settings);
        navigation.move_by(1);
        assert_eq!(navigation.current(), Route::Overview);

        for state in [
            FooterState::NormalNavigationWithRefresh,
            FooterState::LocalList,
            FooterState::Detail,
            FooterState::SettingsEdit,
            FooterState::DirtyDraft,
            FooterState::DiscardConfirmation,
            FooterState::HelpOverlay,
            FooterState::ConflictWarning,
        ] {
            let footer = format_footer(state, action_name);
            assert!(!footer.contains('·'));
            if state.hints().len() > 1 {
                assert!(footer.contains("  "), "{state:?}: {footer}");
            } else {
                assert!(!footer.is_empty(), "{state:?}");
            }
        }

        let mut editor = SettingsEditor::new(FixtureField::One);
        editor.enter_or_finish();
        assert!(editor.is_editing());
        assert!(editor.move_field(&[FixtureField::One, FixtureField::Two], 1));
        assert_eq!(editor.selected_field(), FixtureField::Two);
        assert_eq!(editor.request_quit(true), QuitDisposition::ConfirmDiscard);
        assert!(editor.awaiting_discard_confirmation());
        assert!(!editor.resolve_discard(DiscardDecision::Cancel));
        assert_eq!(editor.request_quit(true), QuitDisposition::ConfirmDiscard);
        assert!(editor.resolve_discard(DiscardDecision::Discard));
    }

    #[test]
    fn source_contract_admits_only_press_events_and_help_owns_input() {
        for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
            let mut key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
            key.kind = kind;
            assert_eq!(command_for(key, false), None);
        }
        let mut press = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        press.kind = KeyEventKind::Press;
        assert_eq!(command_for(press, false), Some(Command::Down));

        let mut app = App::loading(crate::analytics::TimeWindow::Last7Days);
        app.handle(Command::Help);
        assert!(app.help_open());
        app.handle(Command::Down);
        assert_eq!(app.screen(), Screen::Overview);
        app.handle(Command::Quit);
        assert!(!app.help_open());
    }

    #[test]
    fn no_color_preserves_text_and_selection_semantics_without_terminal_colors() {
        let theme = Theme::no_color();
        for role in [
            ColorRole::Primary,
            ColorRole::Secondary,
            ColorRole::Success,
            ColorRole::Warning,
            ColorRole::Danger,
            ColorRole::Info,
            ColorRole::Muted,
            ColorRole::Border,
            ColorRole::Selected,
            ColorRole::Background,
        ] {
            let style = theme.color_style(role);
            assert_eq!(style.fg, None, "{role:?}");
            assert_eq!(style.bg, None, "{role:?}");
        }
        assert!(
            theme
                .color_style(ColorRole::Selected)
                .add_modifier
                .contains(Modifier::REVERSED)
        );
        assert!(
            theme
                .typography_style(TypographyRole::ApplicationTitle)
                .add_modifier
                .contains(Modifier::BOLD)
        );
    }

    #[test]
    fn minimum_size_has_no_partial_shared_shell() {
        assert!(matches!(
            ApplicationShell::new().layout(Rect::new(
                0,
                0,
                HUMAN_SHELL.minimum_width.saturating_sub(1),
                HUMAN_SHELL.minimum_height,
            )),
            ShellLayout::TooSmall { .. }
        ));
        assert!(matches!(
            ApplicationShell::new().layout(Rect::new(
                0,
                0,
                HUMAN_SHELL.minimum_width,
                HUMAN_SHELL.minimum_height.saturating_sub(1),
            )),
            ShellLayout::TooSmall { .. }
        ));
    }
}
