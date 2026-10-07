use super::CredentialType;
use super::paths::OsFamily;
use crate::server::shared::types::field_definition::{
    DependentPlaceholder, FieldDefinition, FieldType, InlineFormat,
};

/// The id a `DependentPlaceholder` uses for the credential's own `daemon_os`, which is a column on
/// the credential rather than a field of its type.
pub const DAEMON_OS_FIELD: &str = "daemon_os";

/// The SSH credentials' Scanned Host OS picker. Fields whose placeholder follows it are the ones
/// that depend on the scanned host's OS.
pub const TARGET_OS_FIELD: &str = "target_os";

// ============================================================================
// Credential field definitions — the form each credential type renders.
//
// The `FieldDefinition` / `FieldType` / `SelectOption` / `InlineFormat` types themselves live in
// `server/shared/types/field_definition.rs`; discovery scan settings build the same ones.
// ============================================================================

impl CredentialType {
    /// Returns field definitions for this credential type.
    /// Uses exhaustive destructuring for compile-time enforcement:
    /// adding a field to the enum variant without updating this method causes a compile error.
    pub fn field_definitions(&self) -> Vec<FieldDefinition> {
        match self {
            Self::SnmpV1 { community: _ } | Self::SnmpV2c { community: _ } => {
                vec![FieldDefinition {
                    id: "community",
                    label: "Community String",
                    field_type: FieldType::SecretPathOrInline,
                    placeholder: Some("custom-community-string"),
                    placeholder_by: None,
                    secret: true,
                    optional: false,
                    help_text: Some(
                        "Custom SNMP community string. The default 'public' community is always tried automatically during scans. On Cisco switches, append '@' and a VLAN id — for example 'readonly@20' — to read that VLAN's MAC-address table, which the plain community does not return.",
                    ),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::Plain),
                    group: None,
                    file_name: Some("snmp-community"),
                    half_width: false,
                }]
            }
            Self::Gnmi {
                port: _,
                username: _,
                password: _,
                // API-settable only for now: no boolean field type exists in this form
                // vocabulary, and the validated lab NOSes serve plaintext gRPC anyway.
                tls: _,
                skip_verify: _,
            } => vec![
                FieldDefinition {
                    id: "username",
                    label: "Username",
                    field_type: FieldType::String,
                    placeholder: Some("gnmi-user"),
                    placeholder_by: None,
                    secret: false,
                    optional: false,
                    help_text: Some(
                        "Sent as gRPC `username` metadata (the OpenConfig convention).",
                    ),
                    options: None,
                    default_value: None,
                    inline_format: None,
                    group: Some("Connection"),
                    file_name: None,
                    half_width: true,
                },
                FieldDefinition {
                    id: "port",
                    label: "gNMI Port",
                    field_type: FieldType::Port,
                    placeholder: Some("9339"),
                    placeholder_by: None,
                    secret: false,
                    optional: true,
                    help_text: Some(
                        "9339 is the IANA gNMI port; some NOSes listen on 6030 or 57400 instead.",
                    ),
                    options: None,
                    default_value: Some("9339"),
                    inline_format: None,
                    group: Some("Connection"),
                    file_name: None,
                    half_width: true,
                },
                FieldDefinition {
                    id: "password",
                    label: "Password",
                    field_type: FieldType::SecretPathOrInline,
                    placeholder: None,
                    placeholder_by: None,
                    secret: true,
                    optional: false,
                    help_text: Some("Sent as gRPC `password` metadata."),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::Plain),
                    group: Some("Authentication"),
                    file_name: Some("gnmi-password"),
                    half_width: false,
                },
            ],
            Self::SnmpV3 {
                security_name: _,
                auth_protocol: _,
                auth_password: _,
                priv_protocol: _,
                priv_password: _,
                context_name: _,
            } => vec![
                FieldDefinition {
                    id: "security_name",
                    label: "Security Name",
                    field_type: FieldType::String,
                    placeholder: Some("snmp-user"),
                    placeholder_by: None,
                    secret: false,
                    optional: false,
                    help_text: Some("SNMPv3 USM user name (security name)."),
                    options: None,
                    default_value: None,
                    inline_format: None,
                    group: Some("Authentication"),
                    file_name: None,
                    half_width: true,
                },
                FieldDefinition {
                    id: "auth_protocol",
                    label: "Auth Protocol",
                    field_type: FieldType::Select,
                    placeholder: None,
                    placeholder_by: None,
                    secret: false,
                    optional: false,
                    help_text: Some("Authentication hash algorithm."),
                    options: Some(super::snmp::SnmpV3AuthProtocol::OPTIONS),
                    default_value: Some("Sha256"),
                    inline_format: None,
                    group: Some("Authentication"),
                    file_name: None,
                    half_width: true,
                },
                FieldDefinition {
                    id: "auth_password",
                    label: "Auth Password",
                    field_type: FieldType::SecretPathOrInline,
                    placeholder: None,
                    placeholder_by: None,
                    secret: true,
                    optional: false,
                    help_text: Some("Authentication password (minimum 8 characters)."),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::Plain),
                    group: Some("Authentication"),
                    file_name: Some("snmpv3-auth-password"),
                    half_width: false,
                },
                FieldDefinition {
                    id: "priv_protocol",
                    label: "Privacy Protocol",
                    field_type: FieldType::Select,
                    placeholder: None,
                    placeholder_by: None,
                    secret: false,
                    optional: false,
                    help_text: Some("Privacy (encryption) algorithm."),
                    options: Some(super::snmp::SnmpV3PrivProtocol::OPTIONS),
                    default_value: Some("Aes128"),
                    inline_format: None,
                    group: Some("Privacy"),
                    file_name: None,
                    half_width: false,
                },
                FieldDefinition {
                    id: "priv_password",
                    label: "Privacy Password",
                    field_type: FieldType::SecretPathOrInline,
                    placeholder: None,
                    placeholder_by: None,
                    secret: true,
                    optional: false,
                    help_text: Some("Privacy (encryption) password (minimum 8 characters)."),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::Plain),
                    group: Some("Privacy"),
                    file_name: Some("snmpv3-priv-password"),
                    half_width: false,
                },
                FieldDefinition {
                    id: "context_name",
                    label: "Context Name",
                    field_type: FieldType::String,
                    placeholder: Some("(default)"),
                    placeholder_by: None,
                    secret: false,
                    optional: true,
                    help_text: Some(
                        "Optional SNMPv3 context name, applied to bridge and VLAN queries only. Cisco and some other vendors keep a separate MAC-address table per VLAN in a named context; naming it here reads that table instead of the near-empty default one. Interface, LLDP and ARP data always come from the default context.",
                    ),
                    options: None,
                    default_value: None,
                    inline_format: None,
                    group: None,
                    file_name: None,
                    half_width: false,
                },
            ],
            Self::DockerProxy { .. } => container_proxy_field_definitions(
                "Docker API Port",
                "Docker API port. Use 2375 for non-TLS proxies (Tecnativa, HAProxy) or 2376 for TLS.",
            ),
            Self::PodmanProxy { .. } => container_proxy_field_definitions(
                "Podman API Port",
                "Podman API port. Point at a TCP-exposed Podman service (e.g. `podman system service tcp:`) directly or behind a TLS proxy.",
            ),
            Self::DockerSocket { .. } => vec![socket_path_field(
                "/var/run/docker.sock",
                &[DependentPlaceholder {
                    depends_on: DAEMON_OS_FIELD,
                    value: "Windows",
                    placeholder: r"\\.\pipe\docker_engine",
                }],
                "The Docker socket on Unix, or its named pipe on Windows. Leave blank to auto-detect (DOCKER_HOST or /var/run/docker.sock on Unix, the docker_engine pipe on Windows).",
            )],
            Self::PodmanSocket { .. } => vec![socket_path_field(
                "/run/podman/podman.sock",
                &[DependentPlaceholder {
                    depends_on: DAEMON_OS_FIELD,
                    value: "Windows",
                    placeholder: r"\\.\pipe\podman-machine-default",
                }],
                "The Podman socket on Unix, or Podman machine's named pipe on Windows. Leave blank to auto-detect (CONTAINER_HOST, then the rootful and rootless sockets on Unix or the podman-machine-default pipe on Windows).",
            )],
            Self::UnifiApiKey { .. } => {
                let mut fields = unifi_connection_fields();
                fields.push(FieldDefinition {
                    id: "api_key",
                    label: "API Key",
                    field_type: FieldType::SecretPathOrInline,
                    placeholder: None,
                    placeholder_by: None,
                    secret: true,
                    optional: false,
                    help_text: Some(
                        "Network Application API key (Settings → Control Plane → Integrations → Create API Key). Requires UniFi OS; the legacy self-hosted Network Application on 8443 has no API keys — use a UniFi Local Admin credential there instead.",
                    ),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::Plain),
                    group: Some("Authentication"),
                    file_name: Some("unifi-api-key"),
                    half_width: false,
                });
                fields
            }
            Self::UnifiLocalAdmin { .. } => {
                let mut fields = unifi_connection_fields();
                fields.push(FieldDefinition {
                    id: "username",
                    label: "Username",
                    field_type: FieldType::String,
                    placeholder: None,
                    placeholder_by: None,
                    secret: false,
                    optional: false,
                    help_text: Some(
                        "Controller admin username. Use a local-only admin account so multi-factor authentication does not block the login.",
                    ),
                    options: None,
                    default_value: None,
                    inline_format: None,
                    group: Some("Authentication"),
                    file_name: None,
                    half_width: false,
                });
                fields.push(FieldDefinition {
                    id: "password",
                    label: "Password",
                    field_type: FieldType::SecretPathOrInline,
                    placeholder: None,
                    placeholder_by: None,
                    secret: true,
                    optional: false,
                    help_text: Some("Password for the local admin account."),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::Plain),
                    group: Some("Authentication"),
                    file_name: Some("unifi-password"),
                    half_width: false,
                });
                fields
            }
            Self::InstantOnAccount { .. } => vec![
                FieldDefinition {
                    id: "username",
                    label: "Portal Account",
                    field_type: FieldType::String,
                    placeholder: Some("scanopy@example.com"),
                    placeholder_by: None,
                    secret: false,
                    optional: false,
                    help_text: Some(
                        "Email address of an Instant On portal account with access to the site. Add a dedicated account with the read-only Viewer role (Site management → Accounts managing this site) rather than using your own administrator login, and make sure multi-factor authentication is disabled on it — the sign-in cannot answer an MFA prompt.",
                    ),
                    options: None,
                    default_value: None,
                    inline_format: None,
                    group: Some("Authentication"),
                    file_name: None,
                    half_width: false,
                },
                FieldDefinition {
                    id: "password",
                    label: "Password",
                    field_type: FieldType::SecretPathOrInline,
                    placeholder: None,
                    placeholder_by: None,
                    secret: true,
                    optional: false,
                    help_text: Some("Password for that portal account."),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::Plain),
                    group: Some("Authentication"),
                    file_name: Some("instant-on-password"),
                    half_width: false,
                },
                FieldDefinition {
                    id: "site",
                    label: "Site",
                    field_type: FieldType::String,
                    placeholder: Some("(all sites)"),
                    placeholder_by: None,
                    secret: false,
                    optional: true,
                    help_text: Some(
                        "Limit the fetch to one Instant On site by name. Leave blank to read every site this account can see. Assign this credential to a single switch — each host it is assigned to fetches the whole site again.",
                    ),
                    options: None,
                    default_value: None,
                    inline_format: None,
                    group: Some("Scope"),
                    file_name: None,
                    half_width: false,
                },
            ],
            Self::SshPassword {
                port: _,
                username: _,
                password: _,
                target_os: _,
                script: _,
                timeout_seconds: _,
                host_key_fingerprint: _,
            } => ssh_field_definitions(vec![FieldDefinition {
                id: "password",
                label: "Password",
                field_type: FieldType::SecretPathOrInline,
                placeholder: None,
                placeholder_by: None,
                secret: true,
                optional: false,
                help_text: Some(
                    "Password for that account. The server must allow password authentication.",
                ),
                options: None,
                default_value: None,
                inline_format: Some(InlineFormat::Plain),
                group: Some("Authentication"),
                file_name: Some("ssh-password"),
                half_width: false,
            }]),
            Self::SshKey {
                port: _,
                username: _,
                private_key: _,
                passphrase: _,
                target_os: _,
                script: _,
                timeout_seconds: _,
                host_key_fingerprint: _,
            } => ssh_field_definitions(vec![
                FieldDefinition {
                    id: "private_key",
                    label: "Private Key",
                    field_type: FieldType::SecretPathOrInline,
                    placeholder: None,
                    placeholder_by: None,
                    secret: true,
                    optional: false,
                    help_text: Some(
                        "OpenSSH (ssh-keygen's default) or PEM private key whose public key is in the account's authorized_keys.",
                    ),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::SshPrivateKey),
                    group: Some("Authentication"),
                    file_name: Some("ssh-key"),
                    half_width: false,
                },
                FieldDefinition {
                    id: "passphrase",
                    label: "Key Passphrase",
                    field_type: FieldType::SecretPathOrInline,
                    placeholder: None,
                    placeholder_by: None,
                    secret: true,
                    optional: true,
                    help_text: Some("Only if the private key is encrypted."),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::Plain),
                    group: Some("Authentication"),
                    file_name: Some("ssh-key-passphrase"),
                    half_width: false,
                },
            ]),
            Self::WakeOnLan {
                port: _,
                wait_seconds: _,
                broadcast_address: _,
                secure_on_password: _,
            } => vec![
                FieldDefinition {
                    id: "broadcast_address",
                    label: "Broadcast Address",
                    field_type: FieldType::String,
                    placeholder: Some("(host's subnet broadcast)"),
                    placeholder_by: None,
                    secret: false,
                    optional: true,
                    help_text: Some(
                        "Leave blank to send to the broadcast address of each host's subnet, which reaches it when the daemon is on the same network segment or the router forwards directed broadcasts. Otherwise enter where to send it instead: a router address with a relay rule, a Wake-on-LAN relay device, or 255.255.255.255.",
                    ),
                    options: None,
                    default_value: None,
                    inline_format: None,
                    group: Some("Delivery"),
                    file_name: None,
                    half_width: true,
                },
                FieldDefinition {
                    id: "port",
                    label: "UDP Port",
                    field_type: FieldType::Port,
                    placeholder: Some("9"),
                    placeholder_by: None,
                    secret: false,
                    optional: true,
                    help_text: Some(
                        "Most network cards accept the packet on any port. Change it only if a router relay rule listens on another port, commonly 7.",
                    ),
                    options: None,
                    default_value: Some("9"),
                    inline_format: None,
                    group: Some("Delivery"),
                    file_name: None,
                    half_width: true,
                },
                FieldDefinition {
                    id: "wait_seconds",
                    label: "Wait (seconds)",
                    field_type: FieldType::Number,
                    placeholder: Some("90"),
                    placeholder_by: None,
                    secret: false,
                    optional: true,
                    help_text: Some(
                        "How long the daemon waits after sending the packets before the scan starts. Hosts the scan finds count as woken. Allow for disks spinning up and services starting.",
                    ),
                    options: None,
                    default_value: Some("90"),
                    inline_format: None,
                    group: Some("Delivery"),
                    file_name: None,
                    half_width: false,
                },
                FieldDefinition {
                    id: "secure_on_password",
                    label: "SecureOn Password",
                    field_type: FieldType::SecretPathOrInline,
                    placeholder: Some("01:23:45:67:89:ab"),
                    placeholder_by: None,
                    secret: true,
                    optional: true,
                    help_text: Some(
                        "Only for network cards configured to require one. Six bytes written as a MAC address.",
                    ),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::MacAddress),
                    group: Some("Delivery"),
                    file_name: Some("secureon-password"),
                    half_width: false,
                },
            ],
            Self::ProxmoxApiToken {
                port: _,
                token_id: _,
                token_secret: _,
            } => vec![
                FieldDefinition {
                    id: "token_id",
                    label: "Token ID",
                    field_type: FieldType::String,
                    placeholder: Some("scanopy@pve!discovery"),
                    placeholder_by: None,
                    secret: false,
                    optional: false,
                    help_text: Some(
                        "The full token ID, user@realm!tokenname (Datacenter → Permissions → API Tokens). Give the token the PVEAuditor role on / and, for VM addresses from the guest agent, VM.GuestAgent.Audit on Proxmox VE 9 or VM.Monitor on 8. Assign the credential to one node: its API answers for the whole cluster.",
                    ),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::ProxmoxTokenId),
                    group: Some("Connection"),
                    file_name: None,
                    half_width: true,
                },
                FieldDefinition {
                    id: "port",
                    label: "API Port",
                    field_type: FieldType::Port,
                    placeholder: Some("8006"),
                    placeholder_by: None,
                    secret: false,
                    optional: true,
                    help_text: Some(
                        "The port the Proxmox VE web interface and API listen on. 8006 unless a reverse proxy sits in front of it.",
                    ),
                    options: None,
                    default_value: Some("8006"),
                    inline_format: None,
                    group: Some("Connection"),
                    file_name: None,
                    half_width: true,
                },
                FieldDefinition {
                    id: "token_secret",
                    label: "Token Secret",
                    field_type: FieldType::SecretPathOrInline,
                    placeholder: None,
                    placeholder_by: None,
                    secret: true,
                    optional: false,
                    help_text: Some("The secret shown once when the token is created, a UUID."),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::Plain),
                    group: Some("Authentication"),
                    file_name: Some("proxmox-token-secret"),
                    half_width: false,
                },
            ],
        }
    }
}

/// Fields shared by both SSH transports, around the transport's own auth fields: connection
/// first, then auth, then the script and how it is trusted and bounded.
fn ssh_field_definitions(auth_fields: Vec<FieldDefinition>) -> Vec<FieldDefinition> {
    let mut fields = vec![
        FieldDefinition {
            id: "username",
            label: "Username",
            field_type: FieldType::String,
            placeholder: Some("scanopy"),
            placeholder_by: None,
            secret: false,
            optional: false,
            help_text: Some(
                "Account the script runs as. Use a dedicated account with only the access the script needs.",
            ),
            options: None,
            default_value: None,
            inline_format: None,
            group: Some("Connection"),
            file_name: None,
            half_width: true,
        },
        FieldDefinition {
            id: "port",
            label: "SSH Port",
            field_type: FieldType::Port,
            placeholder: Some("22"),
            placeholder_by: None,
            secret: false,
            optional: true,
            help_text: None,
            options: None,
            default_value: Some("22"),
            inline_format: None,
            group: Some("Connection"),
            file_name: None,
            half_width: true,
        },
    ];
    fields.extend(auth_fields);
    fields.extend([
        FieldDefinition {
            id: TARGET_OS_FIELD,
            label: "Scanned Host OS",
            field_type: FieldType::Radio,
            placeholder: None,
            placeholder_by: None,
            secret: false,
            optional: false,
            help_text: Some(
                "Sets how the script runs, in the login shell on Linux, macOS and BSD or in PowerShell on Windows, and the path format for a file on the scanned host.",
            ),
            options: Some(OsFamily::OPTIONS),
            default_value: Some("Unix"),
            inline_format: None,
            group: Some("Script"),
            file_name: None,
            half_width: false,
        },
        FieldDefinition {
            id: "script",
            label: "Script",
            field_type: FieldType::ScriptSource,
            // The file-on-scanned-host path, which is the default mode.
            placeholder: Some("/usr/local/bin/scanopy-inventory.sh"),
            placeholder_by: Some(&[DependentPlaceholder {
                depends_on: TARGET_OS_FIELD,
                value: "Windows",
                placeholder: r"C:\Scanopy\scanopy-inventory.ps1",
            }]),
            secret: false,
            optional: false,
            help_text: Some(
                "Runs at each scan. Must print one JSON object of host fields.",
            ),
            options: None,
            default_value: None,
            inline_format: None,
            group: Some("Script"),
            file_name: Some("scanopy-inventory.sh"),
            half_width: false,
        },
        FieldDefinition {
            id: "timeout_seconds",
            label: "Timeout (seconds)",
            field_type: FieldType::Number,
            placeholder: Some("60"),
            placeholder_by: None,
            secret: false,
            optional: true,
            help_text: Some("The script is stopped and the run reports a failure after this long."),
            options: None,
            default_value: Some("60"),
            inline_format: None,
            group: Some("Script"),
            file_name: None,
            half_width: false,
        },
        FieldDefinition {
            id: "host_key_fingerprint",
            label: "Host Key Fingerprint",
            field_type: FieldType::String,
            placeholder: Some("SHA256:…"),
            placeholder_by: None,
            secret: false,
            optional: true,
            help_text: Some(
                "Leave blank to trust the key each host presents the first time and refuse it if it later changes. Enter a fingerprint (ssh-keygen -lf) to require that key instead.",
            ),
            options: None,
            default_value: None,
            inline_format: None,
            group: Some("Script"),
            file_name: None,
            half_width: false,
        },
    ]);
    fields
}

/// Connection fields shared by both UniFi transports. Only the auth fields differ, so the
/// port/site pair is defined once — a UniFi credential of either type points at the same
/// controller endpoint.
fn unifi_connection_fields() -> Vec<FieldDefinition> {
    vec![
        FieldDefinition {
            id: "site",
            label: "Site",
            field_type: FieldType::String,
            placeholder: Some("default"),
            placeholder_by: None,
            secret: false,
            optional: true,
            help_text: Some(
                "Internal site name, taken from the controller URL (/manage/site/<name>) — not the site's display name. Most installations use 'default'.",
            ),
            options: None,
            default_value: Some("default"),
            inline_format: None,
            group: Some("Connection"),
            file_name: None,
            half_width: true,
        },
        FieldDefinition {
            id: "port",
            label: "Controller Port",
            field_type: FieldType::Port,
            placeholder: Some("443"),
            placeholder_by: None,
            secret: false,
            optional: true,
            help_text: Some(
                "443 for a UniFi OS console (Dream Machine, Cloud Key, Cloud Gateway), 11443 for a self-hosted UniFi OS Server, or 8443 for the legacy self-hosted Network Application. Check the port in your controller's URL — the wrong port fails to connect.",
            ),
            options: None,
            default_value: Some("443"),
            inline_format: None,
            group: Some("Connection"),
            file_name: None,
            half_width: true,
        },
    ]
}

/// Optional, non-secret socket-path field for local container-socket credentials. The
/// `placeholder` shows the runtime's default socket so it differs for Docker vs Podman. Blank ⇒
/// the daemon auto-detects, so a socket credential needs no required config.
fn socket_path_field(
    placeholder: &'static str,
    placeholder_by: &'static [DependentPlaceholder],
    help: &'static str,
) -> FieldDefinition {
    FieldDefinition {
        id: "socket_path",
        label: "Socket Path",
        field_type: FieldType::String,
        placeholder: Some(placeholder),
        placeholder_by: Some(placeholder_by),
        secret: false,
        optional: true,
        help_text: Some(help),
        options: None,
        default_value: None,
        inline_format: None,
        group: Some("Connection"),
        file_name: None,
        half_width: false,
    }
}

/// Shared field definitions for a container-runtime (Docker/Podman) proxy
/// credential. Only the port label/help differ between runtimes; the path and
/// TLS fields are identical because both speak the Docker-compatible API.
fn container_proxy_field_definitions(
    port_label: &'static str,
    port_help: &'static str,
) -> Vec<FieldDefinition> {
    vec![
        FieldDefinition {
            id: "port",
            label: port_label,
            field_type: FieldType::Port,
            placeholder: Some("2375"),
            placeholder_by: None,
            secret: false,
            optional: true,
            help_text: Some(port_help),
            options: None,
            default_value: Some("2375"),
            inline_format: None,
            group: Some("Connection"),
            file_name: None,
            half_width: true,
        },
        FieldDefinition {
            id: "path",
            label: "URL Path Prefix",
            field_type: FieldType::String,
            placeholder: Some("/"),
            placeholder_by: None,
            secret: false,
            optional: true,
            help_text: Some("Optional URL path prefix appended after the port"),
            options: None,
            default_value: None,
            inline_format: None,
            group: Some("Connection"),
            file_name: None,
            half_width: true,
        },
        FieldDefinition {
            id: "ssl_cert",
            label: "SSL Certificate",
            field_type: FieldType::PathOrInline,
            placeholder: Some("-----BEGIN CERTIFICATE-----"),
            placeholder_by: None,
            secret: false,
            optional: true,
            help_text: Some(
                "PEM-encoded client certificate. All three TLS fields (cert, key, CA chain) must be provided together.",
            ),
            options: None,
            default_value: None,
            inline_format: Some(InlineFormat::PemCertificate),
            group: Some("TLS"),
            file_name: Some("cert.pem"),
            half_width: false,
        },
        FieldDefinition {
            id: "ssl_key",
            label: "SSL Private Key",
            field_type: FieldType::SecretPathOrInline,
            placeholder: None,
            placeholder_by: None,
            secret: true,
            optional: true,
            help_text: Some("PEM private key. All three TLS fields must be provided together."),
            options: None,
            default_value: None,
            inline_format: Some(InlineFormat::PemPrivateKey),
            group: Some("TLS"),
            file_name: Some("key.pem"),
            half_width: false,
        },
        FieldDefinition {
            id: "ssl_chain",
            label: "SSL CA Chain",
            field_type: FieldType::PathOrInline,
            placeholder: Some("-----BEGIN CERTIFICATE-----"),
            placeholder_by: None,
            secret: false,
            optional: true,
            help_text: Some(
                "PEM-encoded CA certificate chain. All three TLS fields must be provided together.",
            ),
            options: None,
            default_value: None,
            inline_format: Some(InlineFormat::PemCertificate),
            group: Some("TLS"),
            file_name: Some("ca.pem"),
            half_width: false,
        },
    ]
}

/// Which fields depend on the credential's Daemon OS. Computed from the field definitions, so the
/// form places the Daemon OS picker without its own rule: with the one field that depends on it, or
/// above the first of several. The Scanned Host OS is an ordinary field of the SSH types and sits
/// in its declared place, above Script, because it sets how the script runs as well as its path.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct CredentialOsFields {
    /// Fields that can read a file or socket on the daemon, so depend on the credential's
    /// `daemon_os`: every file-or-value field, a script that can come from the daemon, and any
    /// field whose placeholder follows `daemon_os`.
    pub daemon: Vec<&'static str>,
}

impl CredentialType {
    pub fn os_fields(&self) -> CredentialOsFields {
        let fields = self.field_definitions();
        let follows = |f: &FieldDefinition, id: &str| {
            f.placeholder_by
                .unwrap_or_default()
                .iter()
                .any(|d| d.depends_on == id)
        };
        CredentialOsFields {
            daemon: fields
                .iter()
                .filter(|f| {
                    matches!(
                        f.field_type,
                        FieldType::SecretPathOrInline
                            | FieldType::PathOrInline
                            | FieldType::ScriptSource
                    ) || follows(f, DAEMON_OS_FIELD)
                })
                .map(|f| f.id)
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::credentials::r#impl::types::CredentialTypeDiscriminants;
    use strum::IntoEnumIterator;

    /// An SSH key credential's key, passphrase and a daemon-side script depend on the daemon's OS. A
    /// socket credential depends on it through its placeholder alone.
    #[test]
    fn os_fields_follow_field_types_and_placeholders() {
        let ssh = CredentialTypeDiscriminants::SshKey
            .to_credential_type()
            .os_fields();
        assert_eq!(ssh.daemon, vec!["private_key", "passphrase", "script"]);

        let socket = CredentialTypeDiscriminants::DockerSocket
            .to_credential_type()
            .os_fields();
        assert_eq!(socket.daemon, vec!["socket_path"]);
    }

    /// Every field that can be read from a file names its own example file, so its placeholder
    /// and path errors show a path for that field rather than a generic one.
    #[test]
    fn every_file_capable_field_names_an_example_file() {
        for d in CredentialTypeDiscriminants::iter() {
            for field in d.to_credential_type().field_definitions() {
                if matches!(
                    field.field_type,
                    FieldType::SecretPathOrInline
                        | FieldType::PathOrInline
                        | FieldType::ScriptSource
                ) {
                    assert!(
                        field.file_name.is_some(),
                        "{d:?}.{} has no file_name",
                        field.id
                    );
                }
            }
        }
    }

    /// Every id the form is told to place the picker by is a field the form renders.
    #[test]
    fn os_fields_name_real_fields() {
        for d in CredentialTypeDiscriminants::iter() {
            let ct = d.to_credential_type();
            let ids: Vec<&str> = ct.field_definitions().iter().map(|f| f.id).collect();
            let os = ct.os_fields();
            for id in os.daemon.iter() {
                assert!(ids.contains(id), "{d:?}: {id} is not a field");
            }
        }
    }
}
