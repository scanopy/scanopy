//! Wake-on-LAN credential type for discovery dispatch.
//!
//! Not a credential in the authentication sense: the only secret is the optional SecureOn
//! password some NICs require. It is a credential because it is per-host configuration the daemon
//! needs at scan time, assigned the same way, and dispatched through the same mapping.
//!
//! The target is addressed by MAC, not IP. A sleeping host has no IP stack running; its NIC
//! watches for a magic packet carrying its own MAC, so the server sends the MACs it knows for each
//! assigned address alongside the override (`IpOverride::mac_address`).

use crate::server::credentials::r#impl::mapping::{
    BannerField, BannerFieldValue, ResolvableSecret,
};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// The discard port. Most NICs match the payload regardless of port; 9 is what `wakeonlan`,
/// `etherwake` and router relay rules default to.
pub const DEFAULT_WAKE_ON_LAN_PORT: u16 = 9;

/// How long to wait for a woken host before the sweep starts. A NAS spinning up disks from S5
/// commonly takes over a minute.
pub const DEFAULT_WAKE_ON_LAN_WAIT_SECONDS: u32 = 90;

pub fn default_wake_on_lan_port() -> u16 {
    DEFAULT_WAKE_ON_LAN_PORT
}

pub fn default_wake_on_lan_wait_seconds() -> u32 {
    DEFAULT_WAKE_ON_LAN_WAIT_SECONDS
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct WakeOnLanQueryCredential {
    pub port: u16,
    pub wait_seconds: u32,
    /// Send here instead of the target subnet's directed broadcast: a router relay address, a WoL
    /// relay device, or `255.255.255.255` for a plain local broadcast.
    #[serde(default)]
    pub broadcast_address: Option<IpAddr>,
    /// Six bytes written as a MAC address, appended to the packet for NICs that require it.
    #[serde(default)]
    pub secure_on_password: Option<ResolvableSecret>,
}

impl WakeOnLanQueryCredential {
    pub fn banner_lines(&self) -> Vec<BannerField> {
        let mut lines = vec![
            BannerField {
                label: "Port",
                value: BannerFieldValue::Plain(self.port.to_string()),
            },
            BannerField {
                label: "Wait",
                value: BannerFieldValue::Plain(format!("{}s", self.wait_seconds)),
            },
        ];
        if let Some(addr) = &self.broadcast_address {
            lines.push(BannerField {
                label: "Broadcast address",
                value: BannerFieldValue::Plain(addr.to_string()),
            });
        }
        if let Some(password) = &self.secure_on_password {
            lines.push(BannerField {
                label: "SecureOn password",
                value: password.banner_value(),
            });
        }
        lines
    }
}
