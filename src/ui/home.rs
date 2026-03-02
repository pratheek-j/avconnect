//! Home screen — renders the main menu (SSH, SFTP, Config, Quit) with
//! arrow-key navigation and Enter to confirm.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::ui::theme::Theme;

// Menu labels: ASCII-only for maximum compatibility. Nerd Font icons can be
// added with a theme/terminal check and fallback to these labels.
const MENU_ITEMS: &[&str] = &["SSH", "SFTP", "Config", "Quit"];

pub fn render(
    frame: &mut Frame,
    main_area: Rect,
    status_area: Rect,
    theme: Theme,
    selected: usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(4)])
        .split(main_area);

    let title = Paragraph::new("avconnect")
        .style(theme.accent_bright())
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_focused())
                .title(" avconnect — Home "),
        );
    frame.render_widget(title, chunks[0]);

    let items: Vec<ListItem> = MENU_ITEMS
        .iter()
        .enumerate()
        .map(|(i, &label)| {
            let style = if i == selected {
                theme.selected()
            } else {
                ratatui::style::Style::default().fg(theme.text())
            };
            ListItem::new(label).style(style)
        })
        .collect();
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border())
            .title(" Menu "),
    );
    frame.render_widget(list, chunks[1]);

    crate::ui::components::render_status_bar(
        frame,
        status_area,
        theme,
        " ↑/↓: Select  Enter: Confirm  q: Quit ",
    );
}
