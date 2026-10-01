//! What each stored credential did in one discovery run: the results half of a run's record, next
//! to its warnings.
//!
//! One row per credential. Its name, type and integration come from the credential id, so nothing
//! here repeats them. The outcome's shape follows the kind of integration: a count for the ones
//! that collect from a host, per-host detail for SSH scripts and Wake-on-LAN.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::server::credentials::r#impl::mapping::CredentialQueryPayloadDiscriminants;
use crate::server::credentials::r#impl::types::ssh_script::SshScriptRun;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct CredentialRunResult {
    pub credential_id: Uuid,
    pub outcome: CredentialRunOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum CredentialRunOutcome {
    /// An integration that collects from a host: how many hosts it collected from.
    #[schema(title = "Collected")]
    Collected { hosts: u32 },
    /// One entry per host the script ran on.
    #[schema(title = "SshScript")]
    SshScript { runs: Vec<SshScriptRun> },
    /// One entry per address the daemon tried to wake.
    #[schema(title = "WakeOnLan")]
    WakeOnLan { hosts: Vec<WakeOnLanResult> },
    /// An outcome from a newer daemon than this server.
    #[serde(other)]
    Unknown,
}

impl CredentialRunOutcome {
    /// The empty outcome for a credential of this kind, before anything is recorded. Exhaustive,
    /// so a new integration has to say which shape its results take.
    pub fn empty_for(integration: CredentialQueryPayloadDiscriminants) -> Self {
        use CredentialQueryPayloadDiscriminants as D;
        match integration {
            D::Snmp
            | D::Gnmi
            | D::DockerProxy
            | D::DockerSocket
            | D::PodmanProxy
            | D::PodmanSocket
            | D::UnifiController
            | D::InstantOn => Self::Collected { hosts: 0 },
            D::Ssh => Self::SshScript { runs: Vec::new() },
            D::WakeOnLan => Self::WakeOnLan { hosts: Vec::new() },
            D::Unknown => Self::Unknown,
        }
    }
}

/// Whether one address woke.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct WakeOnLanResult {
    #[schema(value_type = String)]
    pub ip: IpAddr,
    pub woke: bool,
    /// How long after the packets the address answered, or how long the daemon waited for it.
    pub waited_ms: u64,
}
