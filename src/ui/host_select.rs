//! Searchable host list — shared by SSH, SFTP source, and SFTP destination
//! screens. Renders a search bar and a filterable list of hosts.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;

use crate::state::{Host, HostSource, InstanceState, NewHostForm};
use crate::ui::theme::Theme;

pub fn render(
    frame: &mut Frame,
    main_area: Rect,
    status_area: Rect,
    theme: Theme,
    title: &str,
    hosts: &[Host],
    filtered: &[usize],
    selected: usize,
    query: &str,
    new_host_form: Option<&NewHostForm>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(4)])
        .split(main_area);

    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .title(format!(" {} ", title));
    let search_para = Paragraph::new(format!("Search: {}", query))
        .style(ratatui::style::Style::default().fg(theme.text()))
        .block(search_block);
    frame.render_widget(search_para, chunks[0]);

    let visible_rows = chunks[1].height.saturating_sub(3) as usize;
    let start = if visible_rows == 0 {
        0
    } else {
        selected.saturating_sub(visible_rows / 2)
    };
    let end = if visible_rows == 0 {
        filtered.len()
    } else {
        (start + visible_rows).min(filtered.len())
    };

    let rows: Vec<Row> = filtered[start..end]
        .iter()
        .map(|&i| hosts.get(i).expect("filtered host index must be valid"))
        .enumerate()
        .map(|(offset, host)| {
            let idx = start + offset;
            let state_str = host.state.as_ref().map(state_short).unwrap_or("static");
            let state_style = state_style(&theme, host.state.as_ref());
            let source_str = match &host.source {
                HostSource::Aws {
                    region,
                    instance_id,
                } => {
                    let short_id = instance_id.chars().take(10).collect::<String>();
                    format!("{} ({})", region, short_id)
                }
                HostSource::Static => "static".to_string(),
                HostSource::Config => "config".to_string(),
                HostSource::Imported => "imported".to_string(),
                HostSource::Local => "local".to_string(),
            };
            let source_style = source_style(&source_str);
            let base = Style::default().fg(theme.text());
            let style = if idx == selected { theme.selected() } else { base };
            Row::new(vec![
                Cell::from(host.name.clone()),
                Cell::from(host.address.clone()),
                Cell::from(host.username.clone()),
                Cell::from(state_str).style(state_style),
                Cell::from(source_str).style(source_style),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(28),
            Constraint::Percentage(27),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
        ],
    )
    .header(
        Row::new(vec!["Name", "Address", "User", "State", "Source"]).style(
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
    )
    .column_spacing(1)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border())
            .title(" Hosts "),
    );
    frame.render_widget(table, chunks[1]);

    if let Some(form) = new_host_form {
        render_new_host_popup(frame, main_area, theme, form);
    }

    crate::ui::components::render_status_bar(
        frame,
        status_area,
        theme,
        " Type to search  ↑/↓: Move  Enter: Select  Ctrl+Enter: Profile  Ctrl+N: Add  Del: Remove ",
    );
}

fn state_short(s: &InstanceState) -> &'static str {
    use crate::state::InstanceState::*;
    match s {
        Pending => "pending",
        Running => "running",
        Stopping => "stopping",
        Stopped => "stopped",
        ShuttingDown => "shutting-down",
        Terminated => "terminated",
    }
}

fn state_style(theme: &Theme, state: Option<&InstanceState>) -> Style {
    match state {
        Some(InstanceState::Running) => Style::default().fg(ratatui::style::Color::Green),
        Some(InstanceState::Stopped) => Style::default().fg(ratatui::style::Color::Yellow),
        Some(InstanceState::Terminated) => Style::default().fg(theme.error()),
        Some(InstanceState::Pending) | Some(InstanceState::Stopping) | Some(InstanceState::ShuttingDown) => {
            Style::default().fg(ratatui::style::Color::Cyan)
        }
        None => Style::default().fg(theme.muted()),
    }
}

fn source_style(source: &str) -> Style {
    let palette = [
        ratatui::style::Color::Cyan,
        ratatui::style::Color::Magenta,
        ratatui::style::Color::Green,
        ratatui::style::Color::Yellow,
        ratatui::style::Color::Blue,
    ];
    let idx = source.bytes().fold(0usize, |acc, b| acc.wrapping_add(b as usize)) % palette.len();
    Style::default()
        .fg(palette[idx])
        .add_modifier(Modifier::BOLD)
        .bg(ratatui::style::Color::Reset)
}

fn render_new_host_popup(frame: &mut Frame, area: Rect, theme: Theme, form: &NewHostForm) {
    let width = area.width.min(80);
    let height = area.height.min(12);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    let popup = Rect {
        x,
        y,
        width,
        height,
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .title(" Add Host (Enter submit, Esc cancel) ");
    frame.render_widget(block, popup);
    let inner = Block::default().inner(popup);
    let lines = [
        format_field("Hostname", &form.hostname, form.focused_field == 0),
        format_field("IP", &form.ip, form.focused_field == 1),
        format_field("Username", &form.username, form.focused_field == 2),
        format_field("Port", &form.port, form.focused_field == 3),
        format_field("PEM path", &form.pem_path, form.focused_field == 4),
    ];
    let para = Paragraph::new(lines.join("\n"))
        .style(Style::default().fg(theme.text()))
        .wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(para, inner);
}

fn format_field(label: &str, value: &str, focused: bool) -> String {
    if focused {
        format!("> {:<10}: {}", label, value)
    } else {
        format!("  {:<10}: {}", label, value)
    }
}
