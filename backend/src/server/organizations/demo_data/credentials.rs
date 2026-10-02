//! Credentials, and network-to-credential junction assignments

use super::*;
use crate::server::credentials::r#impl::types::{OsFamily, ScriptSource};

pub(super) fn generate_credentials(organization_id: Uuid, now: DateTime<Utc>) -> Vec<Credential> {
    vec![
        Credential {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: CredentialBase {
                organization_id,
                daemon_os: None,
                name: "Default SNMPv2c".to_string(),
                description: Some("Read-only public community most switches ship with.".to_string()),
                credential_type: CredentialType::SnmpV2c {
                    community: SecretValue::Inline {
                        value: SecretString::from("public".to_string()),
                    },
                },
                tags: Vec::new(),
                assigned_network_ids: Vec::new(),
                host_assignments: Vec::new(),
            },
        },
        Credential {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: CredentialBase {
                organization_id,
                daemon_os: None,
                name: "Network Devices".to_string(),
                description: Some(
                    "Community for core and access switches, firewalls and APs. Rotated each quarter by the network team."
                        .to_string(),
                ),
                credential_type: CredentialType::SnmpV2c {
                    community: SecretValue::Inline {
                        value: SecretString::from("acme-network".to_string()),
                    },
                },
                tags: Vec::new(),
                assigned_network_ids: Vec::new(),
                host_assignments: Vec::new(),
            },
        },
        Credential {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: CredentialBase {
                organization_id,
                daemon_os: None,
                name: "Docker TLS Proxy".to_string(),
                description: Some("TLS proxy in front of the Docker API on the container hosts.".to_string()),
                credential_type: CredentialType::DockerProxy {
                    port: 2376,
                    path: None,
                    ssl_cert: None,
                    ssl_key: None,
                    ssl_chain: None,
                },
                tags: Vec::new(),
                assigned_network_ids: Vec::new(),
                host_assignments: Vec::new(),
            },
        },
        Credential {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: CredentialBase {
                organization_id,
                daemon_os: Some(OsFamily::Unix),
                name: "Linux Inventory".to_string(),
                description: Some(
                    "Runs the inventory script Ansible deploys to every Linux server. Read-only account.".to_string(),
                ),
                credential_type: CredentialType::SshKey {
                    target_os: OsFamily::Unix,
                    port: 22,
                    username: "scanopy".to_string(),
                    // A key file on the daemon host, as an operator would deploy it, rather than an
                    // inline key in the demo database.
                    private_key: SecretValue::FilePath {
                        path: "/etc/scanopy/ssh/inventory_ed25519".into(),
                    },
                    passphrase: None,
                    // The default mode: the script is deployed to each server (by config management),
                    // and only its path is stored here.
                    script: ScriptSource::HostFile {
                        path: "/usr/local/bin/scanopy-inventory.sh".into(),
                    },
                    timeout_seconds: 60,
                    host_key_fingerprint: None,
                },
                tags: Vec::new(),
                assigned_network_ids: Vec::new(),
                host_assignments: Vec::new(),
            },
        },
        Credential {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: CredentialBase {
                organization_id,
                daemon_os: None,
                name: "Backup NAS Wake".to_string(),
                description: Some("Wakes the backup NAS, which sleeps between weekly backup windows.".to_string()),
                credential_type: CredentialType::WakeOnLan {
                    port: 9,
                    // The NAS spins up eight drives before DSM answers.
                    wait_seconds: 180,
                    broadcast_address: None,
                    secure_on_password: None,
                },
                tags: Vec::new(),
                assigned_network_ids: Vec::new(),
                host_assignments: Vec::new(),
            },
        },
    ]
}

pub(super) fn generate_network_credential_assignments(
    networks: &[Network],
    credentials: &[Credential],
) -> Vec<NetworkCredentialAssignment> {
    let find_network = |name: &str| {
        networks
            .iter()
            .find(|n| n.base.name.contains(name))
            .unwrap()
    };
    let find_cred = |name: &str| {
        credentials
            .iter()
            .find(|c| c.base.name.contains(name))
            .unwrap()
    };

    let default_snmp = find_cred("Default SNMPv2c");
    let network_snmp = find_cred("Network Devices");
    let hq = find_network("Headquarters");
    let dc = find_network("Data Center");

    vec![
        // HQ: both SNMP credentials + Docker proxy
        NetworkCredentialAssignment {
            network_id: hq.id,
            credential_ids: vec![default_snmp.id, network_snmp.id],
        },
        // DC: both SNMP credentials
        NetworkCredentialAssignment {
            network_id: dc.id,
            credential_ids: vec![default_snmp.id, network_snmp.id],
        },
    ]
}
