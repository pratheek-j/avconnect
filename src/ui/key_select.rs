//! PEM key picker — modal list of available SSH keys for the selected host.

use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

use crate::state::SshProfile;
use crate::ui::theme::Theme;

pub fn render(
    frame: &mut Frame,
    main_area: Rect,
    status_area: Rect,
    theme: Theme,
    host_name: &str,
    profiles: &[SshProfile],
    selected: usize,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .title(format!(" avconnect — Select profile for {} ", host_name));
    let inner = block.inner(main_area);
    frame.render_widget(block, main_area);

    let items: Vec<ListItem> = profiles
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let style = if i == selected {
                theme.selected()
            } else {
                ratatui::style::Style::default().fg(theme.text())
            };
            ListItem::new(format!(
                "{}  {}@*:{}  {}",
                p.name,
                p.username,
                p.port,
                p.key_path.display()
            ))
            .style(style)
        })
        .collect();
    let list = List::new(items);
    frame.render_widget(list, inner);

    crate::ui::components::render_status_bar(
        frame,
        status_area,
        theme,
        " Enter: Use profile  q: Back ",
    );
}
