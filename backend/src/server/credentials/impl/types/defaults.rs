//! Seeded default credentials.
//!
//! Every organization gets one credential per seeded type when it is created: the local Docker
//! socket, the local Podman socket and the SNMP community `public`. A default is recognized by its content
//! alone. A credential identical to its type's seeded default is that default, and an
//! organization holds at most one per type.

use secrecy::{ExposeSecret, SecretString};

use super::{CredentialType, CredentialTypeDiscriminants, SecretValue};

/// The community the SNMP default sends. Most devices ship with it.
const PUBLIC_COMMUNITY: &str = "public";

/// What an organization's default credential of one type is created with.
#[derive(Debug, Clone)]
pub struct SeededDefault {
    pub name: &'static str,
    pub description: &'static str,
    pub credential_type: CredentialType,
}

impl CredentialTypeDiscriminants {
    /// The credential every organization starts with for this type, if any. Exhaustive, so a new
    /// type has to decide whether it is seeded.
    pub fn seeded_default(&self) -> Option<SeededDefault> {
        match self {
            Self::DockerSocket => Some(SeededDefault {
                name: "Docker socket",
                description: "Added to every new daemon. Reads the local Docker socket at its default path.",
                credential_type: CredentialType::DockerSocket { socket_path: None },
            }),
            Self::PodmanSocket => Some(SeededDefault {
                name: "Podman socket",
                description: "Added to every new daemon. Reads the local Podman socket at its default path.",
                credential_type: CredentialType::PodmanSocket { socket_path: None },
            }),
            Self::SnmpCommunity => Some(SeededDefault {
                name: "SNMP (public)",
                description: "Assigned to every new site. Tries the SNMP community \"public\" on every host, over v2c and then v1.",
                credential_type: CredentialType::SnmpCommunity {
                    community: SecretValue::Inline {
                        value: SecretString::from(PUBLIC_COMMUNITY.to_string()),
                    },
                },
            }),
            Self::SnmpV3
            | Self::Gnmi
            | Self::DockerProxy
            | Self::PodmanProxy
            | Self::UnifiApiKey
            | Self::UnifiLocalAdmin
            | Self::InstantOnAccount
            | Self::SshPassword
            | Self::SshKey
            | Self::SshScript
            | Self::Libvirt
            | Self::WakeOnLan
            | Self::ProxmoxApiToken => None,
        }
    }
}

impl CredentialType {
    /// Whether this credential is identical to its type's seeded default. A blank socket path is
    /// the same as none: both leave the daemon to find its local socket.
    pub fn is_default(&self) -> bool {
        let Some(seeded) = CredentialTypeDiscriminants::from(self).seeded_default() else {
            return false;
        };
        match (self, &seeded.credential_type) {
            (Self::DockerSocket { socket_path }, Self::DockerSocket { .. })
            | (Self::PodmanSocket { socket_path }, Self::PodmanSocket { .. }) => {
                socket_path.as_ref().is_none_or(|p| p.is_blank())
            }
            (Self::SnmpCommunity { community }, Self::SnmpCommunity { community: seeded }) => {
                // Compared on the value, not with `==`, so the redacted sentinel never matches.
                match (community, seeded) {
                    (SecretValue::Inline { value }, SecretValue::Inline { value: expected }) => {
                        value.expose_secret() == expected.expose_secret()
                    }
                    _ => false,
                }
            }
            _ => *self == seeded.credential_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::credentials::r#impl::types::DaemonSocket;
    use strum::IntoEnumIterator;

    #[test]
    fn every_seeded_default_is_a_default() {
        for discriminant in CredentialTypeDiscriminants::iter() {
            if let Some(seeded) = discriminant.seeded_default() {
                assert_eq!(
                    CredentialTypeDiscriminants::from(&seeded.credential_type),
                    discriminant,
                    "{discriminant} seeds a credential of another type"
                );
                assert!(
                    seeded.credential_type.is_default(),
                    "{discriminant}'s seeded credential is not recognized as its default"
                );
            }
        }
    }

    #[test]
    fn a_blank_socket_path_is_still_the_default_and_a_real_path_is_not() {
        let blank = CredentialType::DockerSocket {
            socket_path: Some(serde_json::from_str::<DaemonSocket>("\"  \"").unwrap()),
        };
        assert!(blank.is_default());

        let custom = CredentialType::PodmanSocket {
            socket_path: Some(
                serde_json::from_str::<DaemonSocket>("\"/run/podman/podman.sock\"").unwrap(),
            ),
        };
        assert!(!custom.is_default());
    }

    #[test]
    fn another_community_or_a_file_is_not_the_snmp_default() {
        let other = CredentialType::SnmpCommunity {
            community: SecretValue::Inline {
                value: SecretString::from("private".to_string()),
            },
        };
        assert!(!other.is_default());

        let file = CredentialType::SnmpCommunity {
            community: SecretValue::FilePath {
                path: PUBLIC_COMMUNITY.into(),
            },
        };
        assert!(!file.is_default());
    }
}
