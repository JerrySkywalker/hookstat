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
}

pub fn command_for(key: KeyEvent) -> Option<Command> {
    if key.kind != KeyEventKind::Press {
        return None;
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
        assert_eq!(command_for(key), None);
        key.kind = KeyEventKind::Release;
        assert_eq!(command_for(key), None);
        key.kind = KeyEventKind::Press;
        assert_eq!(command_for(key), Some(Command::Down));
    }

    #[test]
    fn required_commands_have_stable_bindings() {
        assert_eq!(
            command_for(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Some(Command::Enter)
        );
        assert_eq!(
            command_for(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE)),
            Some(Command::Refresh)
        );
        assert_eq!(
            command_for(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
            Some(Command::Quit)
        );
    }
}
