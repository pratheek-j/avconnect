//! UI rendering — dispatches to per-screen render functions, provides
//! the shared main/status-bar layout split, and maps screens to hint text.

mod components;
pub mod config_view;
mod first_run;
mod home;
mod host_select;
mod key_select;
mod sftp_browser;
mod theme;

pub use home::render as render_home;
pub use theme::Theme;

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::state::Screen;

pub fn render_screen(frame: &mut Frame, area: Rect, screen: &Screen, theme: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);
    let main_area = chunks[0];
    let status_area = chunks[1];

    if let Screen::Error { message, .. } = screen {
        components::render_error_popup(frame, area, *theme, message);
        components::render_status_bar(frame, status_area, *theme, status_hint(screen));
        return;
    }

    match screen {
        Screen::Home { selected } => render_home(frame, main_area, status_area, *theme, *selected),

        Screen::LoadingHosts { .. } => {
            render_placeholder(frame, main_area, theme, "Loading hosts…");
            components::render_status_bar(frame, status_area, *theme, status_hint(screen));
        }

        Screen::SshSelect {
            query,
            hosts,
            filtered,
            selected,
            new_host_form,
        } => {
            host_select::render(
                frame, main_area, status_area, *theme, "avconnect — SSH", hosts, filtered,
                *selected, query, new_host_form.as_ref(),
            );
        }

        Screen::SshKeySelect {
            host,
            profiles,
            selected,
        } => {
            key_select::render(
                frame, main_area, status_area, *theme, &host.name, profiles, *selected,
            );
        }

        Screen::SftpSourceSelect {
            query,
            hosts,
            filtered,
            selected,
            new_host_form,
        } => {
            host_select::render(
                frame, main_area, status_area, *theme, "avconnect — SFTP source", hosts,
                filtered, *selected, query, new_host_form.as_ref(),
            );
        }

        Screen::SftpKeySelect {
            host,
            profiles,
            selected,
        } => {
            key_select::render(
                frame, main_area, status_area, *theme, &host.name, profiles, *selected,
            );
        }

        Screen::SftpBrowser { tree, .. } => {
            sftp_browser::render(frame, main_area, status_area, *theme, tree);
        }

        Screen::SftpDestSelect {
            source,
            query,
            hosts,
            filtered,
            selected,
            new_host_form,
        } => {
            let title = format!(
                "avconnect — SFTP destination (from {} via {} | {} path(s))",
                source.host.name,
                source.key.name,
                source.paths.len()
            );
            host_select::render(
                frame,
                main_area,
                status_area,
                *theme,
                &title,
                hosts,
                filtered,
                *selected,
                query,
                new_host_form.as_ref(),
            );
        }

        Screen::SftpProgress { jobs, cancelled } => {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_focused())
                .title(" avconnect — Transfer ");
            let inner = block.inner(main_area);
            frame.render_widget(block, main_area);

            let text = if *cancelled {
                "Transfer cancelled.".to_string()
            } else if jobs.is_empty() {
                "Preparing transfer…".to_string()
            } else {
                jobs.iter()
                    .map(|j| {
                        let pct = if j.bytes_total > 0 {
                            j.bytes_done * 100 / j.bytes_total
                        } else {
                            0
                        };
                        format!(
                            "{} -> {} ({}%)",
                            j.source_path.display(),
                            j.bytes_done,
                            pct
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            let p = Paragraph::new(text)
                .style(ratatui::style::Style::default().fg(theme.text()))
                .alignment(Alignment::Left);
            frame.render_widget(p, inner);
            components::render_status_bar(frame, status_area, *theme, status_hint(screen));
        }

        Screen::Config {
            draft,
            focused_field,
            editing,
        } => {
            config_view::render(
                frame,
                main_area,
                status_area,
                *theme,
                draft,
                *focused_field,
                *editing,
            );
        }

        Screen::FirstRunSetup {
            step,
            draft,
            current_input,
            editing,
        } => {
            first_run::render(
                frame,
                main_area,
                status_area,
                *theme,
                step,
                draft,
                current_input,
                *editing,
            );
        }

        Screen::Error { .. } => {}
    }
}

fn render_placeholder(frame: &mut Frame, area: Rect, theme: &Theme, msg: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(" avconnect ");
    let p = Paragraph::new(msg)
        .style(ratatui::style::Style::default().fg(theme.text()))
        .alignment(Alignment::Center)
        .block(block);
    frame.render_widget(p, area);
}

pub fn status_hint(screen: &Screen) -> &'static str {
    match screen {
        Screen::Home { .. } => " ↑/↓: Select  Enter: Confirm  q: Quit ",
        Screen::LoadingHosts { .. } => " Loading… ",
        Screen::SshSelect { .. } => " Type to search  ↑/↓: Move  Enter: Connect  Ctrl+Enter: Profile  Ctrl+N: Add  Del: Remove  F5: Refresh ",
        Screen::SshKeySelect { .. } => " ↑/↓: Select  Enter: Use profile  Esc: Back ",
        Screen::SftpSourceSelect { .. } => " Type to search  ↑/↓: Move  Enter: Select source  Ctrl+Enter: Profile  Ctrl+N: Add  Del: Remove ",
        Screen::SftpKeySelect { .. } => " ↑/↓: Select  Enter: Use profile  Esc: Back ",
        Screen::SftpBrowser { .. } => " Enter: Confirm selection  Esc: Back ",
        Screen::SftpDestSelect { .. } => " Type to search  ↑/↓: Move  Enter: Select destination  Esc: Back ",
        Screen::SftpProgress { .. } => " Esc: Cancel ",
        Screen::Config { editing, .. } => {
            if *editing {
                " Editing field: type/backspace  Enter/Esc: stop "
            } else {
                " ↑/↓: Move field  Enter: Edit  Ctrl+S/F2: Save  F6: Import  Esc: Back "
            }
        }
        Screen::FirstRunSetup { editing, .. } => {
            if *editing {
                " Type input  Enter: Next  Esc: Stop editing "
            } else {
                " Enter: Edit/Next  Esc: Back "
            }
        }
        Screen::Error { .. } => " Enter/Esc: Dismiss ",
    }
}
