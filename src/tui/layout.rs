//! Responsive application-shell geometry.

use jerry_terminal_ui::layout::HUMAN_SHELL;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApplicationShell;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShellAreas {
    pub title: Rect,
    pub navigation: Rect,
    pub content: Rect,
    pub footer: Rect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellLayout {
    Ready(ShellAreas),
    TooSmall { available: Rect },
}

impl ApplicationShell {
    pub const fn new() -> Self {
        Self
    }

    pub fn layout(self, area: Rect) -> ShellLayout {
        if !HUMAN_SHELL.supports(area.width, area.height) {
            return ShellLayout::TooSmall { available: area };
        }

        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(HUMAN_SHELL.header_rows),
                Constraint::Min(3),
                Constraint::Length(HUMAN_SHELL.footer_rows),
            ])
            .split(area);
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(HUMAN_SHELL.sidebar_columns),
                Constraint::Min(3),
            ])
            .split(vertical[1]);
        ShellLayout::Ready(ShellAreas {
            title: vertical[0],
            navigation: horizontal[0],
            content: horizontal[1],
            footer: vertical[2],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_layout_has_all_four_regions() {
        let ShellLayout::Ready(areas) = ApplicationShell::new().layout(Rect::new(0, 0, 100, 30))
        else {
            panic!("normal terminal must have a shell");
        };
        assert_eq!(areas.title.height, HUMAN_SHELL.header_rows);
        assert_eq!(areas.footer.height, 2);
        assert_eq!(areas.navigation.width, HUMAN_SHELL.sidebar_columns);
        assert!(areas.content.width > areas.navigation.width);
    }

    #[test]
    fn narrow_layout_keeps_content_and_navigation() {
        let ShellLayout::Ready(areas) = ApplicationShell::new().layout(Rect::new(0, 0, 30, 10))
        else {
            panic!("minimum terminal must have a shell");
        };
        assert_eq!(areas.navigation.width, HUMAN_SHELL.sidebar_columns);
        assert!(areas.content.width >= 3);
    }

    #[test]
    fn too_small_layout_has_no_partial_widgets() {
        assert!(matches!(
            ApplicationShell::new().layout(Rect::new(
                0,
                0,
                HUMAN_SHELL.minimum_width - 1,
                HUMAN_SHELL.minimum_height
            )),
            ShellLayout::TooSmall { .. }
        ));
        assert!(matches!(
            ApplicationShell::new().layout(Rect::new(
                0,
                0,
                HUMAN_SHELL.minimum_width,
                HUMAN_SHELL.minimum_height - 1
            )),
            ShellLayout::TooSmall { .. }
        ));
    }
}
