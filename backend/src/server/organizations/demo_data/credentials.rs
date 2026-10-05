//! Credentials, and site-to-credential junction assignments

use super::*;
use crate::server::credentials::r#impl::types::{
    OsFamily, ScriptSource, SnmpV3AuthProtocol, SnmpV3PrivProtocol,
};

pub(super) fn generate_credentials(organization_id: Uuid, now: DateTime<Utc>) -> Vec<Credential> {
    let credential = |name: &str,
                      description: &str,
                      daemon_os: Option<OsFamily>,
                      credential_type: CredentialType| Credential {
        id: Uuid::new_v4(),
        created_at: now,
        updated_at: now,
        base: CredentialBase {
            organization_id,
            daemon_os,
            name: name.to_string(),
            description: Some(description.to_string()),
            credential_type,
            tags: Vec::new(),
            assigned_site_ids: Vec::new(),
            host_assignments: Vec::new(),
        },
    };
    let inline = |value: &str| SecretValue::Inline {
        value: SecretString::from(value.to_string()),
    };

    vec![
        credential(
            "Default SNMPv2c",
            "Read-only public community most switches ship with.",
            None,
            CredentialType::SnmpV2c {
                community: inline("public"),
            },
        ),
        credential(
            "Network Devices",
            "Community for core and access switches, firewalls and APs. Rotated each quarter by the network team.",
            None,
            CredentialType::SnmpV2c {
                community: inline("acme-network"),
            },
        ),
        credential(
            "Legacy Printers",
            "SNMPv1 community for the older office printers, which predate v2c support.",
            None,
            CredentialType::SnmpV1 {
                community: inline("acme-print"),
            },
        ),
        credential(
            "Data Center SNMPv3",
            "Authenticated, encrypted SNMP for the data center switch and firewall.",
            None,
            CredentialType::SnmpV3 {
                security_name: "scanopy-ro".to_string(),
                auth_protocol: SnmpV3AuthProtocol::Sha256,
                auth_password: inline("dc-snmp-auth-2026"),
                priv_protocol: SnmpV3PrivProtocol::Aes128,
                priv_password: inline("dc-snmp-priv-2026"),
                context_name: None,
            },
        ),
        credential(
            "Arista gNMI",
            "Streaming telemetry from the Arista data center switch over gNMI.",
            None,
            CredentialType::Gnmi {
                // Arista EOS serves gNMI on 6030 rather than the IANA 9339.
                port: 6030,
                username: "scanopy".to_string(),
                password: inline("arista-gnmi-ro"),
                tls: true,
                skip_verify: false,
            },
        ),
        credential(
            "Docker TLS Proxy",
            "TLS proxy in front of the Docker API on the container hosts.",
            None,
            CredentialType::DockerProxy {
                port: 2376,
                path: None,
                ssl_cert: None,
                ssl_key: None,
                ssl_chain: None,
            },
        ),
        credential(
            "Local Docker Socket",
            "Docker socket on each daemon host, read directly by the daemon.",
            None,
            // No path: the daemon finds the socket at its platform default.
            CredentialType::DockerSocket { socket_path: None },
        ),
        credential(
            "Podman API Proxy",
            "Podman API exposed over TCP on the rootless container hosts.",
            None,
            CredentialType::PodmanProxy {
                port: 2375,
                path: None,
                ssl_cert: None,
                ssl_key: None,
                ssl_chain: None,
            },
        ),
        credential(
            "Local Podman Socket",
            "Podman socket for daemons installed on Podman hosts.",
            None,
            CredentialType::PodmanSocket { socket_path: None },
        ),
        credential(
            "UniFi Controller Admin",
            "Read-only local admin on the self-hosted UniFi Network controller.",
            None,
            CredentialType::UnifiLocalAdmin {
                // The self-hosted controller, not a UniFi OS console, so the legacy 8443 port.
                port: 8443,
                site: "default".to_string(),
                username: "scanopy".to_string(),
                password: inline("unifi-ro-admin"),
            },
        ),
        credential(
            "UniFi OS API Key",
            "API key for the Network application on the UniFi OS Cloud Key.",
            None,
            CredentialType::UnifiApiKey {
                port: 443,
                site: "default".to_string(),
                api_key: inline("demo-unifi-os-api-key"),
            },
        ),
        credential(
            "Instant On Cloud Account",
            "Read-only Instant On portal account for the facility switch.",
            None,
            CredentialType::InstantOnAccount {
                username: "netops@acme-corp.com".to_string(),
                password: inline("instant-on-ro"),
                site: Some("Acme HQ".to_string()),
            },
        ),
        credential(
            "Linux Inventory",
            "Runs the inventory script Ansible deploys to every Linux server. Read-only account.",
            Some(OsFamily::Unix),
            CredentialType::SshKey {
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
        ),
        credential(
            "Hypervisor SSH",
            "Password login for the HQ Proxmox nodes, which sit outside Ansible.",
            None,
            CredentialType::SshPassword {
                port: 22,
                username: "scanopy".to_string(),
                password: inline("pve-inventory"),
                target_os: OsFamily::Unix,
                script: ScriptSource::HostFile {
                    path: "/usr/local/bin/scanopy-inventory.sh".into(),
                },
                timeout_seconds: 60,
                host_key_fingerprint: None,
            },
        ),
        credential(
            "Proxmox Cluster API",
            "Read-only API token (PVEAuditor) for the HQ Proxmox cluster. Either node reports both.",
            None,
            CredentialType::ProxmoxApiToken {
                port: 8006,
                token_id: "scanopy@pve!discovery".to_string(),
                token_secret: inline("3f9c2a7e-5b1d-4c8e-9a6f-2d7b8e1c4a90"),
            },
        ),
        credential(
            "Backup NAS Wake",
            "Wakes the backup NAS, which sleeps between weekly backup windows.",
            None,
            CredentialType::WakeOnLan {
                port: 9,
                // The NAS spins up eight drives before DSM answers.
                wait_seconds: 180,
                broadcast_address: None,
                secure_on_password: None,
            },
        ),
    ]
}

pub(super) fn generate_site_credential_assignments(
    sites: &[Site],
    credentials: &[Credential],
) -> Vec<SiteCredentialAssignment> {
    let find_site = |name: &str| sites.iter().find(|n| n.base.name.contains(name)).unwrap();
    let find_cred = |name: &str| {
        credentials
            .iter()
            .find(|c| c.base.name.contains(name))
            .unwrap()
    };

    let default_snmp = find_cred("Default SNMPv2c");
    let network_snmp = find_cred("Network Devices");
    let hq = find_site("Headquarters");
    let dc = find_site("Data Center");

    vec![
        // HQ: both SNMP credentials + Docker proxy
        SiteCredentialAssignment {
            site_id: hq.id,
            credential_ids: vec![default_snmp.id, network_snmp.id],
        },
        // DC: both SNMP credentials
        SiteCredentialAssignment {
            site_id: dc.id,
            credential_ids: vec![default_snmp.id, network_snmp.id],
        },
    ]
}
