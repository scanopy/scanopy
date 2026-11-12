//! libvirt credential type for discovery dispatch.
//!
//! The daemon logs in over SSH with the credential's SSH login and reads the host's domains with
//! `virsh`, so the wire payload is the login plus the connection URI `virsh` opens.

use crate::server::credentials::r#impl::mapping::{BannerField, BannerFieldValue};
use crate::server::credentials::r#impl::types::ssh::SshLogin;
use serde::{Deserialize, Serialize};

/// The system libvirt daemon, where the guests a host runs as a hypervisor are defined.
pub const DEFAULT_LIBVIRT_URI: &str = "qemu:///system";

pub fn default_libvirt_uri() -> String {
    DEFAULT_LIBVIRT_URI.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct LibvirtQueryCredential {
    /// The SSH login it signs in with.
    #[serde(flatten)]
    pub login: SshLogin,
    /// The URI `virsh` connects to on the host.
    pub connection_uri: String,
}

impl LibvirtQueryCredential {
    pub fn banner_lines(&self) -> Vec<BannerField> {
        let mut lines = self.login.banner_lines();
        lines.push(BannerField {
            label: "Connection URI",
            value: BannerFieldValue::Plain(self.connection_uri.clone()),
        });
        lines
    }
}
