//! Screen enum — the finite state machine defining every possible
//! application screen and its associated data.

use crate::state::{ConfigDraft, FileNode, Host, PemKey, SshProfile, TransferJob};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum HostLoadPurpose {
    Ssh,
    SftpSource,
    SftpDest(SftpSource),
}

#[derive(Clone, Debug, PartialEq)]
pub enum SetupStep {
    AwsProfileOrKeys,
    Region,
    DefaultPem,
    Save,
}

#[derive(Clone, Debug, Default)]
pub struct TreeState {
    pub root: Vec<FileNode>,
    pub selected_index: usize,
}

#[derive(Clone, Debug)]
pub struct SftpSource {
    pub host: Host,
    pub key: PemKey,
    pub paths: Vec<PathBuf>,
}

#[derive(Clone, Debug, Default)]
pub struct NewHostForm {
    pub hostname: String,
    pub ip: String,
    pub username: String,
    pub port: String,
    pub pem_path: String,
    pub focused_field: usize,
}

#[derive(Clone, Debug)]
pub enum Screen {
    Home { selected: usize },
    LoadingHosts { purpose: HostLoadPurpose },
    SshSelect {
        query: String,
        hosts: Vec<Host>,
        filtered: Vec<usize>,
        selected: usize,
        new_host_form: Option<NewHostForm>,
    },
    SshKeySelect {
        host: Host,
        profiles: Vec<SshProfile>,
        selected: usize,
    },
    SftpSourceSelect {
        query: String,
        hosts: Vec<Host>,
        filtered: Vec<usize>,
        selected: usize,
        new_host_form: Option<NewHostForm>,
    },
    SftpKeySelect {
        host: Host,
        profiles: Vec<SshProfile>,
        selected: usize,
    },
    SftpBrowser {
        host: Host,
        key: PemKey,
        tree: TreeState,
        selections: Vec<PathBuf>,
    },
    SftpDestSelect {
        source: SftpSource,
        query: String,
        hosts: Vec<Host>,
        filtered: Vec<usize>,
        selected: usize,
        new_host_form: Option<NewHostForm>,
    },
    SftpProgress {
        jobs: Vec<TransferJob>,
        cancelled: bool,
    },
    Config {
        draft: ConfigDraft,
        focused_field: usize,
        editing: bool,
    },
    FirstRunSetup {
        step: SetupStep,
        draft: ConfigDraft,
        current_input: String,
        editing: bool,
    },
    Error {
        message: String,
        return_to: Box<Screen>,
    },
}
