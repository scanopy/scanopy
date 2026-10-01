//! SSH credential types for discovery dispatch.
//!
//! Both SSH transports (password and private key) reach the same `sshd` with the same user and run
//! the same script; only the auth material differs. So they collapse to one wire payload carrying
//! an [`SshAuth`] discriminator, the same way the two UniFi transports collapse to one
//! `UnifiQueryCredential`.

use crate::server::credentials::r#impl::mapping::{
    BannerField, BannerFieldValue, ResolvableSecret,
};
use serde::{Deserialize, Serialize};

pub const DEFAULT_SSH_PORT: u16 = 22;

/// How long the script may run before the daemon closes the channel.
pub const DEFAULT_SSH_TIMEOUT_SECONDS: u32 = 60;

pub fn default_ssh_port() -> u16 {
    DEFAULT_SSH_PORT
}

pub fn default_ssh_timeout_seconds() -> u32 {
    DEFAULT_SSH_TIMEOUT_SECONDS
}

/// How the daemon authenticates to `sshd`.
///
/// An enum rather than optional fields, so a payload with both a password and a key (or neither)
/// cannot be represented on the wire.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(tag = "mode")]
pub enum SshAuth {
    Password {
        password: ResolvableSecret,
    },
    PrivateKey {
        private_key: ResolvableSecret,
        #[serde(default)]
        passphrase: Option<ResolvableSecret>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct SshQueryCredential {
    pub port: u16,
    pub username: String,
    pub auth: SshAuth,
    /// Shell script run in one `exec` channel. Its stdout must be one JSON object of
    /// `SshScriptField` keys.
    pub script: String,
    pub timeout_seconds: u32,
    /// Require this host key (OpenSSH `SHA256:…` fingerprint). `None` ⇒ the daemon pins the
    /// first key it sees per address and port.
    #[serde(default)]
    pub host_key_fingerprint: Option<String>,
}

impl SshQueryCredential {
    pub fn banner_lines(&self) -> Vec<BannerField> {
        let mut lines = vec![
            BannerField {
                label: "Port",
                value: BannerFieldValue::Plain(self.port.to_string()),
            },
            BannerField {
                label: "Username",
                value: BannerFieldValue::Plain(self.username.clone()),
            },
        ];
        match &self.auth {
            SshAuth::Password { password } => lines.push(BannerField {
                label: "Password",
                value: password.banner_value(),
            }),
            SshAuth::PrivateKey { private_key, .. } => lines.push(BannerField {
                label: "Private key",
                value: private_key.banner_value(),
            }),
        }
        lines.push(BannerField {
            label: "Script",
            value: BannerFieldValue::InlineSummary(self.script.len()),
        });
        lines
    }
}
