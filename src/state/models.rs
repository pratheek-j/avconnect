//! Core data models — Host, PemKey, FileNode, TransferJob, Config, and
//! their associated enums used across the application.

use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Host {
    pub name: String,
    pub address: String,
    pub username: String,
    pub port: u16,
    pub key_path: Option<PathBuf>,
    pub state: Option<InstanceState>,
    pub source: HostSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstanceState {
    Pending,
    Running,
    Stopping,
    Stopped,
    ShuttingDown,
    Terminated,
}

#[derive(Clone, Debug)]
pub enum HostSource {
    Aws {
        instance_id: String,
        region: String,
    },
    Config,
    Imported,
    Local,
    Static,
}

#[derive(Clone, Debug)]
pub struct PemKey {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub enum FileKind {
    File,
    Dir,
}

#[derive(Clone, Debug)]
pub struct FileNode {
    pub name: String,
    pub path: PathBuf,
    pub kind: FileKind,
    pub size: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct TransferJob {
    pub source_path: PathBuf,
    pub bytes_total: u64,
    pub bytes_done: u64,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub aws: AwsConfig,
    pub ssh: SshConfig,
    pub keys: Vec<PemKey>,
    pub profiles: Vec<SshProfile>,
    pub hosts: Vec<ManagedHost>,
    pub import_path: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AwsConfig {
    pub profile: String,
    pub regions: Vec<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SshConfig {
    pub default_key: String,
    pub hosts_file_path: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SshProfile {
    pub name: String,
    pub username: String,
    pub port: u16,
    pub key_path: PathBuf,
    pub is_default: bool,
}

#[derive(Clone, Debug)]
pub struct ManagedHost {
    pub name: String,
    pub address: String,
    pub username: String,
    pub port: u16,
    pub key_path: Option<PathBuf>,
}

/// Editable draft for config screen; all fields as strings.
#[derive(Clone, Debug, Default)]
pub struct ConfigDraft {
    pub aws_profile: String,
    pub aws_region: String,
    pub aws_access_key_id: String,
    pub aws_secret_access_key: String,
    pub ssh_default_key: String,
    pub ssh_hosts_file: String,
    pub default_profile: String,
    pub import_path: String,
    pub key_names: Vec<String>,
    pub key_paths: Vec<String>,
}
