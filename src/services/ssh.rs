//! SSH execution — replaces the process with ssh on Unix (exec),
//! or spawns and waits on Windows.

use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

use crate::state::{Host, SshProfile};

fn expand_path(p: &PathBuf) -> PathBuf {
    let s = p.to_string_lossy();
    shellexpand::full(s.as_ref())
        .map(|s| PathBuf::from(s.as_ref()))
        .unwrap_or_else(|_| p.clone())
}

pub fn connect(host: &Host, profile: &SshProfile) -> Result<()> {
    let path = expand_path(&profile.key_path);
    let path = path.to_string_lossy();
    let target = format!("{}@{}", profile.username, host.address);
    let mut cmd = Command::new("ssh");
    cmd.arg("-i")
        .arg(path.as_ref())
        .arg("-p")
        .arg(profile.port.to_string())
        .arg(&target);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = cmd.exec();
        Err(err.into())
    }

    #[cfg(not(unix))]
    {
        let status = cmd.status()?;
        if status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("ssh exited with {}", status))
        }
    }
}
