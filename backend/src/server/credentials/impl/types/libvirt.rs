//! libvirt credential type for discovery dispatch.
//!
//! The daemon logs in over SSH with the credential's SSH login and reads the host's domains with
//! `virsh`, so the wire payload is the login's fields plus the connection URI `virsh` opens.

use crate::server::credentials::r#impl::mapping::{BannerField, BannerFieldValue, SshAuth};
use serde::{Deserialize, Serialize};

/// The system libvirt daemon, where the guests a host runs as a hypervisor are defined.
pub const DEFAULT_LIBVIRT_URI: &str = "qemu:///system";

pub fn default_libvirt_uri() -> String {
    DEFAULT_LIBVIRT_URI.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct LibvirtQueryCredential {
    /// The SSH login's port, account, auth and required host key.
    pub port: u16,
    pub username: String,
    pub auth: SshAuth,
    #[serde(default)]
    pub host_key_fingerprint: Option<String>,
    /// The URI `virsh` connects to on the host.
    pub connection_uri: String,
}

impl LibvirtQueryCredential {
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
            label: "Connection URI",
            value: BannerFieldValue::Plain(self.connection_uri.clone()),
        });
        lines
    }
}
