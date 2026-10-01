//! File paths a credential can name, typed by whose disk they are on, and validated for the OS
//! the credential declares for that machine.
//!
//! Two machines can hold a credential's files, and they need not share an OS: a Linux daemon
//! scanning Windows servers is ordinary. So there are two declarations, the credential's
//! `daemon_os` and an SSH credential's `target_os`, and each path type is checked against the one
//! for its machine:
//!
//! - [`DaemonPath`] and [`DaemonSocket`] are read on the daemon (`daemon_os`).
//! - [`HostPath`] is executed on the scanned host (`target_os`) and never read by the daemon.
//!
//! Parsing is `typed-path`'s, which reads Unix and Windows paths the same way on any host, so a
//! Linux server can tell an absolute Windows path from a relative one. `std::path` cannot: it only
//! knows the OS it was built for.
//!
//! Every type serializes as a bare string, so stored credentials and the daemon wire format are
//! unchanged. Deserialization is lenient (a stored row with an old path still loads); `validate`
//! is what the API applies on create and update.

use std::fmt;
use std::path::Path;

use anyhow::Error;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, IntoStaticStr};
use typed_path::{Utf8UnixPath, Utf8WindowsComponent, Utf8WindowsPath, Utf8WindowsPrefix};
use utoipa::ToSchema;

use crate::server::credentials::r#impl::mapping::UnresolvableCredential;
use crate::server::shared::types::field_definition::SelectOption;

/// The OS family of a machine a credential's paths live on. Paths, sockets and the shell a script
/// runs in all follow from it.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Default,
    Serialize,
    Deserialize,
    ToSchema,
    Display,
    EnumIter,
    IntoStaticStr,
    strum::VariantNames,
)]
pub enum OsFamily {
    /// Linux, macOS, BSD. Every credential path before this existed was one of these.
    #[default]
    Unix,
    Windows,
}

impl OsFamily {
    /// The OS this binary runs on.
    pub fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else {
            Self::Unix
        }
    }

    pub const OPTIONS: &'static [SelectOption] = &[
        SelectOption {
            value: "Unix",
            label: "Linux, macOS, BSD",
        },
        SelectOption {
            value: "Windows",
            label: "Windows",
        },
    ];

    fn is_absolute(self, p: &str) -> bool {
        match self {
            Self::Unix => Utf8UnixPath::new(p).is_absolute(),
            Self::Windows => Utf8WindowsPath::new(p).is_absolute(),
        }
    }

    /// What the credential form shows for this OS: its label, and the placeholder for a file on the
    /// daemon. Emitted as `os-families.json`.
    pub fn metadata(self) -> OsFamilyMetadata {
        OsFamilyMetadata {
            id: self.into(),
            name: Self::OPTIONS
                .iter()
                .find(|o| o.value == <&'static str>::from(self))
                .map(|o| o.label)
                .unwrap_or_default(),
            example_file_path: self.example(),
        }
    }

    /// An example absolute path for error messages.
    fn example(self) -> &'static str {
        match self {
            Self::Unix => "/etc/scanopy/key.pem",
            Self::Windows => r"C:\ProgramData\Scanopy\key.pem",
        }
    }
}

/// One row of `os-families.json`.
#[derive(Debug, Clone, Serialize)]
pub struct OsFamilyMetadata {
    pub id: &'static str,
    pub name: &'static str,
    pub example_file_path: &'static str,
}

/// Reject a path that is not absolute for `os`, with a message that says what would be.
fn require_absolute(p: &str, field: &str, os: OsFamily) -> Result<(), Error> {
    if os.is_absolute(p) {
        return Ok(());
    }
    let example = os.example();
    if p.starts_with('~') {
        crate::bail_validation!(
            "{field}: use an absolute path such as {example}. ~ is not expanded, and the daemon may run as another user."
        );
    }
    crate::bail_validation!("{field}: use an absolute {os} path, such as {example}.");
}

/// A path a credential names, with the machine it is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialPath<'a> {
    /// A file on the daemon, checked against the credential's `daemon_os`.
    Daemon(&'a DaemonPath),
    /// A socket or named pipe on the daemon, checked against `daemon_os`.
    Socket(&'a DaemonSocket),
    /// A file on the scanned host, checked against the SSH credential's `target_os`.
    Host(&'a HostPath, OsFamily),
}

/// A file on the daemon's own machine, read by the daemon at scan time.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[schema(value_type = String)]
pub struct DaemonPath(String);

impl DaemonPath {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_path(&self) -> &Path {
        Path::new(self.0.trim())
    }

    pub fn is_blank(&self) -> bool {
        self.0.trim().is_empty()
    }

    pub fn validate(&self, field: &str, daemon_os: OsFamily) -> Result<(), Error> {
        require_absolute(self.0.trim(), field, daemon_os)
    }

    /// Read the file. A failure is an [`UnresolvableCredential`], which callers classify as a
    /// configuration fault rather than an unreachable device.
    pub fn read(&self, field: &str, label: &str) -> Result<String, anyhow::Error> {
        std::fs::read_to_string(self.as_path()).map_err(|source| {
            anyhow::Error::new(UnresolvableCredential {
                field: field.to_string(),
                path: self.0.clone(),
                label: label.to_string(),
                source,
            })
        })
    }

    pub fn exists(&self) -> bool {
        self.as_path().exists()
    }
}

/// A container runtime's API socket on the daemon: a Unix socket, or a Windows named pipe.
///
/// Handed to bollard's `connect_with_socket`, which strips a `unix://` or `npipe://` scheme and
/// opens a Unix socket or a named pipe for the OS it runs on.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[schema(value_type = String)]
pub struct DaemonSocket(String);

impl DaemonSocket {
    pub fn as_str(&self) -> &str {
        self.0.trim()
    }

    pub fn is_blank(&self) -> bool {
        self.0.trim().is_empty()
    }

    /// `/…` or `unix:///…` on Unix; `\\.\pipe\…` or `npipe:////./pipe/…` on Windows.
    pub fn validate(&self, field: &str, daemon_os: OsFamily) -> Result<(), Error> {
        let raw = self.0.trim();
        let ok = match daemon_os {
            OsFamily::Unix => {
                let p = raw.strip_prefix("unix://").unwrap_or(raw);
                OsFamily::Unix.is_absolute(p)
            }
            OsFamily::Windows => {
                let p = raw.strip_prefix("npipe://").unwrap_or(raw);
                matches!(
                    Utf8WindowsPath::new(p).components().next(),
                    Some(Utf8WindowsComponent::Prefix(prefix))
                        if matches!(prefix.kind(), Utf8WindowsPrefix::DeviceNS(d) if d.eq_ignore_ascii_case("pipe"))
                )
            }
        };
        if ok {
            return Ok(());
        }
        match daemon_os {
            OsFamily::Unix => crate::bail_validation!(
                "{field}: use the socket's absolute path, such as /var/run/docker.sock."
            ),
            OsFamily::Windows => crate::bail_validation!(
                r"{field}: use a named pipe, such as \\.\pipe\docker_engine."
            ),
        }
    }
}

/// A file on the scanned host, executed there over SSH. Never read by the daemon.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[schema(value_type = String)]
pub struct HostPath(String);

impl HostPath {
    pub fn as_str(&self) -> &str {
        self.0.trim()
    }

    pub fn is_blank(&self) -> bool {
        self.0.trim().is_empty()
    }

    /// Absolute for the target's OS. A Windows path also cannot hold `"`: it is passed to
    /// PowerShell's `-File` inside double quotes.
    pub fn validate(&self, field: &str, target_os: OsFamily) -> Result<(), Error> {
        require_absolute(self.as_str(), field, target_os)?;
        if target_os == OsFamily::Windows && self.0.contains('"') {
            crate::bail_validation!("{field}: a Windows path cannot contain \".");
        }
        Ok(())
    }

    /// The command that runs this file on a host of `target_os`.
    pub fn invocation(&self, target_os: OsFamily) -> String {
        match target_os {
            // One POSIX shell word: single-quoted, each `'` written as `'\''`.
            OsFamily::Unix => format!("'{}'", self.as_str().replace('\'', r"'\''")),
            OsFamily::Windows => format!(
                "powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File \"{}\"",
                self.as_str()
            ),
        }
    }
}

macro_rules! string_newtype_conversions {
    ($($t:ty),*) => {$(
        impl From<String> for $t {
            fn from(s: String) -> Self {
                Self(s)
            }
        }
        impl From<&str> for $t {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }
        impl fmt::Display for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    )*};
}
string_newtype_conversions!(DaemonPath, DaemonSocket, HostPath);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_paths_must_be_absolute_for_the_declared_os() {
        let ok = |p: &str, os| DaemonPath::from(p).validate("f", os).is_ok();
        assert!(ok("/etc/scanopy/key", OsFamily::Unix));
        for bad in ["~/key", "keys/k", r"C:\key"] {
            assert!(!ok(bad, OsFamily::Unix), "{bad} on Unix");
        }
        for good in [
            r"C:\Scanopy\key",
            "D:/keys/k",
            r"\\srv\share\k",
            r"\\?\C:\key",
        ] {
            assert!(ok(good, OsFamily::Windows), "{good} on Windows");
        }
        // `\key` has a root but no drive; `C:key` is relative to C:'s current directory.
        for bad in ["/etc/key", r"\key", "C:key", "~/key"] {
            assert!(!ok(bad, OsFamily::Windows), "{bad} on Windows");
        }
    }

    #[test]
    fn sockets_are_unix_paths_or_named_pipes() {
        let ok = |p: &str, os| DaemonSocket::from(p).validate("f", os).is_ok();
        assert!(ok("/var/run/docker.sock", OsFamily::Unix));
        assert!(ok("unix:///run/podman/podman.sock", OsFamily::Unix));
        assert!(!ok("docker.sock", OsFamily::Unix));
        assert!(ok(r"\\.\pipe\docker_engine", OsFamily::Windows));
        assert!(ok("npipe:////./pipe/docker_engine", OsFamily::Windows));
        assert!(!ok(r"C:\docker.sock", OsFamily::Windows));
        assert!(!ok("/var/run/docker.sock", OsFamily::Windows));
    }

    #[test]
    fn host_paths_must_be_absolute_for_the_target_os() {
        let ok = |p: &str, os| HostPath::from(p).validate("f", os).is_ok();
        assert!(ok("/usr/local/bin/inventory", OsFamily::Unix));
        assert!(!ok("~/inventory", OsFamily::Unix));
        assert!(!ok(r"C:\inventory.ps1", OsFamily::Unix));
        assert!(ok(r"C:\Scanopy\inventory.ps1", OsFamily::Windows));
        assert!(!ok("/usr/local/bin/inventory", OsFamily::Windows));
        assert!(!ok(r#"C:\a"b.ps1"#, OsFamily::Windows));
    }

    /// The quoted word has to survive a real shell, spaces and quotes included, or a script at an
    /// awkward path would run something else.
    #[cfg(unix)]
    #[test]
    fn unix_invocation_survives_a_real_shell() {
        let path = HostPath::from("/opt/it's a dir/run $x;.sh");
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("printf %s {}", path.invocation(OsFamily::Unix)))
            .output()
            .unwrap();
        assert_eq!(String::from_utf8(out.stdout).unwrap(), path.as_str());
    }
}
