//! Responsive application-shell geometry.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub const MINIMUM_WIDTH: u16 = 24;
pub const MINIMUM_HEIGHT: u16 = 10;

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
        if area.width < MINIMUM_WIDTH || area.height < MINIMUM_HEIGHT {
            return ShellLayout::TooSmall { available: area };
        }

        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(2),
            ])
            .split(area);
        let navigation_width = if vertical[1].width >= 52 { 21 } else { 12 };
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(navigation_width), Constraint::Min(10)])
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
        assert_eq!(areas.title.height, 3);
        assert_eq!(areas.footer.height, 2);
        assert_eq!(areas.navigation.width, 21);
        assert!(areas.content.width > areas.navigation.width);
    }

    #[test]
    fn narrow_layout_keeps_content_and_navigation() {
        let ShellLayout::Ready(areas) = ApplicationShell::new().layout(Rect::new(0, 0, 30, 10))
        else {
            panic!("minimum terminal must have a shell");
        };
        assert_eq!(areas.navigation.width, 12);
        assert!(areas.content.width >= 10);
    }

    #[test]
    fn too_small_layout_has_no_partial_widgets() {
        assert!(matches!(
            ApplicationShell::new().layout(Rect::new(0, 0, 23, 10)),
            ShellLayout::TooSmall { .. }
        ));
        assert!(matches!(
            ApplicationShell::new().layout(Rect::new(0, 0, 24, 9)),
            ShellLayout::TooSmall { .. }
        ));
    }
}
