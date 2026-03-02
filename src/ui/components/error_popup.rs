//! Centered error popup overlay — displays an error message with a
//! dismissible prompt on top of the current screen.

use ratatui::layout::{Alignment, Rect};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::ui::theme::Theme;

const POPUP_WIDTH: u16 = 60;
const POPUP_HEIGHT: u16 = 6;

pub fn render(frame: &mut Frame, area: Rect, theme: Theme, message: &str) {
    let width = area.width.min(POPUP_WIDTH);
    let height = area.height.min(POPUP_HEIGHT);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    let rect = Rect::new(x, y, width, height);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(ratatui::style::Style::default().fg(theme.error()))
        .title(" Error ");
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let p = Paragraph::new(format!("{}\n\nPress Enter or Esc to dismiss", message))
        .style(ratatui::style::Style::default().fg(theme.text()))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
    frame.render_widget(p, inner);
}
