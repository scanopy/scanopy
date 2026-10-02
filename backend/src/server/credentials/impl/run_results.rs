//! What each stored credential did in one discovery run: the results half of a run's record, next
//! to its warnings.
//!
//! One row per credential. Its name, type and integration come from the credential id, so nothing
//! here repeats them. The outcome's shape follows the type of integration: a count for the ones
//! that collect from a host, per-host detail for SSH scripts and Wake-on-LAN.

use std::net::IpAddr;

use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::server::credentials::r#impl::mapping::CredentialQueryPayloadDiscriminants;
use crate::server::credentials::r#impl::types::ssh_script::SshScriptRun;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct CredentialRunResult {
    /// The credential these results belong to.
    pub credential_id: Uuid,
    /// What it did in this run.
    pub outcome: CredentialRunOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum CredentialRunOutcome {
    /// An integration that collects from a host: how many hosts it collected from.
    #[schema(title = "Collected")]
    Collected {
        /// Number of hosts it collected from.
        hosts: u32,
    },
    /// One entry per host the script ran on.
    #[schema(title = "SshScript")]
    SshScript {
        /// The script runs, one per host.
        runs: Vec<SshScriptRun>,
    },
    /// One entry per address the daemon tried to wake.
    #[schema(title = "WakeOnLan")]
    WakeOnLan {
        /// The addresses woken or tried, one per address.
        hosts: Vec<WakeOnLanResult>,
    },
    /// An outcome from a newer daemon than this server.
    #[schema(title = "Unknown")]
    #[serde(other)]
    Unknown,
}

impl CredentialRunOutcome {
    /// The empty outcome for a credential of this type, before anything is recorded. Exhaustive,
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

    /// Record whether one address woke, replacing any earlier result for it: the wake step records
    /// every target as not woken, and the Wake-on-LAN integration overwrites the ones the scan
    /// found. A no-op on any other outcome.
    pub fn record_wake_on_lan(&mut self, result: WakeOnLanResult) {
        if let Self::WakeOnLan { hosts } = self {
            match hosts.iter_mut().find(|h| h.ip == result.ip) {
                Some(existing) => *existing = result,
                None => hosts.push(result),
            }
        }
    }
}

/// Reads `credential_results` one entry at a time, so an entry this server can't read costs that
/// entry's outcome and not the whole payload. A daemon newer than the server can change an
/// outcome's shape (a field added, dropped or retyped), and `#[serde(other)]` only covers a new
/// `type` tag. An entry whose outcome doesn't parse keeps its credential as `Unknown`; one without a
/// readable `credential_id` has nothing to attach to and is dropped.
pub fn deserialize_credential_results<'de, D>(
    deserializer: D,
) -> Result<Vec<CredentialRunResult>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: Vec<serde_json::Value> = Vec::deserialize(deserializer)?;
    Ok(raw.into_iter().filter_map(result_from_value).collect())
}

fn result_from_value(value: serde_json::Value) -> Option<CredentialRunResult> {
    if let Ok(result) = serde_json::from_value::<CredentialRunResult>(value.clone()) {
        return Some(result);
    }
    let credential_id = value
        .get("credential_id")
        .and_then(|id| serde_json::from_value::<Uuid>(id.clone()).ok());
    match credential_id {
        Some(credential_id) => {
            tracing::warn!(%credential_id, entry = %value, "Unreadable credential result outcome");
            Some(CredentialRunResult {
                credential_id,
                outcome: CredentialRunOutcome::Unknown,
            })
        }
        None => {
            tracing::warn!(entry = %value, "Dropping credential result without a credential id");
            None
        }
    }
}

/// Whether one address woke: whether the scan found it after the magic packets went out.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct WakeOnLanResult {
    /// The address the magic packets were sent for.
    #[schema(value_type = String)]
    pub ip: IpAddr,
    /// Whether the scan found the address after the packets went out.
    pub woke: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(ip: &str, woke: bool) -> WakeOnLanResult {
        WakeOnLanResult {
            ip: ip.parse().unwrap(),
            woke,
        }
    }

    #[test]
    fn a_host_the_scan_found_replaces_its_not_woken_entry() {
        let mut outcome =
            CredentialRunOutcome::empty_for(CredentialQueryPayloadDiscriminants::WakeOnLan);
        outcome.record_wake_on_lan(result("10.0.40.21", false));
        outcome.record_wake_on_lan(result("10.0.40.22", false));
        outcome.record_wake_on_lan(result("10.0.40.21", true));

        assert_eq!(
            outcome,
            CredentialRunOutcome::WakeOnLan {
                hosts: vec![result("10.0.40.21", true), result("10.0.40.22", false)]
            }
        );
    }
}
