//! Credentials, and network-to-credential junction assignments

use super::*;

pub(super) fn generate_credentials(organization_id: Uuid, now: DateTime<Utc>) -> Vec<Credential> {
    vec![
        Credential {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: CredentialBase {
                organization_id,
                name: "Default SNMPv2c".to_string(),
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
                name: "Network Devices".to_string(),
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
                name: "Docker TLS Proxy".to_string(),
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
