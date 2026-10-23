use std::process::Stdio;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::process::{Child, Command};

use crate::profile::SshConfig;

const READY_TIMEOUT: Duration = Duration::from_secs(20);

/// Local port forward through the system `ssh` binary, so `~/.ssh/config`, agents, keys and
/// `ProxyJump` keep working. Killed when dropped.
pub struct Tunnel {
    _child: Child,
    pub local_port: u16,
}

impl Tunnel {
    pub async fn open(ssh: &SshConfig, target_host: &str, target_port: u16) -> Result<Self> {
        let local_port = free_local_port()?;
        let askpass = if ssh.secret.is_empty() { None } else { Some(Askpass::create(local_port)?) };
        let mut command = Command::new("ssh");
        command
            .args(forward_args(ssh, local_port, target_host, target_port))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if let Some(askpass) = &askpass {
            command
                .env("SSH_ASKPASS", &askpass.path)
                .env("SSH_ASKPASS_REQUIRE", "force")
                .env(SECRET_ENV, &ssh.secret);
        }
        let mut child = command.spawn().context("failed to start `ssh` (is OpenSSH installed?)")?;

        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait()? {
                let mut stderr = String::new();
                if let Some(mut pipe) = child.stderr.take() {
                    pipe.read_to_string(&mut stderr).await.ok();
                }
                bail!("ssh exited ({status}): {}", stderr.trim());
            }
            if TcpStream::connect(("127.0.0.1", local_port)).await.is_ok() {
                return Ok(Self { _child: child, local_port });
            }
            if started.elapsed() > READY_TIMEOUT {
                bail!("timed out waiting for the SSH tunnel to come up");
            }
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
    }
}

const SECRET_ENV: &str = "DBVIEWER_SSH_SECRET";

/// Throwaway helper that ssh runs to read the secret from the environment; removed once ssh is done authenticating.
struct Askpass {
    path: std::path::PathBuf,
}

impl Askpass {
    fn create(tag: u16) -> Result<Self> {
        use std::io::Write;
        let path = std::env::temp_dir().join(format!("dbviewer-askpass-{}-{tag}", std::process::id()));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o700);
        let mut file = options.open(&path).context("creating ssh askpass helper")?;
        writeln!(file, "#!/bin/sh\nprintf '%s\\n' \"${SECRET_ENV}\"")?;
        Ok(Self { path })
    }
}

impl Drop for Askpass {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn free_local_port() -> Result<u16> {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0))?;
    Ok(listener.local_addr()?.port())
}

fn forward_args(ssh: &SshConfig, local_port: u16, target_host: &str, target_port: u16) -> Vec<String> {
    let mut args: Vec<String> = [
        "-N",
        "-T",
        "-o", "ExitOnForwardFailure=yes",
        "-o", "ServerAliveInterval=30",
        "-o", "StrictHostKeyChecking=accept-new",
    ]
    .map(String::from)
    .into();
    if ssh.secret.is_empty() {
        args.extend(["-o".into(), "BatchMode=yes".into()]);
    }
    args.push("-L".into());
    args.push(format!("127.0.0.1:{local_port}:{target_host}:{target_port}"));
    if let Some(port) = ssh.port {
        args.push("-p".into());
        args.push(port.to_string());
    }
    if !ssh.identity_file.is_empty() {
        args.push("-i".into());
        args.push(ssh.identity_file.clone());
    }
    args.push(if ssh.user.is_empty() { ssh.host.clone() } else { format!("{}@{}", ssh.user, ssh.host) });
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn askpass_prints_the_secret() {
        let askpass = Askpass::create(1).unwrap();
        let output = std::process::Command::new(&askpass.path).env(SECRET_ENV, "p@ss 'w\"rd").output().unwrap();
        assert_eq!(String::from_utf8_lossy(&output.stdout), "p@ss 'w\"rd\n");
    }

    #[test]
    fn builds_forward_args() {
        let ssh = SshConfig { host: "bastion".into(), port: Some(2222), user: "me".into(), identity_file: "/k".into(), secret: String::new() };
        let args = forward_args(&ssh, 5555, "db.internal", 5432);
        assert!(args.windows(2).any(|w| w == ["-L", "127.0.0.1:5555:db.internal:5432"]));
        assert!(args.windows(2).any(|w| w == ["-p", "2222"]));
        assert!(args.windows(2).any(|w| w == ["-i", "/k"]));
        assert_eq!(args.last().unwrap(), "me@bastion");
    }
}
