//! First-run setup wizard — collects AWS profile, region, and default
//! PEM key path step-by-step, then saves the config.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::state::{ConfigDraft, SetupStep};
use crate::ui::theme::Theme;

pub fn render(
    frame: &mut Frame,
    main_area: Rect,
    status_area: Rect,
    theme: Theme,
    step: &SetupStep,
    _draft: &ConfigDraft,
    current_input: &str,
    editing: bool,
) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(18), Constraint::Min(8), Constraint::Percentage(18)])
        .split(main_area);
    let row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(14), Constraint::Min(44), Constraint::Percentage(14)])
        .split(outer[1]);
    let panel = row[1];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .title(" avconnect — Setup ");
    let inner = block.inner(panel);
    frame.render_widget(block, panel);

    let (title, value) = match step {
        SetupStep::AwsProfileOrKeys => ("AWS profile name", current_input),
        SetupStep::Region => ("AWS regions (comma-separated)", current_input),
        SetupStep::DefaultPem => ("Default PEM key path", current_input),
        SetupStep::Save => ("Save config", "Press Enter to save"),
    };

    let mode = if matches!(step, SetupStep::Save) {
        "Confirm"
    } else if editing {
        "Editing"
    } else {
        "Selection"
    };
    let p = Paragraph::new(format!("{}\n\nMode: {}\n\nValue: {}", title, mode, value))
        .style(ratatui::style::Style::default().fg(theme.text()))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(p, inner);

    crate::ui::components::render_status_bar(
        frame,
        status_area,
        theme,
        if matches!(step, SetupStep::Save) {
            " Enter: Save  Esc: Back "
        } else if editing {
            " Type input  Enter: Next  Esc: Stop editing "
        } else {
            " Enter: Edit  Esc: Back "
        },
    );
}
