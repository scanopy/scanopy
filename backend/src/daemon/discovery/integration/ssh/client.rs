//! The SSH session in three steps: a handshake that reads the host key, authentication, and one
//! `exec` channel.
//!
//! The steps are separate so the caller decides whether to trust the key between the first two:
//! nothing is sent to the host until [`Handshake::authenticate`], and the key it checked is the key
//! of the session it authenticates on, because both happen on one connection.

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

/// Accepts the host key and records its fingerprint, for the caller to judge before
/// authenticating.
struct KeyRecorder {
    seen: Arc<Mutex<Option<String>>>,
}

impl client::Handler for KeyRecorder {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        key: &russh::keys::PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let fingerprint = key.public_key().fingerprint(HashAlg::Sha256).to_string();
        if let Ok(mut seen) = self.seen.lock() {
            *seen = Some(fingerprint);
        }
        Ok(true)
    }
}

/// No inactivity timeout: a script that prints nothing until it finishes is silent for as long as
/// it runs, and the credential's own timeout bounds that. Keepalives detect a host that went away.
fn config() -> Arc<client::Config> {
    Arc::new(client::Config {
        inactivity_timeout: None,
        keepalive_interval: Some(Duration::from_secs(15)),
        keepalive_max: 3,
        ..Default::default()
    })
}

#[derive(Debug)]
pub enum SessionError {
    Unreachable(String),
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
        }
    }
}

/// A completed handshake. Nothing has been sent to the host yet.
pub struct Handshake {
    handle: Handle<KeyRecorder>,
    fingerprint: String,
}

/// An authenticated session, ready to run one script.
pub struct Session {
    handle: Handle<KeyRecorder>,
}

/// Complete a handshake and read the host key's `SHA256:` fingerprint.
pub async fn handshake(ip: IpAddr, port: u16) -> Result<Handshake, SessionError> {
    let seen = Arc::new(Mutex::new(None));
    let handler = KeyRecorder { seen: seen.clone() };
    let handle = tokio::time::timeout(
        CONNECT_TIMEOUT,
        client::connect(config(), (ip, port), handler),
    )
    .await
    .map_err(|_| {
        SessionError::Unreachable(format!(
            "no SSH handshake within {}s",
            CONNECT_TIMEOUT.as_secs()
        ))
    })?
    .map_err(|e| SessionError::Unreachable(e.to_string()))?;
    let fingerprint = seen.lock().ok().and_then(|s| s.clone()).unwrap_or_default();
    Ok(Handshake {
        handle,
        fingerprint,
    })
}

impl Handshake {
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// Log in. Call only once the fingerprint is trusted: this sends the password or key signature.
    pub async fn authenticate(
        mut self,
        username: &str,
        auth: &SshAuth,
    ) -> Result<Session, SessionError> {
        let resolve = |secret: &crate::server::credentials::r#impl::mapping::ResolvableSecret,
                       field: &str| {
            secret
                .resolve(field, "SSH")
                .map(|s| s.expose_secret().to_string())
                .map_err(|e| SessionError::Malformed(e.to_string()))
        };
        let handle = &mut self.handle;
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
                    .authenticate_publickey(
                        username,
                        PrivateKeyWithHashAlg::new(Arc::new(key), hash),
                    )
                    .await
                    .map_err(|e| SessionError::Unreachable(e.to_string()))?
            }
        };
        if !authenticated.success() {
            let _ = self
                .handle
                .disconnect(Disconnect::ByApplication, "", "English")
                .await;
            return Err(SessionError::AuthenticationFailed(format!(
                "the host refused {} for user {username}",
                match auth {
                    SshAuth::Password { .. } => "the password",
                    SshAuth::PrivateKey { .. } => "the private key",
                }
            )));
        }
        Ok(Session {
            handle: self.handle,
        })
    }
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

impl Session {
    /// Run `script` in one exec channel, then close the session.
    pub async fn run_script(
        self,
        script: &str,
        timeout: Duration,
    ) -> Result<ScriptResult, SessionError> {
        let started = Instant::now();
        let mut channel = self
            .handle
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
        let _ = self
            .handle
            .disconnect(Disconnect::ByApplication, "", "English")
            .await;
        Ok(result)
    }
}
