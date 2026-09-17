use std::borrow::Cow;
use std::sync::Arc;

use async_trait::async_trait;
use russh::client::{self};
use russh::keys::{load_secret_key, PrivateKeyWithHashAlg};
use russh::{ChannelMsg, Disconnect, Preferred};
use russh_sftp::client::SftpSession;
use thiserror::Error;

use crate::config::{AuthConfig, Config};

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("SSH error: {0}")]
    Ssh(#[from] russh::Error),
    #[error("SFTP error: {0}")]
    Sftp(#[from] russh_sftp::client::error::Error),
    #[error("authentication to {0}@{1} failed")]
    AuthFailed(String, String),
    #[error("failed to load private key {path}: {source}")]
    KeyLoad {
        path: std::path::PathBuf,
        #[source]
        source: russh::keys::Error,
    },
    #[error("command `{command}` failed (exit status {exit_status}): {stderr}")]
    CommandFailed {
        command: String,
        exit_status: u32,
        stderr: String,
    },
    #[error("xochitl did not report `active` within the retry budget after restart")]
    RestartTimedOut,
    #[error("{0}")]
    Simulated(String),
}

pub struct ExecOutput {
    pub exit_status: u32,
    pub stdout: String,
    pub stderr: String,
}

/// The seam production code depends on instead of talking to `russh`/
/// `russh-sftp` directly, so tests can substitute an in-memory fake instead
/// of touching the real tablet.
#[async_trait]
pub trait SftpTransport: Send + Sync {
    async fn write(&self, path: &str, data: &[u8]) -> Result<(), TransportError>;
    async fn mkdir(&self, path: &str) -> Result<(), TransportError>;
    async fn rename(&self, from: &str, to: &str) -> Result<(), TransportError>;
    async fn remove_file(&self, path: &str) -> Result<(), TransportError>;
    async fn remove_dir(&self, path: &str) -> Result<(), TransportError>;
    async fn read(&self, path: &str) -> Result<Vec<u8>, TransportError>;
    async fn list_names(&self, path: &str) -> Result<Vec<String>, TransportError>;
    async fn file_size(&self, path: &str) -> Result<u64, TransportError>;
    async fn exec(&self, command: &str) -> Result<ExecOutput, TransportError>;
}

struct ClientHandler;

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    // Known simplification: accepts any server host key, matching russh's
    // own examples. The tablet is reached over a direct USB point-to-point
    // link rather than a shared network, which limits (without eliminating)
    // MITM exposure. Host-key pinning would be a reasonable future
    // hardening item but wasn't in scope for this pass.
    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

fn cipher_names(ciphers: &[String]) -> Vec<russh::cipher::Name> {
    ciphers
        .iter()
        .filter_map(|name| match name.as_str() {
            "aes128-gcm@openssh.com" => Some(russh::cipher::AES_128_GCM),
            "aes256-gcm@openssh.com" => Some(russh::cipher::AES_256_GCM),
            "chacha20-poly1305@openssh.com" => Some(russh::cipher::CHACHA20_POLY1305),
            "aes128-ctr" => Some(russh::cipher::AES_128_CTR),
            "aes256-ctr" => Some(russh::cipher::AES_256_CTR),
            _ => None,
        })
        .collect()
}

/// The real, `russh`-backed transport used against an actual tablet. Holds
/// both the session handle (for `exec`) and the SFTP subsystem session
/// (for file operations) opened over the same authenticated connection.
pub struct RemoteTransport {
    handle: client::Handle<ClientHandler>,
    sftp: SftpSession,
}

impl RemoteTransport {
    pub async fn connect(config: &Config) -> Result<Self, TransportError> {
        let mut ssh_config = client::Config::default();
        let preferred_ciphers = cipher_names(&config.ciphers);
        if !preferred_ciphers.is_empty() {
            ssh_config.preferred = Preferred {
                cipher: Cow::Owned(preferred_ciphers),
                ..Default::default()
            };
        }

        let mut handle = client::connect(
            Arc::new(ssh_config),
            (config.host.as_str(), config.port),
            ClientHandler,
        )
        .await?;

        let authenticated = match &config.auth {
            AuthConfig::Password { password } => {
                handle
                    .authenticate_password(&config.username, password)
                    .await?
                    .success()
            }
            AuthConfig::Key {
                private_key_path,
                passphrase,
            } => {
                let key_pair = load_secret_key(private_key_path, passphrase.as_deref())
                    .map_err(|e| TransportError::KeyLoad {
                        path: private_key_path.clone(),
                        source: e,
                    })?;
                let hash_alg = handle.best_supported_rsa_hash().await?.flatten();
                handle
                    .authenticate_publickey(
                        &config.username,
                        PrivateKeyWithHashAlg::new(Arc::new(key_pair), hash_alg),
                    )
                    .await?
                    .success()
            }
        };

        if !authenticated {
            return Err(TransportError::AuthFailed(
                config.username.clone(),
                config.host.clone(),
            ));
        }

        let sftp_channel = handle.channel_open_session().await?;
        sftp_channel.request_subsystem(true, "sftp").await?;
        let sftp = SftpSession::new(sftp_channel.into_stream()).await?;

        Ok(Self { handle, sftp })
    }

    pub async fn disconnect(&mut self) -> Result<(), TransportError> {
        self.handle
            .disconnect(Disconnect::ByApplication, "", "English")
            .await?;
        Ok(())
    }
}

#[async_trait]
impl SftpTransport for RemoteTransport {
    async fn write(&self, path: &str, data: &[u8]) -> Result<(), TransportError> {
        self.sftp.write(path, data).await?;
        Ok(())
    }

    async fn mkdir(&self, path: &str) -> Result<(), TransportError> {
        self.sftp.create_dir(path).await?;
        Ok(())
    }

    async fn rename(&self, from: &str, to: &str) -> Result<(), TransportError> {
        self.sftp.rename(from, to).await?;
        Ok(())
    }

    async fn remove_file(&self, path: &str) -> Result<(), TransportError> {
        self.sftp.remove_file(path).await?;
        Ok(())
    }

    async fn remove_dir(&self, path: &str) -> Result<(), TransportError> {
        self.sftp.remove_dir(path).await?;
        Ok(())
    }

    async fn read(&self, path: &str) -> Result<Vec<u8>, TransportError> {
        Ok(self.sftp.read(path).await?)
    }

    async fn list_names(&self, path: &str) -> Result<Vec<String>, TransportError> {
        let entries = self.sftp.read_dir(path).await?;
        Ok(entries
            .into_iter()
            .map(|entry| entry.file_name())
            .collect())
    }

    async fn file_size(&self, path: &str) -> Result<u64, TransportError> {
        let metadata = self.sftp.metadata(path).await?;
        Ok(metadata.size.unwrap_or(0))
    }

    async fn exec(&self, command: &str) -> Result<ExecOutput, TransportError> {
        let mut channel = self.handle.channel_open_session().await?;
        channel.exec(true, command).await?;

        let mut exit_status = None;
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        while let Some(msg) = channel.wait().await {
            match msg {
                ChannelMsg::Data { data } => stdout.extend_from_slice(&data),
                ChannelMsg::ExtendedData { data, .. } => stderr.extend_from_slice(&data),
                ChannelMsg::ExitStatus { exit_status: code } => exit_status = Some(code),
                _ => {}
            }
        }

        Ok(ExecOutput {
            exit_status: exit_status.unwrap_or(u32::MAX),
            stdout: String::from_utf8_lossy(&stdout).to_string(),
            stderr: String::from_utf8_lossy(&stderr).to_string(),
        })
    }
}
