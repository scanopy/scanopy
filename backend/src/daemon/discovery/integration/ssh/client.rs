//! The SSH session: handshake, host-key check, authentication, one `exec` channel.

use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use russh::client::{self, Handle};
use russh::keys::{HashAlg, PrivateKeyWithHashAlg, decode_secret_key};
use russh::{ChannelMsg, Disconnect};

use crate::server::credentials::r#impl::mapping::SshAuth;
use crate::server::credentials::r#impl::types::ssh_script::{
    MAX_SCRIPT_OUTPUT_BYTES, MAX_STDERR_EXCERPT_BYTES,
};

pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Accepts exactly one host key, or (with `None`) any key, and records the one it saw.
///
/// The probe passes `None` and only reads the fingerprint; it sends no credentials. Execute passes
/// the fingerprint it has decided to trust, so the key cannot change between that decision and the
/// password or key signature going out.
struct KeyCheck {
    require: Option<String>,
    seen: Arc<Mutex<Option<String>>>,
}

impl client::Handler for KeyCheck {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        key: &russh::keys::PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let fingerprint = key.public_key().fingerprint(HashAlg::Sha256).to_string();
        let accept = self.require.as_deref().is_none_or(|r| r == fingerprint);
        if let Ok(mut seen) = self.seen.lock() {
            *seen = Some(fingerprint);
        }
        Ok(accept)
    }
}

fn config() -> Arc<client::Config> {
    Arc::new(client::Config {
        inactivity_timeout: Some(Duration::from_secs(30)),
        ..Default::default()
    })
}

async fn connect(
    ip: IpAddr,
    port: u16,
    require: Option<String>,
) -> Result<(Handle<KeyCheck>, String), SessionError> {
    let seen = Arc::new(Mutex::new(None));
    let handler = KeyCheck {
        require: require.clone(),
        seen: seen.clone(),
    };
    let result = tokio::time::timeout(
        CONNECT_TIMEOUT,
        client::connect(config(), (ip, port), handler),
    )
    .await;
    let observed = seen.lock().ok().and_then(|s| s.clone());
    match result {
        Err(_) => Err(SessionError::Unreachable(format!(
            "no SSH handshake within {}s",
            CONNECT_TIMEOUT.as_secs()
        ))),
        Ok(Ok(handle)) => Ok((handle, observed.unwrap_or_default())),
        Ok(Err(e)) => match (require, observed) {
            (Some(required), Some(observed)) if required != observed => {
                Err(SessionError::HostKeyMismatch { observed })
            }
            _ => Err(SessionError::Unreachable(e.to_string())),
        },
    }
}

#[derive(Debug)]
pub enum SessionError {
    Unreachable(String),
    HostKeyMismatch {
        observed: String,
    },
    AuthenticationFailed(String),
    /// The credential could not be used: an unreadable file, a key that does not parse.
    Malformed(String),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable(m) | Self::AuthenticationFailed(m) | Self::Malformed(m) => {
                f.write_str(m)
            }
            Self::HostKeyMismatch { observed } => {
                write!(
                    f,
                    "the host presented key {observed}, which is not the trusted key"
                )
            }
        }
    }
}

/// Complete a handshake and return the host key's `SHA256:` fingerprint. Sends no credentials.
pub async fn read_host_key(ip: IpAddr, port: u16) -> Result<String, SessionError> {
    let (handle, fingerprint) = connect(ip, port, None).await?;
    let _ = handle
        .disconnect(Disconnect::ByApplication, "", "English")
        .await;
    Ok(fingerprint)
}

/// What one script run produced.
pub struct ScriptResult {
    pub exit_code: Option<u32>,
    pub stdout: Vec<u8>,
    /// Stdout went past [`MAX_SCRIPT_OUTPUT_BYTES`]; `stdout` holds the first part only.
    pub stdout_overflowed: bool,
    /// The end of stderr.
    pub stderr_tail: Vec<u8>,
    pub timed_out: bool,
    pub elapsed: Duration,
}

/// Authenticate to a host whose key is `trusted`, then run `script` in one exec channel.
pub async fn run_script(
    ip: IpAddr,
    port: u16,
    trusted: &str,
    username: &str,
    auth: &SshAuth,
    script: &str,
    timeout: Duration,
) -> Result<ScriptResult, SessionError> {
    let (mut handle, _) = connect(ip, port, Some(trusted.to_string())).await?;

    let resolve = |secret: &crate::server::credentials::r#impl::mapping::ResolvableSecret,
                   field: &str| {
        secret
            .resolve(field, "SSH")
            .map(|s| s.expose_secret().to_string())
            .map_err(|e| SessionError::Malformed(e.to_string()))
    };
    let authenticated = match auth {
        SshAuth::Password { password } => handle
            .authenticate_password(username, resolve(password, "password")?)
            .await
            .map_err(|e| SessionError::Unreachable(e.to_string()))?,
        SshAuth::PrivateKey {
            private_key,
            passphrase,
        } => {
            let passphrase = passphrase
                .as_ref()
                .map(|p| resolve(p, "passphrase"))
                .transpose()?;
            let key =
                decode_secret_key(&resolve(private_key, "private_key")?, passphrase.as_deref())
                    .map_err(|e| {
                        SessionError::Malformed(format!("could not read the private key: {e}"))
                    })?;
            let hash = handle
                .best_supported_rsa_hash()
                .await
                .map_err(|e| SessionError::Unreachable(e.to_string()))?
                .flatten();
            handle
                .authenticate_publickey(username, PrivateKeyWithHashAlg::new(Arc::new(key), hash))
                .await
                .map_err(|e| SessionError::Unreachable(e.to_string()))?
        }
    };
    if !authenticated.success() {
        return Err(SessionError::AuthenticationFailed(format!(
            "the host refused {} for user {username}",
            match auth {
                SshAuth::Password { .. } => "the password",
                SshAuth::PrivateKey { .. } => "the private key",
            }
        )));
    }

    let started = Instant::now();
    let mut channel = handle
        .channel_open_session()
        .await
        .map_err(|e| SessionError::Unreachable(e.to_string()))?;
    channel
        .exec(true, script)
        .await
        .map_err(|e| SessionError::Unreachable(e.to_string()))?;

    let mut result = ScriptResult {
        exit_code: None,
        stdout: Vec::new(),
        stdout_overflowed: false,
        stderr_tail: Vec::new(),
        timed_out: false,
        elapsed: Duration::ZERO,
    };
    let read = async {
        while let Some(msg) = channel.wait().await {
            match msg {
                ChannelMsg::Data { data } => {
                    let room = MAX_SCRIPT_OUTPUT_BYTES.saturating_sub(result.stdout.len());
                    if data.len() > room {
                        result.stdout.extend_from_slice(&data[..room]);
                        result.stdout_overflowed = true;
                        // Nothing past the cap is used; stop the script rather than drain it.
                        break;
                    }
                    result.stdout.extend_from_slice(&data);
                }
                ChannelMsg::ExtendedData { data, ext: 1 } => {
                    result.stderr_tail.extend_from_slice(&data);
                    let excess = result
                        .stderr_tail
                        .len()
                        .saturating_sub(MAX_STDERR_EXCERPT_BYTES);
                    result.stderr_tail.drain(..excess);
                }
                ChannelMsg::ExitStatus { exit_status } => result.exit_code = Some(exit_status),
                _ => {}
            }
        }
    };
    if tokio::time::timeout(timeout, read).await.is_err() {
        result.timed_out = true;
    }
    result.elapsed = started.elapsed();

    let _ = channel.close().await;
    let _ = handle
        .disconnect(Disconnect::ByApplication, "", "English")
        .await;
    Ok(result)
}
