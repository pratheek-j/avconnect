//! SFTP connectivity via russh and russh-sftp.
//!
//! Spawns a background worker thread that maintains an SSH+SFTP session,
//! processes directory listing requests, and sends results back through
//! the application message channel.

use std::path::PathBuf;

use anyhow::{Context, Result};
use russh::keys::key;
use russh::keys::load_secret_key;
use russh_sftp::client::SftpSession;
use std::sync::Arc;

use crate::message::AppMessage;
use crate::state::{FileKind, FileNode, Host, SshProfile};

struct SshHandler;

#[async_trait::async_trait]
impl russh::client::Handler for SshHandler {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // TODO: verify against known_hosts for production use
        Ok(true)
    }
}

fn expand_path(p: &PathBuf) -> PathBuf {
    let s = p.to_string_lossy();
    shellexpand::full(s.as_ref())
        .map(|expanded| PathBuf::from(expanded.as_ref()))
        .unwrap_or_else(|_| p.clone())
}

/// Spawn a background thread that connects SSH+SFTP and listens for commands.
/// Results flow back through `app_tx` as `AppMessage` variants.
pub fn spawn_sftp_worker(
    host: &Host,
    profile: &SshProfile,
    app_tx: std::sync::mpsc::Sender<AppMessage>,
) {
    let host = host.clone();
    let profile = profile.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            match connect_sftp(&host, &profile).await {
                Ok(sftp) => {
                    let home: String = sftp
                        .canonicalize(".")
                        .await
                        .unwrap_or_else(|_| "/".to_string());
                    let entries = list_dir(&sftp, &home).await.unwrap_or_default();
                    let _ = app_tx.send(AppMessage::SftpConnected {
                        host: host.clone(),
                        profile: profile.clone(),
                        root_entries: entries,
                    });
                }
                Err(e) => {
                    let _ = app_tx.send(AppMessage::HostsLoadFailed(format!(
                        "SFTP connection failed: {}",
                        e
                    )));
                }
            }
        });
    });
}

async fn connect_sftp(host: &Host, profile: &SshProfile) -> Result<SftpSession> {
    let key_path = expand_path(&profile.key_path);
    let key_pair = load_secret_key(&key_path, None).context("failed to load PEM key")?;

    let config = Arc::new(russh::client::Config::default());
    let mut session: russh::client::Handle<SshHandler> =
        russh::client::connect(config, (host.address.as_str(), profile.port), SshHandler)
            .await
            .context("SSH connect failed")?;

    session
        .authenticate_publickey(&profile.username, Arc::new(key_pair))
        .await
        .context("SSH publickey auth failed")?;

    let channel = session
        .channel_open_session()
        .await
        .context("open session channel")?;
    channel
        .request_subsystem(true, "sftp")
        .await
        .context("request sftp subsystem")?;
    let sftp = SftpSession::new(channel.into_stream())
        .await
        .context("sftp session init")?;

    Ok(sftp)
}

async fn list_dir(sftp: &SftpSession, path: &str) -> Result<Vec<FileNode>> {
    let entries = sftp.read_dir(path).await.context("read_dir")?;
    let mut nodes = Vec::new();
    for entry in entries {
        let name = entry.file_name();
        if name == "." || name == ".." {
            continue;
        }
        let full_path = format!("{}/{}", path.trim_end_matches('/'), name);
        let is_dir = entry.file_type().is_dir();
        let size = if is_dir {
            None
        } else {
            Some(entry.metadata().len())
        };
        nodes.push(FileNode {
            name,
            path: PathBuf::from(&full_path),
            kind: if is_dir { FileKind::Dir } else { FileKind::File },
            size,
        });
    }
    nodes.sort_by(|a, b| {
        let da = matches!(a.kind, FileKind::Dir);
        let db = matches!(b.kind, FileKind::Dir);
        db.cmp(&da).then(a.name.cmp(&b.name))
    });
    Ok(nodes)
}
