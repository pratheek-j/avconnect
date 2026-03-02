//! Host registry — merges AWS-discovered EC2 instances with a static
//! hosts file, deduplicating by name and address.

use anyhow::Result;
use std::path::PathBuf;

use crate::services::aws;
use crate::state::{Host, HostSource};

/// Load config, fetch AWS hosts, load static file, merge. Uses config from ~/.avconnect/config.toml.
pub async fn load_hosts() -> Result<Vec<Host>> {
    let cfg = crate::services::load_config()?;
    let mut hosts = Vec::new();
    for region in &cfg.aws.regions {
        let region_hosts = aws::fetch_hosts(
            &cfg.aws.profile,
            region,
            cfg.aws.access_key_id.as_deref(),
            cfg.aws.secret_access_key.as_deref(),
        )
        .await
        .unwrap_or_default();
        for host in region_hosts {
            if !hosts
                .iter()
                .any(|x: &Host| x.name == host.name || x.address == host.address)
            {
                hosts.push(host);
            }
        }
    }

    if let Some(ref path) = cfg.ssh.hosts_file_path {
        let expanded: std::borrow::Cow<'_, str> =
            shellexpand::full(path).unwrap_or_else(|_| std::borrow::Cow::Owned(path.clone()));
        let path = PathBuf::from(expanded.as_ref());
        if path.exists() {
            if let Ok(static_hosts) = load_static_file(&path) {
                for h in static_hosts {
                    if !hosts.iter().any(|x| x.name == h.name || x.address == h.address) {
                        hosts.push(h);
                    }
                }
            }
        }
    }

    for h in &cfg.hosts {
        let mapped = Host {
            name: h.name.clone(),
            address: h.address.clone(),
            username: h.username.clone(),
            port: h.port,
            key_path: h.key_path.clone(),
            state: None,
            source: HostSource::Config,
        };
        hosts.push(mapped);
    }

    hosts.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(hosts)
}

/// Simple format: one host per line, "name address [username]". Lines starting with # are ignored.
fn load_static_file(path: &std::path::Path) -> Result<Vec<Host>> {
    let s = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    for line in s.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let name = parts[0].to_string();
            let address = parts[1].to_string();
            let username = parts.get(2).map(|s| (*s).to_string()).unwrap_or_else(|| "ubuntu".to_string());
            out.push(Host {
                name,
                address,
                username,
                port: 22,
                key_path: None,
                state: None,
                source: HostSource::Static,
            });
        }
    }
    Ok(out)
}
