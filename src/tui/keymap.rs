//! Press-only key-to-command mapping.

use crate::analytics::TimeWindow;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Command {
    Up,
    Down,
    Enter,
    Back,
    Refresh,
    Quit,
    ToggleFocus,
    Window(TimeWindow),
    Search,
    SearchInput(char),
    SearchBackspace,
    CloseSearch,
    Filter,
    Sort,
    PreviousSetting,
    NextSetting,
    RevertSettings,
}

pub fn command_for(key: KeyEvent, search_editing: bool) -> Option<Command> {
    if key.kind != KeyEventKind::Press {
        return None;
    }
    if search_editing {
        return match key.code {
            KeyCode::Esc | KeyCode::Enter => Some(Command::CloseSearch),
            KeyCode::Backspace => Some(Command::SearchBackspace),
            KeyCode::Char(value)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                Some(Command::SearchInput(value))
            }
            _ => None,
        };
    }
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(Command::Up),
        KeyCode::Down | KeyCode::Char('j') => Some(Command::Down),
        KeyCode::Enter => Some(Command::Enter),
        KeyCode::Esc | KeyCode::Backspace => Some(Command::Back),
        KeyCode::Char('r') => Some(Command::Refresh),
        KeyCode::Char('q') => Some(Command::Quit),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Command::Quit),
        KeyCode::Tab => Some(Command::ToggleFocus),
        KeyCode::Char('1') => Some(Command::Window(TimeWindow::Last24Hours)),
        KeyCode::Char('7') => Some(Command::Window(TimeWindow::Last7Days)),
        KeyCode::Char('3') => Some(Command::Window(TimeWindow::Last30Days)),
        KeyCode::Char('a') => Some(Command::Window(TimeWindow::All)),
        KeyCode::Char('/') => Some(Command::Search),
        KeyCode::Char('f') => Some(Command::Filter),
        KeyCode::Char('s') => Some(Command::Sort),
        KeyCode::Left | KeyCode::Char('h') => Some(Command::PreviousSetting),
        KeyCode::Right | KeyCode::Char('l') => Some(Command::NextSetting),
        KeyCode::Char('x') => Some(Command::RevertSettings),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_navigation_is_press_only() {
        let mut key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        key.kind = KeyEventKind::Repeat;
        assert_eq!(command_for(key, false), None);
        key.kind = KeyEventKind::Release;
        assert_eq!(command_for(key, false), None);
        key.kind = KeyEventKind::Press;
        assert_eq!(command_for(key, false), Some(Command::Down));
    }

    #[test]
    fn required_commands_have_stable_bindings() {
        assert_eq!(
            command_for(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), false),
            Some(Command::Enter)
        );
        assert_eq!(
            command_for(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE), false),
            Some(Command::Search)
        );
        assert_eq!(
            command_for(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE), false),
            Some(Command::Refresh)
        );
    }

    #[test]
    fn search_mode_accepts_literal_query_text() {
        assert_eq!(
            command_for(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE), true),
            Some(Command::SearchInput('q'))
        );
        assert_eq!(
            command_for(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), true),
            Some(Command::CloseSearch)
        );
    }
}
