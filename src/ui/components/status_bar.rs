//! Bottom status bar — displays contextual keybinding hints.

use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use crate::ui::theme::Theme;

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, theme: Theme, hint: &str) {
    let bar = Paragraph::new(hint)
        .style(theme.status_bar())
        .block(Block::new().style(theme.border()));
    frame.render_widget(bar, area);
}
