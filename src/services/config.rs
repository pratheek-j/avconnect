//! Config persistence — TOML load/save for ~/.avconnect/config.toml,
//! plus helpers for converting between Config, ConfigDraft, and PemKey.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::state::{AwsConfig, Config, ConfigDraft, ManagedHost, PemKey, SshConfig, SshProfile};

const CONFIG_DIR: &str = ".avconnect";
const CONFIG_FILE: &str = "config.toml";

/// TOML-serializable shape (paths as strings).
#[derive(Debug, Serialize, Deserialize)]
struct TomlConfig {
    aws: TomlAws,
    ssh: TomlSsh,
    #[serde(default)]
    keys: Vec<TomlKey>,
    #[serde(default)]
    profiles: Vec<TomlProfile>,
    #[serde(default)]
    hosts: Vec<TomlHost>,
    #[serde(default)]
    import_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TomlAws {
    profile: String,
    #[serde(default)]
    region: Option<String>,
    #[serde(default)]
    regions: Vec<String>,
    access_key_id: Option<String>,
    secret_access_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TomlSsh {
    default_key: String,
    #[serde(rename = "hosts_file")]
    hosts_file: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TomlKey {
    name: String,
    path: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TomlProfile {
    name: String,
    username: String,
    port: u16,
    key_path: String,
    #[serde(default)]
    is_default: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct TomlHost {
    name: String,
    address: String,
    username: String,
    #[serde(default = "default_port")]
    port: u16,
    key_path: Option<String>,
}

fn default_port() -> u16 {
    22
}

pub fn config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("no home dir")?;
    Ok(home.join(CONFIG_DIR).join(CONFIG_FILE))
}

pub fn config_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("no home dir")?;
    Ok(home.join(CONFIG_DIR))
}

pub fn first_run_needed() -> Result<bool> {
    Ok(!config_path()?.exists())
}

pub fn load_config() -> Result<Config> {
    let path = config_path()?;
    let s = fs::read_to_string(&path).context("read config")?;
    let t: TomlConfig = toml::from_str(&s).context("parse config")?;
    Ok(Config {
        aws: AwsConfig {
            profile: t.aws.profile,
            regions: {
                if !t.aws.regions.is_empty() {
                    t.aws
                        .regions
                        .into_iter()
                        .map(|r| r.trim().to_string())
                        .filter(|r| !r.is_empty())
                        .collect()
                } else {
                    t.aws
                        .region
                        .unwrap_or_default()
                        .split(',')
                        .map(|r| r.trim().to_string())
                        .filter(|r| !r.is_empty())
                        .collect()
                }
            },
            access_key_id: t.aws.access_key_id,
            secret_access_key: t.aws.secret_access_key,
        },
        ssh: SshConfig {
            default_key: t.ssh.default_key.clone(),
            hosts_file_path: t.ssh.hosts_file,
        },
        keys: t
            .keys
            .into_iter()
            .map(|k| PemKey {
                name: k.name,
                path: PathBuf::from(k.path),
            })
            .collect(),
        profiles: {
            let mut profiles: Vec<SshProfile> = t
                .profiles
                .into_iter()
                .map(|p| SshProfile {
                    name: p.name,
                    username: p.username,
                    port: p.port,
                    key_path: PathBuf::from(p.key_path),
                    is_default: p.is_default,
                })
                .collect();
            if profiles.is_empty() {
                profiles.push(SshProfile {
                    name: "default".to_string(),
                    username: "ubuntu".to_string(),
                    port: 22,
                    key_path: PathBuf::from(t.ssh.default_key),
                    is_default: true,
                });
            } else if !profiles.iter().any(|p| p.is_default) {
                if let Some(first) = profiles.first_mut() {
                    first.is_default = true;
                }
            }
            profiles
        },
        hosts: t
            .hosts
            .into_iter()
            .map(|h| ManagedHost {
                name: h.name,
                address: h.address,
                username: h.username,
                port: h.port,
                key_path: h.key_path.map(PathBuf::from),
            })
            .collect(),
        import_path: t.import_path,
    })
}

pub fn save_config(cfg: &Config) -> Result<()> {
    let dir = config_dir()?;
    fs::create_dir_all(&dir).context("create config dir")?;
    let path = config_path()?;
    let t = TomlConfig {
        aws: TomlAws {
            profile: cfg.aws.profile.clone(),
            region: cfg.aws.regions.first().cloned(),
            regions: cfg.aws.regions.clone(),
            access_key_id: cfg.aws.access_key_id.clone(),
            secret_access_key: cfg.aws.secret_access_key.clone(),
        },
        ssh: TomlSsh {
            default_key: cfg.ssh.default_key.clone(),
            hosts_file: cfg.ssh.hosts_file_path.clone(),
        },
        keys: cfg
            .keys
            .iter()
            .map(|k| TomlKey {
                name: k.name.clone(),
                path: k.path.to_string_lossy().to_string(),
            })
            .collect(),
        profiles: cfg
            .profiles
            .iter()
            .map(|p| TomlProfile {
                name: p.name.clone(),
                username: p.username.clone(),
                port: p.port,
                key_path: p.key_path.to_string_lossy().to_string(),
                is_default: p.is_default,
            })
            .collect(),
        hosts: cfg
            .hosts
            .iter()
            .map(|h| TomlHost {
                name: h.name.clone(),
                address: h.address.clone(),
                username: h.username.clone(),
                port: h.port,
                key_path: h.key_path.as_ref().map(|k| k.to_string_lossy().to_string()),
            })
            .collect(),
        import_path: cfg.import_path.clone(),
    };
    let s = toml::to_string_pretty(&t).context("serialize config")?;
    fs::write(&path, s).context("write config")?;
    Ok(())
}

pub fn config_to_draft(cfg: &Config) -> ConfigDraft {
    ConfigDraft {
        aws_profile: cfg.aws.profile.clone(),
        aws_region: cfg.aws.regions.join(", "),
        aws_access_key_id: cfg.aws.access_key_id.clone().unwrap_or_default(),
        aws_secret_access_key: cfg.aws.secret_access_key.clone().unwrap_or_default(),
        ssh_default_key: cfg.ssh.default_key.clone(),
        ssh_hosts_file: cfg.ssh.hosts_file_path.clone().unwrap_or_default(),
        default_profile: cfg
            .profiles
            .iter()
            .find(|p| p.is_default)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "default".to_string()),
        import_path: cfg.import_path.clone().unwrap_or_default(),
        key_names: cfg.keys.iter().map(|k| k.name.clone()).collect(),
        key_paths: cfg.keys.iter().map(|k| k.path.to_string_lossy().to_string()).collect(),
    }
}

/// Default PEM key from config (for Enter on host list).
pub fn config_to_pem_default(cfg: &Config) -> PemKey {
    PemKey {
        name: "default".to_string(),
        path: PathBuf::from(cfg.ssh.default_key.as_str()),
    }
}

/// All PEM keys (default + named) for key picker modal.
pub fn config_to_pem_list(cfg: &Config) -> Vec<PemKey> {
    let mut keys = vec![PemKey {
        name: "default".to_string(),
        path: PathBuf::from(cfg.ssh.default_key.as_str()),
    }];
    keys.extend(cfg.keys.clone());
    keys
}

pub fn draft_to_config(d: &ConfigDraft) -> Config {
    let keys: Vec<PemKey> = d
        .key_names
        .iter()
        .zip(d.key_paths.iter())
        .map(|(name, path): (&String, &String)| PemKey {
            name: name.clone(),
            path: PathBuf::from(path),
        })
        .collect();
    Config {
        aws: AwsConfig {
            profile: d.aws_profile.clone(),
            regions: {
                let parsed: Vec<String> = d
                    .aws_region
                    .split(',')
                    .map(|r| r.trim().to_string())
                    .filter(|r| !r.is_empty())
                    .collect();
                if parsed.is_empty() {
                    vec!["us-east-1".to_string()]
                } else {
                    parsed
                }
            },
            access_key_id: if d.aws_access_key_id.is_empty() {
                None
            } else {
                Some(d.aws_access_key_id.clone())
            },
            secret_access_key: if d.aws_secret_access_key.is_empty() {
                None
            } else {
                Some(d.aws_secret_access_key.clone())
            },
        },
        ssh: SshConfig {
            default_key: d.ssh_default_key.clone(),
            hosts_file_path: if d.ssh_hosts_file.is_empty() {
                None
            } else {
                Some(d.ssh_hosts_file.clone())
            },
        },
        keys,
        profiles: {
            let default_name = if d.default_profile.is_empty() {
                "default".to_string()
            } else {
                d.default_profile.clone()
            };
            vec![SshProfile {
                name: default_name.clone(),
                username: "ubuntu".to_string(),
                port: 22,
                key_path: PathBuf::from(d.ssh_default_key.as_str()),
                is_default: true,
            }]
        },
        hosts: vec![],
        import_path: if d.import_path.is_empty() {
            None
        } else {
            Some(d.import_path.clone())
        },
    }
}

pub fn add_managed_host(host: ManagedHost) -> Result<()> {
    let mut cfg = load_config()?;
    cfg.hosts.push(host);
    save_config(&cfg)
}

pub fn delete_managed_host(name_or_address: &str) -> Result<()> {
    let mut cfg = load_config()?;
    cfg.hosts
        .retain(|h| h.name != name_or_address && h.address != name_or_address);
    save_config(&cfg)
}

pub fn add_profile(profile: SshProfile) -> Result<()> {
    let mut cfg = load_config()?;
    if profile.is_default {
        for p in &mut cfg.profiles {
            p.is_default = false;
        }
    }
    cfg.profiles.push(profile);
    save_config(&cfg)
}

pub fn default_profile(cfg: &Config) -> Option<SshProfile> {
    cfg.profiles
        .iter()
        .find(|p| p.is_default)
        .cloned()
        .or_else(|| cfg.profiles.first().cloned())
}

pub fn import_hosts_from_ssh_config(path: Option<&str>) -> Result<usize> {
    let path = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        dirs::home_dir()
            .context("no home dir")?
            .join(".ssh")
            .join("config")
    };
    let content = fs::read_to_string(&path).with_context(|| format!("read {:?}", path))?;

    let mut current_aliases: Vec<String> = Vec::new();
    let mut current_host_name: Option<String> = None;
    let mut current_user: Option<String> = None;
    let mut current_port: Option<u16> = None;
    let mut current_key: Option<String> = None;
    let mut out: Vec<ManagedHost> = Vec::new();

    let flush = |aliases: &Vec<String>,
                 host_name: &Option<String>,
                 user: &Option<String>,
                 port: &Option<u16>,
                 key: &Option<String>,
                 out: &mut Vec<ManagedHost>| {
        if let Some(addr) = host_name {
            for alias in aliases {
                if alias == "*" {
                    continue;
                }
                out.push(ManagedHost {
                    name: alias.clone(),
                    address: addr.clone(),
                    username: user.clone().unwrap_or_else(|| "ubuntu".to_string()),
                    port: port.unwrap_or(22),
                    key_path: key.as_ref().map(PathBuf::from),
                });
            }
        }
    };

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("host ") {
            flush(
                &current_aliases,
                &current_host_name,
                &current_user,
                &current_port,
                &current_key,
                &mut out,
            );
            current_aliases = line[5..]
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            current_host_name = None;
            current_user = None;
            current_port = None;
            current_key = None;
            continue;
        }
        if lower.starts_with("hostname ") {
            current_host_name = Some(line[9..].trim().to_string());
        } else if lower.starts_with("user ") {
            current_user = Some(line[5..].trim().to_string());
        } else if lower.starts_with("port ") {
            current_port = line[5..].trim().parse::<u16>().ok();
        } else if lower.starts_with("identityfile ") {
            current_key = Some(line[13..].trim().to_string());
        }
    }
    flush(
        &current_aliases,
        &current_host_name,
        &current_user,
        &current_port,
        &current_key,
        &mut out,
    );

    let mut cfg = load_config()?;
    cfg.hosts.extend(out);
    cfg.import_path = Some(path.to_string_lossy().to_string());
    let imported = cfg.hosts.len();
    save_config(&cfg)?;
    Ok(imported)
}
