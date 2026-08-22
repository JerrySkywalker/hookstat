//! Semantic theme and typography tokens for the internal terminal UI system.

use ratatui::style::{Color, Modifier, Style};
use terminal_ui_contract::chrome::ChromeToken;

use crate::interface_preferences::InterfaceColor;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThemeKind {
    Default,
    Monochrome,
    NoColor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub enum ColorRole {
    Primary,
    Secondary,
    Success,
    Warning,
    Danger,
    Info,
    Muted,
    Border,
    Selected,
    Background,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub enum TypographyRole {
    ApplicationTitle,
    SectionTitle,
    FieldLabel,
    Value,
    Metadata,
    Status,
}

/// A semantic palette. Widgets name roles, never terminal colors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Theme {
    kind: ThemeKind,
}

impl Theme {
    pub const fn default_color() -> Self {
        Self {
            kind: ThemeKind::Default,
        }
    }

    pub const fn monochrome() -> Self {
        Self {
            kind: ThemeKind::Monochrome,
        }
    }

    pub const fn no_color() -> Self {
        Self {
            kind: ThemeKind::NoColor,
        }
    }

    pub fn from_environment() -> Self {
        if std::env::var_os("NO_COLOR").is_some() {
            Self::no_color()
        } else if std::env::var("HOOKSTAT_TUI_THEME")
            .is_ok_and(|value| value.eq_ignore_ascii_case("monochrome"))
        {
            Self::monochrome()
        } else {
            Self::default_color()
        }
    }

    pub fn from_interface_color(color: InterfaceColor) -> Self {
        match color {
            InterfaceColor::Auto => Self::from_environment(),
            InterfaceColor::Always => {
                if std::env::var("HOOKSTAT_TUI_THEME")
                    .is_ok_and(|value| value.eq_ignore_ascii_case("monochrome"))
                {
                    Self::monochrome()
                } else {
                    Self::default_color()
                }
            }
            InterfaceColor::Never => Self::no_color(),
        }
    }

    pub fn color_style(self, role: ColorRole) -> Style {
        match self.kind {
            ThemeKind::Default => default_color_style(role),
            ThemeKind::Monochrome => monochrome_style(role),
            ThemeKind::NoColor => no_color_style(role),
        }
    }

    /// Maps dependency-neutral shared chrome roles to this Ratatui version.
    pub fn chrome_style(self, token: ChromeToken) -> Style {
        match token {
            ChromeToken::HeaderTitle => self.typography_style(TypographyRole::ApplicationTitle),
            ChromeToken::Base
            | ChromeToken::Border
            | ChromeToken::Footer
            | ChromeToken::CurrentScreen
            | ChromeToken::Overlay => self.color_style(ColorRole::Primary),
        }
    }

    pub fn typography_style(self, role: TypographyRole) -> Style {
        let color = match role {
            TypographyRole::ApplicationTitle => ColorRole::Primary,
            TypographyRole::SectionTitle => ColorRole::Primary,
            TypographyRole::FieldLabel => ColorRole::Secondary,
            TypographyRole::Value => ColorRole::Primary,
            TypographyRole::Metadata => ColorRole::Muted,
            TypographyRole::Status => ColorRole::Info,
        };
        let modifier = match role {
            TypographyRole::ApplicationTitle | TypographyRole::SectionTitle => Modifier::BOLD,
            TypographyRole::FieldLabel | TypographyRole::Metadata => Modifier::DIM,
            TypographyRole::Value | TypographyRole::Status => Modifier::empty(),
        };
        self.color_style(color).add_modifier(modifier)
    }
}

fn default_color_style(role: ColorRole) -> Style {
    match role {
        ColorRole::Primary => Style::default().fg(Color::Cyan),
        ColorRole::Secondary => Style::default().fg(Color::Gray),
        ColorRole::Success => Style::default().fg(Color::Green),
        ColorRole::Warning => Style::default().fg(Color::Yellow),
        ColorRole::Danger => Style::default().fg(Color::Red),
        ColorRole::Info => Style::default().fg(Color::Blue),
        ColorRole::Muted => Style::default().fg(Color::DarkGray),
        ColorRole::Border => Style::default().fg(Color::Gray),
        ColorRole::Selected => Style::default().add_modifier(Modifier::REVERSED),
        ColorRole::Background => Style::default(),
    }
}

fn monochrome_style(role: ColorRole) -> Style {
    match role {
        ColorRole::Selected => Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD),
        ColorRole::Primary | ColorRole::Info => Style::default().add_modifier(Modifier::BOLD),
        ColorRole::Secondary | ColorRole::Muted => Style::default().add_modifier(Modifier::DIM),
        ColorRole::Success
        | ColorRole::Warning
        | ColorRole::Danger
        | ColorRole::Border
        | ColorRole::Background => Style::default(),
    }
}

fn no_color_style(role: ColorRole) -> Style {
    match role {
        ColorRole::Selected => Style::default().add_modifier(Modifier::REVERSED),
        ColorRole::Primary | ColorRole::Info => Style::default().add_modifier(Modifier::BOLD),
        ColorRole::Secondary | ColorRole::Muted => Style::default().add_modifier(Modifier::DIM),
        ColorRole::Success
        | ColorRole::Warning
        | ColorRole::Danger
        | ColorRole::Border
        | ColorRole::Background => Style::default(),
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::default_color()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_roles_have_distinct_default_mappings() {
        let theme = Theme::default_color();
        assert_eq!(theme.color_style(ColorRole::Danger).fg, Some(Color::Red));
        assert_eq!(theme.color_style(ColorRole::Success).fg, Some(Color::Green));
        assert_eq!(theme.color_style(ColorRole::Selected).bg, None);
    }

    #[test]
    fn monochrome_preserves_selection_without_color() {
        let style = Theme::monochrome().color_style(ColorRole::Selected);
        assert_eq!(style.fg, None);
        assert_eq!(style.bg, None);
        assert!(style.add_modifier.contains(Modifier::REVERSED));
    }

    #[test]
    fn explicit_color_policy_can_override_auto_detection() {
        assert_eq!(
            Theme::from_interface_color(InterfaceColor::Never),
            Theme::no_color()
        );
    }
}
