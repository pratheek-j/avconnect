//! SFTP file browser — renders the remote directory tree as a navigable
//! list with directory/file indicators.

use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

use crate::state::TreeState;
use crate::ui::theme::Theme;

pub fn render(
    frame: &mut Frame,
    main_area: Rect,
    status_area: Rect,
    theme: Theme,
    tree: &TreeState,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .title(" avconnect — SFTP browser ");
    let inner = block.inner(main_area);
    frame.render_widget(block, main_area);

    let items: Vec<ListItem> = tree
        .root
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let style = if i == tree.selected_index {
                theme.selected()
            } else {
                ratatui::style::Style::default().fg(theme.text())
            };
            let kind = match &node.kind {
                crate::state::FileKind::Dir => "[dir]",
                crate::state::FileKind::File => "    ",
            };
            let size = node.size.map(|s| format!("{} B", s)).unwrap_or_default();
            ListItem::new(format!(
                "{} {}  {:>8}  {}",
                kind,
                node.name,
                size,
                node.path.display()
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
        " ↑/↓: Move  Space/Enter: Toggle select  Ctrl+Enter: Next  Esc: Back ",
    );
}
