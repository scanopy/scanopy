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
        Credential {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: CredentialBase {
                organization_id,
                name: "Linux Inventory".to_string(),
                credential_type: CredentialType::SshKey {
                    port: 22,
                    username: "scanopy".to_string(),
                    // A key file on the daemon host, as an operator would deploy it, rather than an
                    // inline key in the demo database.
                    private_key: SecretValue::FilePath {
                        path: "/etc/scanopy/ssh/inventory_ed25519".to_string(),
                    },
                    passphrase: None,
                    script: LINUX_INVENTORY_SCRIPT.to_string(),
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
                name: "Backup NAS Wake".to_string(),
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

/// What a Linux admin would point the SSH credential at: OS and hardware identity from
/// `/etc/os-release` and DMI, printed as the keys the script contract defines.
const LINUX_INVENTORY_SCRIPT: &str = r#"#!/bin/sh
. /etc/os-release
dmi() { cat "/sys/class/dmi/id/$1" 2>/dev/null; }
printf '{"hostname":"%s","sys_descr":"%s %s","manufacturer":"%s","model":"%s","serial_number":"%s","firmware_revision":"%s","software_revision":"%s"}\n' \
  "$(hostname -f)" "$PRETTY_NAME" "$(uname -r)" \
  "$(dmi sys_vendor)" "$(dmi product_name)" "$(sudo -n cat /sys/class/dmi/id/product_serial)" \
  "$(dmi bios_version)" "$VERSION_ID"
"#;

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
