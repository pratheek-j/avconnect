//! Theme system — detects terminal color depth at startup and provides
//! a consistent palette (xterm-256 Rich vs 16-color Basic) for all screens.

use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Copy, Debug)]
pub enum Theme {
    Rich,
    Basic,
}

impl Theme {
    pub fn detect() -> Self {
        if crossterm::style::available_color_count() >= 256 {
            Theme::Rich
        } else {
            Theme::Basic
        }
    }

    pub fn accent(&self) -> Color {
        match self {
            Theme::Rich => Color::Indexed(39),
            Theme::Basic => Color::Cyan,
        }
    }

    pub fn accent_dim(&self) -> Color {
        match self {
            Theme::Rich => Color::Indexed(75),
            Theme::Basic => Color::White,
        }
    }

    pub fn accent_bright(&self) -> Style {
        match self {
            Theme::Rich => Style::default().fg(Color::Indexed(51)).add_modifier(Modifier::BOLD),
            Theme::Basic => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        }
    }

    pub fn selection_bg(&self) -> Color {
        match self {
            Theme::Rich => Color::Indexed(27),
            Theme::Basic => Color::Blue,
        }
    }

    pub fn muted(&self) -> Color {
        match self {
            Theme::Rich => Color::Indexed(244),
            Theme::Basic => Color::DarkGray,
        }
    }

    pub fn error(&self) -> Color {
        match self {
            Theme::Rich => Color::Indexed(196),
            Theme::Basic => Color::Red,
        }
    }

    pub fn text(&self) -> Color {
        match self {
            Theme::Rich => Color::Indexed(255),
            Theme::Basic => Color::White,
        }
    }

    /// Border style (unfocused)
    pub fn border(&self) -> Style {
        Style::default().fg(self.accent_dim())
    }

    /// Border style (focused)
    pub fn border_focused(&self) -> Style {
        Style::default().fg(self.accent())
    }

    /// Selected list item
    pub fn selected(&self) -> Style {
        Style::default()
            .fg(self.text())
            .bg(self.selection_bg())
            .add_modifier(Modifier::BOLD)
    }

    /// Status bar
    pub fn status_bar(&self) -> Style {
        Style::default().fg(self.text()).bg(self.muted())
    }

}
