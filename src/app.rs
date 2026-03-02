//! Application core — owns the TUI state, processes keyboard/async events
//! through the reducer, and coordinates side effects like host loading,
//! SFTP connections, and config persistence.

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::sync::mpsc;

use crate::message::AppMessage;
use crate::reducer::{reduce, ReducerResult};
use crate::state::{
    ConfigDraft, Host, ManagedHost, PemKey, Screen, SetupStep, SshProfile,
};
use crate::ui;
use crate::ui::config_view;

pub enum AppResult {
    Quit,
    RunSsh(Host, SshProfile),
}

pub struct App {
    screen: Screen,
    theme: ui::Theme,
    msg_tx: mpsc::Sender<AppMessage>,
    pending_ssh: Option<(Host, SshProfile)>,
    pending_sftp_connect: Option<(Host, SshProfile)>,
    direct_sftp: Option<(String, String)>,
    direct_sftp_source_connected: bool,
}

impl App {
    pub fn new(msg_tx: mpsc::Sender<AppMessage>) -> Self {
        Self {
            screen: Screen::Home { selected: 0 },
            theme: ui::Theme::detect(),
            msg_tx,
            pending_ssh: None,
            pending_sftp_connect: None,
            direct_sftp: None,
            direct_sftp_source_connected: false,
        }
    }

    pub fn set_direct_sftp_mode(&mut self, source: String, dest: String) {
        self.direct_sftp = Some((source, dest));
        self.screen = Screen::LoadingHosts {
            purpose: crate::state::HostLoadPurpose::SftpSource,
        };
    }

    pub fn maybe_first_run_screen(&mut self) {
        if let Ok(true) = crate::services::first_run_needed() {
            self.screen = Screen::FirstRunSetup {
                step: SetupStep::AwsProfileOrKeys,
                draft: ConfigDraft::default(),
                current_input: String::new(),
                editing: false,
            };
        }
    }

    pub fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        msg_rx: &mpsc::Receiver<AppMessage>,
    ) -> io::Result<AppResult> {
        self.draw(terminal)?;
        loop {
            while let Ok(msg) = msg_rx.try_recv() {
                if let Some(result) = self.dispatch(msg, terminal)? {
                    return Ok(result);
                }
            }

            if event::poll(std::time::Duration::from_millis(100))? {
                if let Some(msgs) = self.handle_event() {
                    for msg in msgs {
                        if let Some(result) = self.dispatch(msg, terminal)? {
                            return Ok(result);
                        }
                    }
                }
            }

            // Side effects set directly by handle_event (bypassing the reducer)
            self.maybe_spawn_sftp_connect();
            if let Some(pair) = self.pending_ssh.take() {
                return Ok(AppResult::RunSsh(pair.0, pair.1));
            }
        }
    }

    fn dispatch(
        &mut self,
        msg: AppMessage,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> io::Result<Option<AppResult>> {
        let prev = self.screen.clone();
        match reduce(self.screen.clone(), msg) {
            ReducerResult::Screen(s) => self.screen = s,
            ReducerResult::Quit => return Ok(Some(AppResult::Quit)),
        }
        self.maybe_spawn_load_hosts(&prev);
        self.maybe_spawn_sftp_connect();
        self.maybe_autoselect_direct_sftp_source();
        self.maybe_load_config_after_transition();
        self.maybe_save_config_on_save_step();
        if let Some(pair) = self.pending_ssh.take() {
            return Ok(Some(AppResult::RunSsh(pair.0, pair.1)));
        }
        self.draw(terminal)?;
        Ok(None)
    }

    fn maybe_spawn_sftp_connect(&mut self) {
        if let Some((host, profile)) = self.pending_sftp_connect.take() {
            crate::services::sftp::spawn_sftp_worker(&host, &profile, self.msg_tx.clone());
        }
    }

    fn maybe_autoselect_direct_sftp_source(&mut self) {
        if self.direct_sftp_source_connected {
            return;
        }
        let Some((source, _)) = &self.direct_sftp else {
            return;
        };
        if let Screen::SftpSourceSelect { hosts, .. } = &self.screen {
            if let Some(host) = hosts
                .iter()
                .find(|h| h.address == *source || h.name == *source)
                .cloned()
            {
                if let Ok(cfg) = crate::services::load_config() {
                    if let Some(profile) = crate::services::default_profile(&cfg) {
                        self.pending_sftp_connect = Some((host, profile));
                        self.direct_sftp_source_connected = true;
                    }
                }
            }
        }
    }

    fn maybe_spawn_load_hosts(&self, prev: &Screen) {
        let just_entered = matches!(self.screen, Screen::LoadingHosts { .. })
            && !matches!(prev, Screen::LoadingHosts { .. });
        if just_entered {
            let tx = self.msg_tx.clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let result = rt.block_on(crate::services::load_hosts());
                let msg = match result {
                    Ok(h) => AppMessage::HostsLoaded(h),
                    Err(e) => AppMessage::HostsLoadFailed(e.to_string()),
                };
                let _ = tx.send(msg);
            });
        }
    }

    fn maybe_load_config_after_transition(&mut self) {
        if let Screen::Config {
            draft,
            focused_field,
            editing,
        } = &self.screen
        {
            let is_empty = draft.aws_profile.is_empty()
                && draft.aws_region.is_empty()
                && draft.ssh_default_key.is_empty();
            if is_empty {
                if let Ok(cfg) = crate::services::load_config() {
                    self.screen = Screen::Config {
                        draft: crate::services::config_to_draft(&cfg),
                        focused_field: *focused_field,
                        editing: *editing,
                    };
                }
            }
        }
    }

    fn maybe_save_config_on_save_step(&mut self) {
        if let Screen::FirstRunSetup {
            step: SetupStep::Save,
            draft,
            ..
        } = &self.screen
        {
            let cfg = crate::services::draft_to_config(draft);
            if crate::services::save_config(&cfg).is_ok() {
                self.screen = Screen::Home { selected: 0 };
            }
        }
    }

    fn handle_event(&mut self) -> Option<Vec<AppMessage>> {
        let Ok(Event::Key(key)) = event::read() else {
            return None;
        };
        if key.kind != KeyEventKind::Press {
            return None;
        }
        if let Some(msgs) = self.handle_new_host_form_key(&key) {
            return Some(msgs);
        }
        let mut out = Vec::new();
        match &self.screen {
            Screen::Home { selected } => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => out.push(AppMessage::Quit),
                KeyCode::Up => out.push(AppMessage::MenuSelect(selected.saturating_sub(1))),
                KeyCode::Down => out.push(AppMessage::MenuSelect((*selected + 1).min(3))),
                KeyCode::Enter => out.push(AppMessage::MenuConfirm),
                _ => {}
            },

            Screen::FirstRunSetup {
                step,
                current_input,
                editing,
                ..
            } => match key.code {
                KeyCode::Esc => {
                    if *editing {
                        out.push(AppMessage::FirstRunEditStop);
                    } else {
                        out.push(AppMessage::GoBack);
                    }
                }
                KeyCode::Enter => {
                    if matches!(step, SetupStep::Save) {
                        out.push(AppMessage::SetupStepCompleted(step.clone()));
                    } else if *editing {
                        out.push(AppMessage::SetupStepCompleted(step.clone()));
                    } else {
                        out.push(AppMessage::FirstRunEditStart);
                    }
                }
                KeyCode::Char(c) if *editing => {
                    let mut s = current_input.clone();
                    s.push(c);
                    out.push(AppMessage::FirstRunInputChanged(s));
                }
                KeyCode::Backspace if *editing => {
                    let mut s = current_input.clone();
                    s.pop();
                    out.push(AppMessage::FirstRunInputChanged(s));
                }
                _ => {}
            },

            Screen::SshSelect {
                query,
                hosts,
                filtered,
                selected,
                ..
            } => match key.code {
                KeyCode::Esc => out.push(AppMessage::GoBack),
                KeyCode::Up => out.push(AppMessage::HostListMoveUp),
                KeyCode::Down => out.push(AppMessage::HostListMoveDown),
                KeyCode::F(5) => out.push(AppMessage::RefreshHosts),
                KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    out.push(AppMessage::OpenNewHostModal)
                }
                KeyCode::Delete => {
                    if !filtered.is_empty() {
                        let host = hosts[filtered[*selected]].clone();
                        let _ = crate::services::delete_managed_host(&host.address);
                        out.push(AppMessage::RefreshHosts);
                    }
                }
                KeyCode::Enter => {
                    if filtered.is_empty() {
                        return None;
                    }
                    let host = hosts[filtered[*selected]].clone();
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        if let Ok(cfg) = crate::services::load_config() {
                            out.push(AppMessage::OpenKeySelect {
                                host,
                                profiles: cfg.profiles.clone(),
                            });
                        }
                    } else {
                        match crate::services::load_config() {
                            Ok(cfg) => {
                                if let Some(profile) = crate::services::default_profile(&cfg) {
                                    self.pending_ssh = Some((host, profile));
                                }
                            }
                            Err(e) => out.push(AppMessage::HostsLoadFailed(e.to_string())),
                        }
                    }
                }
                KeyCode::Char(c) => {
                    let mut q = query.clone();
                    q.push(c);
                    out.push(AppMessage::SearchQueryChanged(q));
                }
                KeyCode::Backspace => {
                    let mut q = query.clone();
                    q.pop();
                    out.push(AppMessage::SearchQueryChanged(q));
                }
                _ => {}
            },

            Screen::SftpSourceSelect {
                query,
                hosts,
                filtered,
                selected,
                ..
            } => match key.code {
                KeyCode::Esc => out.push(AppMessage::GoBack),
                KeyCode::Up => out.push(AppMessage::HostListMoveUp),
                KeyCode::Down => out.push(AppMessage::HostListMoveDown),
                KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    out.push(AppMessage::OpenNewHostModal)
                }
                KeyCode::Delete => {
                    if !filtered.is_empty() {
                        let host = hosts[filtered[*selected]].clone();
                        let _ = crate::services::delete_managed_host(&host.address);
                        out.push(AppMessage::RefreshHosts);
                    }
                }
                KeyCode::Enter => {
                    if filtered.is_empty() {
                        return None;
                    }
                    let host = hosts[filtered[*selected]].clone();
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        if let Ok(cfg) = crate::services::load_config() {
                            out.push(AppMessage::OpenSftpKeySelect {
                                host,
                                profiles: cfg.profiles.clone(),
                            });
                        }
                    } else if let Ok(cfg) = crate::services::load_config() {
                        if let Some(profile) = crate::services::default_profile(&cfg) {
                            self.pending_sftp_connect = Some((host, profile));
                        }
                    }
                }
                KeyCode::Char(c) => {
                    let mut q = query.clone();
                    q.push(c);
                    out.push(AppMessage::SearchQueryChanged(q));
                }
                KeyCode::Backspace => {
                    let mut q = query.clone();
                    q.pop();
                    out.push(AppMessage::SearchQueryChanged(q));
                }
                _ => {}
            },

            Screen::SftpDestSelect {
                query,
                hosts,
                filtered,
                selected,
                ..
            } => match key.code {
                KeyCode::Esc => out.push(AppMessage::GoBack),
                KeyCode::Up => out.push(AppMessage::HostListMoveUp),
                KeyCode::Down => out.push(AppMessage::HostListMoveDown),
                KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    out.push(AppMessage::OpenNewHostModal)
                }
                KeyCode::Delete => {
                    if !filtered.is_empty() {
                        let host = hosts[filtered[*selected]].clone();
                        let _ = crate::services::delete_managed_host(&host.address);
                        out.push(AppMessage::RefreshHosts);
                    }
                }
                KeyCode::Enter => {
                    if filtered.is_empty() {
                        return None;
                    }
                    out.push(AppMessage::SftpStartTransfer);
                }
                KeyCode::Char(c) => {
                    let mut q = query.clone();
                    q.push(c);
                    out.push(AppMessage::SearchQueryChanged(q));
                }
                KeyCode::Backspace => {
                    let mut q = query.clone();
                    q.pop();
                    out.push(AppMessage::SearchQueryChanged(q));
                }
                _ => {}
            },

            Screen::SftpBrowser {
                host,
                key: pem_key,
                selections,
                ..
            } => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => out.push(AppMessage::GoBack),
                KeyCode::Up => out.push(AppMessage::SftpBrowserMoveUp),
                KeyCode::Down => out.push(AppMessage::SftpBrowserMoveDown),
                KeyCode::Char(' ') => out.push(AppMessage::SftpBrowserToggleSelection),
                KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    let source = crate::state::SftpSource {
                        host: host.clone(),
                        key: pem_key.clone(),
                        paths: selections.clone(),
                    };
                    out.push(AppMessage::SftpConfirmSelection(source));
                }
                KeyCode::Enter => out.push(AppMessage::SftpBrowserToggleSelection),
                _ => {}
            },

            Screen::SftpKeySelect {
                host,
                profiles,
                selected,
            } => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => out.push(AppMessage::GoBack),
                KeyCode::Enter => {
                    if !profiles.is_empty() {
                        self.pending_sftp_connect =
                            Some((host.clone(), profiles[*selected].clone()));
                    }
                }
                KeyCode::Up => {
                    let n = profiles.len().saturating_sub(1);
                    out.push(AppMessage::KeySelectMove(
                        selected.saturating_sub(1).min(n),
                    ));
                }
                KeyCode::Down => {
                    let n = profiles.len().saturating_sub(1);
                    out.push(AppMessage::KeySelectMove((*selected + 1).min(n)));
                }
                _ => {}
            },

            Screen::SshKeySelect {
                host,
                profiles,
                selected,
            } => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => out.push(AppMessage::GoBack),
                KeyCode::Enter => {
                    if !profiles.is_empty() {
                        self.pending_ssh = Some((host.clone(), profiles[*selected].clone()));
                    }
                }
                KeyCode::Up => {
                    let n = profiles.len().saturating_sub(1);
                    out.push(AppMessage::KeySelectMove(
                        selected.saturating_sub(1).min(n),
                    ));
                }
                KeyCode::Down => {
                    let n = profiles.len().saturating_sub(1);
                    out.push(AppMessage::KeySelectMove((*selected + 1).min(n)));
                }
                _ => {}
            },

            Screen::Config {
                draft,
                focused_field,
                editing,
            } => match key.code {
                KeyCode::Esc => {
                    if *editing {
                        out.push(AppMessage::ConfigEditStop);
                    } else {
                        out.push(AppMessage::GoBack);
                    }
                }
                KeyCode::Enter => {
                    if *editing {
                        out.push(AppMessage::ConfigEditStop);
                    } else {
                        out.push(AppMessage::ConfigEditStart);
                    }
                }
                KeyCode::Up => out.push(AppMessage::ConfigFocusPrev),
                KeyCode::Down => out.push(AppMessage::ConfigFocusNext),
                KeyCode::F(2) => {
                    let cfg = crate::services::draft_to_config(draft);
                    if crate::services::save_config(&cfg).is_ok() {
                        out.push(AppMessage::ConfigSaved);
                    }
                }
                KeyCode::F(6) => {
                    let import_path = if draft.import_path.is_empty() {
                        None
                    } else {
                        Some(draft.import_path.as_str())
                    };
                    if let Err(e) = crate::services::import_hosts_from_ssh_config(import_path) {
                        out.push(AppMessage::HostsLoadFailed(e.to_string()));
                    }
                }
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    let cfg = crate::services::draft_to_config(draft);
                    if crate::services::save_config(&cfg).is_ok() {
                        out.push(AppMessage::ConfigSaved);
                    }
                }
                KeyCode::Char(c) if *editing => {
                    let field = config_view::config_field_name(*focused_field).to_string();
                    let current =
                        config_view::draft_field_value(draft, *focused_field).to_string();
                    out.push(AppMessage::ConfigFieldEdited {
                        field,
                        value: format!("{}{}", current, c),
                    });
                }
                KeyCode::Backspace if *editing => {
                    let field = config_view::config_field_name(*focused_field).to_string();
                    let mut value =
                        config_view::draft_field_value(draft, *focused_field).to_string();
                    value.pop();
                    out.push(AppMessage::ConfigFieldEdited { field, value });
                }
                _ => {}
            },

            Screen::Error { .. } => match key.code {
                KeyCode::Esc | KeyCode::Enter => out.push(AppMessage::DismissError),
                _ => {}
            },

            _ => {
                if key.code == KeyCode::Esc {
                    out.push(AppMessage::GoBack);
                }
            }
        }
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    fn handle_new_host_form_key(&mut self, key: &KeyEvent) -> Option<Vec<AppMessage>> {
        let form_opt = match &mut self.screen {
            Screen::SshSelect { new_host_form, .. } => new_host_form,
            Screen::SftpSourceSelect { new_host_form, .. } => new_host_form,
            Screen::SftpDestSelect { new_host_form, .. } => new_host_form,
            _ => return None,
        };
        let Some(form) = form_opt else {
            return None;
        };
        match key.code {
            KeyCode::Esc => {
                *form_opt = None;
                Some(vec![])
            }
            KeyCode::Up => {
                form.focused_field = form.focused_field.saturating_sub(1);
                Some(vec![])
            }
            KeyCode::Down | KeyCode::Tab => {
                form.focused_field = (form.focused_field + 1).min(4);
                Some(vec![])
            }
            KeyCode::Backspace => {
                let target = match form.focused_field {
                    0 => &mut form.hostname,
                    1 => &mut form.ip,
                    2 => &mut form.username,
                    3 => &mut form.port,
                    _ => &mut form.pem_path,
                };
                target.pop();
                Some(vec![])
            }
            KeyCode::Char(c) => {
                let target = match form.focused_field {
                    0 => &mut form.hostname,
                    1 => &mut form.ip,
                    2 => &mut form.username,
                    3 => &mut form.port,
                    _ => &mut form.pem_path,
                };
                target.push(c);
                Some(vec![])
            }
            KeyCode::Enter => {
                let port = form.port.parse::<u16>().unwrap_or(22);
                let host = ManagedHost {
                    name: form.hostname.clone(),
                    address: form.ip.clone(),
                    username: if form.username.is_empty() {
                        "ubuntu".to_string()
                    } else {
                        form.username.clone()
                    },
                    port,
                    key_path: if form.pem_path.is_empty() {
                        None
                    } else {
                        Some(std::path::PathBuf::from(form.pem_path.as_str()))
                    },
                };
                let _ = crate::services::add_managed_host(host);
                if !form.pem_path.is_empty() {
                    if let Ok(mut cfg) = crate::services::load_config() {
                        let exists = cfg
                            .keys
                            .iter()
                            .any(|k| k.path == std::path::PathBuf::from(form.pem_path.as_str()));
                        if !exists {
                            cfg.keys.push(PemKey {
                                name: format!("manual-{}", cfg.keys.len() + 1),
                                path: std::path::PathBuf::from(form.pem_path.as_str()),
                            });
                            let _ = crate::services::save_config(&cfg);
                        }
                    }
                }
                *form_opt = None;
                Some(vec![AppMessage::RefreshHosts])
            }
            _ => Some(vec![]),
        }
    }

    fn draw(
        &self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> io::Result<()> {
        terminal.draw(|f| {
            let area = f.area();
            ui::render_screen(f, area, &self.screen, &self.theme);
        })?;
        Ok(())
    }
}
