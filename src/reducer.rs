//! Central reducer — pure function mapping (Screen, AppMessage) -> Screen.
//! All state transitions live here; side effects are handled in app.rs.

use crate::message::AppMessage;
use crate::state::{ConfigDraft, Host, HostLoadPurpose, Screen, SetupStep};

const HOME_MENU_LEN: usize = 4;
const CONFIG_FIELD_COUNT: usize = 8;

pub enum ReducerResult {
    Screen(Screen),
    Quit,
}

fn apply_search(hosts: &[Host], query: &str) -> (Vec<usize>, usize) {
    use fuzzy_matcher::skim::SkimMatcherV2;
    use fuzzy_matcher::FuzzyMatcher;
    if query.is_empty() {
        return ((0..hosts.len()).collect(), 0);
    }
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, usize)> = hosts
        .iter()
        .enumerate()
        .filter_map(|(i, h)| matcher.fuzzy_match(&h.name, query).map(|s| (s, i)))
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    let filtered: Vec<usize> = scored.into_iter().map(|(_, i)| i).collect();
    (filtered, 0)
}

pub fn reduce(screen: Screen, msg: AppMessage) -> ReducerResult {
    match msg {
        AppMessage::Quit => return ReducerResult::Quit,

        AppMessage::MenuSelect(idx) => {
            if let Screen::Home { .. } = &screen {
                if idx < HOME_MENU_LEN {
                    return ReducerResult::Screen(Screen::Home { selected: idx });
                }
            }
        }

        AppMessage::MenuConfirm => {
            if let Screen::Home { selected } = &screen {
                return match *selected {
                    0 => ReducerResult::Screen(Screen::LoadingHosts {
                        purpose: HostLoadPurpose::Ssh,
                    }),
                    1 => ReducerResult::Screen(Screen::LoadingHosts {
                        purpose: HostLoadPurpose::SftpSource,
                    }),
                    2 => ReducerResult::Screen(Screen::Config {
                        draft: ConfigDraft::default(),
                        focused_field: 0,
                        editing: false,
                    }),
                    3 => ReducerResult::Quit,
                    _ => ReducerResult::Screen(screen),
                };
            }
        }

        AppMessage::GoBack => {
            return ReducerResult::Screen(Screen::Home { selected: 0 });
        }

        AppMessage::DismissError => {
            if let Screen::Error { return_to, .. } = &screen {
                return ReducerResult::Screen(*return_to.clone());
            }
        }

        AppMessage::HostsLoaded(hosts) => {
            if let Screen::LoadingHosts { purpose } = &screen {
                let mut hosts = hosts;
                if matches!(
                    purpose,
                    HostLoadPurpose::SftpSource | HostLoadPurpose::SftpDest(_)
                ) {
                    hosts.insert(
                        0,
                        Host {
                            name: "Local".to_string(),
                            address: "127.0.0.1".to_string(),
                            username: whoami::username(),
                            port: 22,
                            key_path: None,
                            state: None,
                            source: crate::state::HostSource::Local,
                        },
                    );
                }
                let filtered: Vec<usize> = (0..hosts.len()).collect();
                return match purpose {
                    HostLoadPurpose::Ssh => ReducerResult::Screen(Screen::SshSelect {
                        query: String::new(),
                        hosts,
                        filtered,
                        selected: 0,
                        new_host_form: None,
                    }),
                    HostLoadPurpose::SftpSource => {
                        ReducerResult::Screen(Screen::SftpSourceSelect {
                            query: String::new(),
                            hosts,
                            filtered,
                            selected: 0,
                            new_host_form: None,
                        })
                    }
                    HostLoadPurpose::SftpDest(source) => {
                        ReducerResult::Screen(Screen::SftpDestSelect {
                            source: source.clone(),
                            query: String::new(),
                            hosts,
                            filtered,
                            selected: 0,
                            new_host_form: None,
                        })
                    }
                };
            }
        }

        AppMessage::SftpConfirmSelection(source) => {
            if let Screen::SftpBrowser { .. } = &screen {
                return ReducerResult::Screen(Screen::LoadingHosts {
                    purpose: HostLoadPurpose::SftpDest(source),
                });
            }
        }

        AppMessage::SftpStartTransfer => {
            if let Screen::SftpDestSelect { .. } = &screen {
                return ReducerResult::Screen(Screen::SftpProgress {
                    jobs: vec![],
                    cancelled: false,
                });
            }
        }

        AppMessage::RefreshHosts => {
            match &screen {
                Screen::SshSelect { .. } => {
                    return ReducerResult::Screen(Screen::LoadingHosts {
                        purpose: HostLoadPurpose::Ssh,
                    });
                }
                Screen::SftpSourceSelect { .. } => {
                    return ReducerResult::Screen(Screen::LoadingHosts {
                        purpose: HostLoadPurpose::SftpSource,
                    });
                }
                Screen::SftpDestSelect { source, .. } => {
                    return ReducerResult::Screen(Screen::LoadingHosts {
                        purpose: HostLoadPurpose::SftpDest(source.clone()),
                    });
                }
                _ => {}
            }
        }

        AppMessage::OpenNewHostModal => match &screen {
            Screen::SshSelect {
                query,
                hosts,
                filtered,
                selected,
                ..
            } => {
                return ReducerResult::Screen(Screen::SshSelect {
                    query: query.clone(),
                    hosts: hosts.clone(),
                    filtered: filtered.clone(),
                    selected: *selected,
                    new_host_form: Some(crate::state::NewHostForm {
                        username: "ubuntu".to_string(),
                        port: "22".to_string(),
                        ..Default::default()
                    }),
                });
            }
            Screen::SftpSourceSelect {
                query,
                hosts,
                filtered,
                selected,
                ..
            } => {
                return ReducerResult::Screen(Screen::SftpSourceSelect {
                    query: query.clone(),
                    hosts: hosts.clone(),
                    filtered: filtered.clone(),
                    selected: *selected,
                    new_host_form: Some(crate::state::NewHostForm {
                        username: "ubuntu".to_string(),
                        port: "22".to_string(),
                        ..Default::default()
                    }),
                });
            }
            Screen::SftpDestSelect {
                source,
                query,
                hosts,
                filtered,
                selected,
                ..
            } => {
                return ReducerResult::Screen(Screen::SftpDestSelect {
                    source: source.clone(),
                    query: query.clone(),
                    hosts: hosts.clone(),
                    filtered: filtered.clone(),
                    selected: *selected,
                    new_host_form: Some(crate::state::NewHostForm {
                        username: "ubuntu".to_string(),
                        port: "22".to_string(),
                        ..Default::default()
                    }),
                });
            }
            _ => {}
        },

        AppMessage::HostsLoadFailed(msg) => {
            return ReducerResult::Screen(Screen::Error {
                message: msg,
                return_to: Box::new(screen.clone()),
            });
        }

        AppMessage::SearchQueryChanged(query) => match &screen {
            Screen::SftpDestSelect {
                source,
                hosts,
                new_host_form,
                ..
            } => {
                let (filtered, selected) = apply_search(hosts, &query);
                return ReducerResult::Screen(Screen::SftpDestSelect {
                    source: source.clone(),
                    query,
                    hosts: hosts.clone(),
                    filtered,
                    selected,
                    new_host_form: new_host_form.clone(),
                });
            }
            Screen::SshSelect {
                hosts,
                new_host_form,
                ..
            } => {
                let (filtered, selected) = apply_search(hosts, &query);
                return ReducerResult::Screen(Screen::SshSelect {
                    query,
                    hosts: hosts.clone(),
                    filtered,
                    selected,
                    new_host_form: new_host_form.clone(),
                });
            }
            Screen::SftpSourceSelect {
                hosts,
                new_host_form,
                ..
            } => {
                let (filtered, selected) = apply_search(hosts, &query);
                return ReducerResult::Screen(Screen::SftpSourceSelect {
                    query,
                    hosts: hosts.clone(),
                    filtered,
                    selected,
                    new_host_form: new_host_form.clone(),
                });
            }
            _ => {}
        },

        AppMessage::OpenKeySelect { host, profiles } => {
            if let Screen::SshSelect { .. } = &screen {
                return ReducerResult::Screen(Screen::SshKeySelect {
                    host,
                    profiles,
                    selected: 0,
                });
            }
        }

        AppMessage::OpenSftpKeySelect { host, profiles } => {
            if let Screen::SftpSourceSelect { .. } = &screen {
                return ReducerResult::Screen(Screen::SftpKeySelect {
                    host,
                    profiles,
                    selected: 0,
                });
            }
        }

        AppMessage::SftpConnected {
            host,
            profile,
            root_entries,
        } => {
            if let Screen::SftpSourceSelect { .. } | Screen::SftpKeySelect { .. } = &screen {
                return ReducerResult::Screen(Screen::SftpBrowser {
                    host,
                    key: crate::state::PemKey {
                        name: profile.name.clone(),
                        path: profile.key_path.clone(),
                    },
                    tree: crate::state::TreeState {
                        root: root_entries,
                        selected_index: 0,
                    },
                    selections: vec![],
                });
            }
        }

        AppMessage::SftpBrowserMoveUp => {
            if let Screen::SftpBrowser {
                host,
                key,
                tree,
                selections,
            } = &screen
            {
                let mut next = tree.clone();
                next.selected_index = next.selected_index.saturating_sub(1);
                return ReducerResult::Screen(Screen::SftpBrowser {
                    host: host.clone(),
                    key: key.clone(),
                    tree: next,
                    selections: selections.clone(),
                });
            }
        }

        AppMessage::SftpBrowserMoveDown => {
            if let Screen::SftpBrowser {
                host,
                key,
                tree,
                selections,
            } = &screen
            {
                let mut next = tree.clone();
                if !next.root.is_empty() {
                    let n = next.root.len().saturating_sub(1);
                    next.selected_index = (next.selected_index + 1).min(n);
                }
                return ReducerResult::Screen(Screen::SftpBrowser {
                    host: host.clone(),
                    key: key.clone(),
                    tree: next,
                    selections: selections.clone(),
                });
            }
        }

        AppMessage::SftpBrowserToggleSelection => {
            if let Screen::SftpBrowser {
                host,
                key,
                tree,
                selections,
            } = &screen
            {
                let mut next_selections = selections.clone();
                if let Some(node) = tree.root.get(tree.selected_index) {
                    if let Some(pos) = next_selections.iter().position(|p| p == &node.path) {
                        next_selections.remove(pos);
                    } else {
                        next_selections.push(node.path.clone());
                    }
                }
                return ReducerResult::Screen(Screen::SftpBrowser {
                    host: host.clone(),
                    key: key.clone(),
                    tree: tree.clone(),
                    selections: next_selections,
                });
            }
        }

        AppMessage::HostListMoveUp => match &screen {
            Screen::SftpDestSelect {
                source,
                query,
                hosts,
                filtered,
                selected,
                new_host_form,
            } => {
                let n = filtered.len().saturating_sub(1);
                return ReducerResult::Screen(Screen::SftpDestSelect {
                    source: source.clone(),
                    query: query.clone(),
                    hosts: hosts.clone(),
                    filtered: filtered.clone(),
                    selected: selected.saturating_sub(1).min(n),
                    new_host_form: new_host_form.clone(),
                });
            }
            Screen::SshSelect {
                query,
                hosts,
                filtered,
                selected,
                new_host_form,
            } => {
                let n = filtered.len().saturating_sub(1);
                return ReducerResult::Screen(Screen::SshSelect {
                    query: query.clone(),
                    hosts: hosts.clone(),
                    filtered: filtered.clone(),
                    selected: selected.saturating_sub(1).min(n),
                    new_host_form: new_host_form.clone(),
                });
            }
            Screen::SftpSourceSelect {
                query,
                hosts,
                filtered,
                selected,
                new_host_form,
            } => {
                let n = filtered.len().saturating_sub(1);
                return ReducerResult::Screen(Screen::SftpSourceSelect {
                    query: query.clone(),
                    hosts: hosts.clone(),
                    filtered: filtered.clone(),
                    selected: selected.saturating_sub(1).min(n),
                    new_host_form: new_host_form.clone(),
                });
            }
            _ => {}
        },

        AppMessage::KeySelectMove(sel) => match &screen {
            Screen::SshKeySelect { host, profiles, .. } => {
                let n = profiles.len().saturating_sub(1);
                return ReducerResult::Screen(Screen::SshKeySelect {
                    host: host.clone(),
                    profiles: profiles.clone(),
                    selected: sel.min(n),
                });
            }
            Screen::SftpKeySelect { host, profiles, .. } => {
                let n = profiles.len().saturating_sub(1);
                return ReducerResult::Screen(Screen::SftpKeySelect {
                    host: host.clone(),
                    profiles: profiles.clone(),
                    selected: sel.min(n),
                });
            }
            _ => {}
        },

        AppMessage::HostListMoveDown => match &screen {
            Screen::SftpDestSelect {
                source,
                query,
                hosts,
                filtered,
                selected,
                new_host_form,
            } => {
                let n = filtered.len().saturating_sub(1);
                return ReducerResult::Screen(Screen::SftpDestSelect {
                    source: source.clone(),
                    query: query.clone(),
                    hosts: hosts.clone(),
                    filtered: filtered.clone(),
                    selected: (selected + 1).min(n),
                    new_host_form: new_host_form.clone(),
                });
            }
            Screen::SshSelect {
                query,
                hosts,
                filtered,
                selected,
                new_host_form,
            } => {
                let n = filtered.len().saturating_sub(1);
                return ReducerResult::Screen(Screen::SshSelect {
                    query: query.clone(),
                    hosts: hosts.clone(),
                    filtered: filtered.clone(),
                    selected: (selected + 1).min(n),
                    new_host_form: new_host_form.clone(),
                });
            }
            Screen::SftpSourceSelect {
                query,
                hosts,
                filtered,
                selected,
                new_host_form,
            } => {
                let n = filtered.len().saturating_sub(1);
                return ReducerResult::Screen(Screen::SftpSourceSelect {
                    query: query.clone(),
                    hosts: hosts.clone(),
                    filtered: filtered.clone(),
                    selected: (selected + 1).min(n),
                    new_host_form: new_host_form.clone(),
                });
            }
            _ => {}
        },

        AppMessage::ConfigSaved => {
            return ReducerResult::Screen(Screen::Home { selected: 0 });
        }

        AppMessage::ConfigFocusNext => {
            if let Screen::Config {
                draft,
                focused_field,
                editing,
            } = &screen
            {
                return ReducerResult::Screen(Screen::Config {
                    draft: draft.clone(),
                    focused_field: (focused_field + 1) % CONFIG_FIELD_COUNT,
                    editing: *editing,
                });
            }
        }

        AppMessage::ConfigFocusPrev => {
            if let Screen::Config {
                draft,
                focused_field,
                editing,
            } = &screen
            {
                return ReducerResult::Screen(Screen::Config {
                    draft: draft.clone(),
                    focused_field: focused_field
                        .saturating_sub(1)
                        .min(CONFIG_FIELD_COUNT.saturating_sub(1)),
                    editing: *editing,
                });
            }
        }

        AppMessage::ConfigEditStart => {
            if let Screen::Config {
                draft, focused_field, ..
            } = &screen
            {
                return ReducerResult::Screen(Screen::Config {
                    draft: draft.clone(),
                    focused_field: *focused_field,
                    editing: true,
                });
            }
        }

        AppMessage::ConfigEditStop => {
            if let Screen::Config {
                draft, focused_field, ..
            } = &screen
            {
                return ReducerResult::Screen(Screen::Config {
                    draft: draft.clone(),
                    focused_field: *focused_field,
                    editing: false,
                });
            }
        }

        AppMessage::ConfigFieldEdited { field, value } => {
            if let Screen::Config {
                draft,
                focused_field,
                editing,
            } = &screen
            {
                let mut d = draft.clone();
                match field.as_str() {
                    "aws_profile" => d.aws_profile = value,
                    "aws_region" => d.aws_region = value,
                    "default_profile" => d.default_profile = value,
                    "aws_access_key_id" => d.aws_access_key_id = value,
                    "aws_secret_access_key" => d.aws_secret_access_key = value,
                    "ssh_default_key" => d.ssh_default_key = value,
                    "ssh_hosts_file" => d.ssh_hosts_file = value,
                    "import_path" => d.import_path = value,
                    _ => {}
                }
                return ReducerResult::Screen(Screen::Config {
                    draft: d,
                    focused_field: *focused_field,
                    editing: *editing,
                });
            }
        }

        AppMessage::FirstRunInputChanged(s) => {
            if let Screen::FirstRunSetup {
                step,
                draft,
                editing,
                ..
            } = &screen
            {
                return ReducerResult::Screen(Screen::FirstRunSetup {
                    step: step.clone(),
                    draft: draft.clone(),
                    current_input: s,
                    editing: *editing,
                });
            }
        }

        AppMessage::FirstRunEditStart => {
            if let Screen::FirstRunSetup {
                step,
                draft,
                current_input,
                ..
            } = &screen
            {
                return ReducerResult::Screen(Screen::FirstRunSetup {
                    step: step.clone(),
                    draft: draft.clone(),
                    current_input: current_input.clone(),
                    editing: true,
                });
            }
        }

        AppMessage::FirstRunEditStop => {
            if let Screen::FirstRunSetup {
                step,
                draft,
                current_input,
                ..
            } = &screen
            {
                return ReducerResult::Screen(Screen::FirstRunSetup {
                    step: step.clone(),
                    draft: draft.clone(),
                    current_input: current_input.clone(),
                    editing: false,
                });
            }
        }

        AppMessage::SetupStepCompleted(step) => {
            if let Screen::FirstRunSetup {
                step: current_step,
                draft,
                current_input,
                editing: _,
            } = &screen
            {
                if *current_step != step {
                    return ReducerResult::Screen(screen);
                }
                use SetupStep::*;
                let mut d = draft.clone();
                match step {
                    AwsProfileOrKeys => d.aws_profile = current_input.clone(),
                    Region => d.aws_region = current_input.clone(),
                    DefaultPem => d.ssh_default_key = current_input.clone(),
                    Save => {}
                }
                return match step {
                    AwsProfileOrKeys => ReducerResult::Screen(Screen::FirstRunSetup {
                        step: Region,
                        draft: d.clone(),
                        current_input: d.aws_region.clone(),
                        editing: false,
                    }),
                    Region => ReducerResult::Screen(Screen::FirstRunSetup {
                        step: DefaultPem,
                        draft: d.clone(),
                        current_input: d.ssh_default_key.clone(),
                        editing: false,
                    }),
                    DefaultPem => ReducerResult::Screen(Screen::FirstRunSetup {
                        step: Save,
                        draft: d,
                        current_input: String::new(),
                        editing: false,
                    }),
                    Save => ReducerResult::Screen(screen),
                };
            }
        }
    }
    ReducerResult::Screen(screen)
}
