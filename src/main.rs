//! Entry point — initializes the terminal in raw/alternate-screen mode,
//! sets up a panic hook to restore it, and hands off to the App event loop.

use std::io;

use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

mod app;
mod message;
mod reducer;
mod services;
mod state;
mod ui;

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && (args[1] == "--host" || args[1] == "-H") {
        let target = &args[2];
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let hosts = rt
            .block_on(crate::services::load_hosts())
            .unwrap_or_default();
        if let Some(host) = hosts
            .into_iter()
            .find(|h| h.address == *target || h.name == *target)
        {
            if let Ok(cfg) = crate::services::load_config() {
                if let Some(profile) = crate::services::default_profile(&cfg) {
                    if let Err(e) = crate::services::ssh_connect(&host, &profile) {
                        eprintln!("ssh failed: {}", e);
                        std::process::exit(1);
                    }
                    std::process::exit(0);
                }
            }
        }
        eprintln!("Host not found or no default profile configured: {}", target);
        std::process::exit(1);
    }

    let direct_sftp_args = if args.len() >= 4 && args[1] == "-sftp" {
        Some((args[2].clone(), args[3].clone()))
    } else {
        None
    };

    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    crossterm::execute!(stdout, EnterAlternateScreen)?;

    let panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        panic_hook(info);
    }));

    let (msg_tx, msg_rx) = std::sync::mpsc::channel();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = app::App::new(msg_tx);
    app.maybe_first_run_screen();
    if let Some((src, dst)) = direct_sftp_args {
        app.set_direct_sftp_mode(src, dst);
    }
    let result = app.run(&mut terminal, &msg_rx)?;
    restore_terminal()?;
    match result {
        app::AppResult::Quit => std::process::exit(0),
        app::AppResult::RunSsh(host, profile) => {
            if let Err(e) = crate::services::ssh_connect(&host, &profile) {
                eprintln!("ssh failed: {}", e);
                std::process::exit(1);
            }
            std::process::exit(0);
        }
    }
}

fn restore_terminal() -> io::Result<()> {
    terminal::disable_raw_mode()?;
    crossterm::execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}
