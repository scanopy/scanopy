//! Proxmox VE API token credential type for discovery dispatch.
//!
//! A Proxmox VE node's API answers for its whole cluster, so the credential sits on any one node
//! and the integration reports every node and guest it can see.

use crate::server::credentials::r#impl::mapping::{
    BannerField, BannerFieldValue, ResolvableSecret,
};
use serde::{Deserialize, Serialize};

/// Proxmox VE serves its API and web UI on 8006.
pub const DEFAULT_PROXMOX_PORT: u16 = 8006;

pub fn default_proxmox_port() -> u16 {
    DEFAULT_PROXMOX_PORT
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct ProxmoxQueryCredential {
    pub port: u16,
    /// `user@realm!tokenname`.
    pub token_id: String,
    pub token_secret: ResolvableSecret,
}

impl ProxmoxQueryCredential {
    pub fn banner_lines(&self) -> Vec<BannerField> {
        vec![
            BannerField {
                label: "Port",
                value: BannerFieldValue::Plain(self.port.to_string()),
            },
            BannerField {
                label: "Token ID",
                value: BannerFieldValue::Plain(self.token_id.clone()),
            },
            BannerField {
                label: "Token secret",
                value: self.token_secret.banner_value(),
            },
        ]
    }
}
