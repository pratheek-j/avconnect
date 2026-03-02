//! Config screen — renders editable fields for AWS credentials,
//! SSH settings, and PEM key paths. Supports Tab navigation and Enter to save.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;

use crate::state::ConfigDraft;
use crate::ui::theme::Theme;

pub const CONFIG_FIELD_LABELS: &[&str] = &[
    "AWS profile",
    "AWS regions (comma-separated)",
    "Default SSH profile",
    "AWS access key ID (optional)",
    "AWS secret key (optional)",
    "SSH default key path",
    "SSH hosts file path (optional)",
    "Import source path (optional)",
];

pub fn draft_field_value(draft: &ConfigDraft, index: usize) -> &str {
    match index {
        0 => &draft.aws_profile,
        1 => &draft.aws_region,
        2 => &draft.default_profile,
        3 => &draft.aws_access_key_id,
        4 => &draft.aws_secret_access_key,
        5 => &draft.ssh_default_key,
        6 => &draft.ssh_hosts_file,
        7 => &draft.import_path,
        _ => "",
    }
}

pub fn config_field_name(index: usize) -> &'static str {
    match index {
        0 => "aws_profile",
        1 => "aws_region",
        2 => "default_profile",
        3 => "aws_access_key_id",
        4 => "aws_secret_access_key",
        5 => "ssh_default_key",
        6 => "ssh_hosts_file",
        7 => "import_path",
        _ => "",
    }
}

pub fn render(
    frame: &mut Frame,
    main_area: Rect,
    status_area: Rect,
    theme: Theme,
    draft: &ConfigDraft,
    focused_field: usize,
    editing: bool,
) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(12), Constraint::Min(10), Constraint::Percentage(12)])
        .split(main_area);
    let row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(12), Constraint::Min(50), Constraint::Percentage(12)])
        .split(outer[1]);
    let panel = row[1];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .title(" avconnect — Config ");
    let inner = block.inner(panel);
    frame.render_widget(block, panel);

    let rows: Vec<Row> = CONFIG_FIELD_LABELS
        .iter()
        .enumerate()
        .map(|(i, &label)| {
            let value = draft_field_value(draft, i);
            let style = if i == focused_field && editing {
                Style::default().fg(theme.text()).bg(theme.accent())
            } else if i == focused_field {
                theme.accent_bright()
            } else {
                Style::default().fg(theme.text())
            };
            Row::new(vec![Cell::from(label.to_string()), Cell::from(value.to_string())]).style(style)
        })
        .collect();
    let table = Table::new(rows, [Constraint::Length(36), Constraint::Min(12)])
        .header(Row::new(vec!["Field", "Value"]).style(theme.accent_bright()))
        .column_spacing(2);
    frame.render_widget(table, inner);

    let footer = if editing {
        "Editing field (Enter/Esc to stop)"
    } else {
        "Use Up/Down to choose a field, Enter to edit, Ctrl+S/F2 to save"
    };
    let footer_p = Paragraph::new(footer)
        .style(Style::default().fg(theme.muted()))
        .alignment(Alignment::Center);
    let footer_area = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(1),
        width: inner.width,
        height: 1,
    };
    frame.render_widget(footer_p, footer_area);

    crate::ui::components::render_status_bar(
        frame,
        status_area,
        theme,
        if editing {
            " Editing: type text  Enter/Esc: stop editing "
        } else {
            " ↑/↓: Move field  Enter: Edit  Ctrl+S/F2: Save  Esc: Back "
        },
    );
}
