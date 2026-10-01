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
use utoipa::ToSchema;

use super::paths::{DaemonPath, HostPath, OsFamily};

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
    /// The scanned host's OS, which decides the shell the script runs in.
    #[serde(default)]
    pub target_os: OsFamily,
    /// What runs in the `exec` channel. Its stdout must be one JSON object of `SshScriptField`
    /// keys. A `DaemonFile` is read off the daemon's own disk, like every other credential file.
    pub script: ScriptSource,
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
            value: match &self.script {
                ScriptSource::HostFile { path } => {
                    BannerFieldValue::Plain(format!("{path} on the scanned host"))
                }
                ScriptSource::DaemonFile { path } => BannerFieldValue::Plain(path.to_string()),
                ScriptSource::Inline { value } => BannerFieldValue::InlineSummary(value.len()),
            },
        });
        lines
    }
}

/// Where an SSH credential's script comes from. One type for the stored credential and the wire:
/// both sides hold the same typed paths, so there is nothing to translate.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(tag = "mode")]
pub enum ScriptSource {
    /// An executable already on each scanned host. Nothing but the path is stored in Scanopy.
    #[schema(title = "HostFile")]
    HostFile { path: HostPath },
    /// A script file on the daemon's machine; the daemon reads it and runs its text.
    #[schema(title = "DaemonFile")]
    DaemonFile { path: DaemonPath },
    /// The script text, stored on the credential.
    #[schema(title = "Inline")]
    Inline { value: String },
}

impl Default for ScriptSource {
    /// The form opens on a file already on the scanned host.
    fn default() -> Self {
        Self::HostFile {
            path: String::new().into(),
        }
    }
}

impl ScriptSource {
    /// The command line sent in the exec channel to a host of `target_os`.
    ///
    /// Unix hosts run it in their login shell as-is. Windows hosts get PowerShell: script text as
    /// `-EncodedCommand` (base64 of UTF-16LE), which survives either default shell a Windows
    /// OpenSSH server may have, cmd.exe or PowerShell, multi-line and unquoted.
    pub fn command(&self, target_os: OsFamily) -> Result<String, anyhow::Error> {
        let text = match self {
            Self::HostFile { path } => return Ok(path.invocation(target_os)),
            Self::DaemonFile { path } => path.read("script", "SSH")?,
            Self::Inline { value } => value.clone(),
        };
        Ok(match target_os {
            OsFamily::Unix => text,
            OsFamily::Windows => format!(
                "powershell -NoProfile -NonInteractive -EncodedCommand {}",
                encode_powershell(&text)
            ),
        })
    }

    /// Read a daemon-side file into `Inline`, for `resolve_file_paths`, which reads every file a
    /// credential names up front.
    pub fn resolve_daemon_file(&self) -> Result<Self, anyhow::Error> {
        match self {
            Self::DaemonFile { path } => Ok(Self::Inline {
                value: path.read("script", "SSH")?,
            }),
            other => Ok(other.clone()),
        }
    }
}

/// PowerShell's `-EncodedCommand` form: base64 of the UTF-16LE bytes.
fn encode_powershell(script: &str) -> String {
    use base64ct::{Base64, Encoding};
    let utf16: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    Base64::encode_string(&utf16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64ct::{Base64, Encoding};

    fn decode_powershell(command: &str) -> String {
        let b64 = command.rsplit(' ').next().unwrap();
        let bytes = Base64::decode_vec(b64).unwrap();
        let units: Vec<u16> = bytes
            .chunks(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16(&units).unwrap()
    }

    #[test]
    fn a_windows_script_round_trips_through_encoded_command() {
        let script = "$h = hostname\n@{ hostname = $h; model = 'Ü \"x\"' } | ConvertTo-Json";
        let command = ScriptSource::Inline {
            value: script.into(),
        }
        .command(OsFamily::Windows)
        .unwrap();
        assert!(command.starts_with("powershell -NoProfile -NonInteractive -EncodedCommand "));
        assert_eq!(decode_powershell(&command), script);
    }

    #[test]
    fn a_unix_inline_script_is_sent_as_is() {
        let script = "#!/bin/sh\necho '{}'";
        let source = ScriptSource::Inline {
            value: script.into(),
        };
        assert_eq!(source.command(OsFamily::Unix).unwrap(), script);
    }

    #[test]
    fn a_host_file_runs_through_the_targets_shell() {
        let unix = ScriptSource::HostFile {
            path: "/usr/local/bin/inv".into(),
        };
        assert_eq!(
            unix.command(OsFamily::Unix).unwrap(),
            "'/usr/local/bin/inv'"
        );
        let windows = ScriptSource::HostFile {
            path: r"C:\Scanopy\inventory.ps1".into(),
        };
        assert!(
            windows
                .command(OsFamily::Windows)
                .unwrap()
                .ends_with(r#"-File "C:\Scanopy\inventory.ps1""#)
        );
    }
}
