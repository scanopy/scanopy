//! Generic credential resolution and summarization for discovery integrations.

use std::collections::HashMap;
use std::net::IpAddr;
use uuid::Uuid;

use crate::daemon::discovery::types::warnings::DiscoveryWarning;
use crate::server::credentials::r#impl::mapping::{
    CredentialMapping, CredentialQueryPayload, CredentialQueryPayloadDiscriminants,
};
use crate::server::credentials::r#impl::types::OsFamily;
use crate::server::hosts::r#impl::base::Host;

/// One credential to try at an address, and where it came from.
///
/// `user_assigned` is deliberately its own field rather than `credential_id.is_some()`. It used to
/// be derivable that way only because a network default arrived with no id at all; now that it
/// carries one, the two questions have come apart. Conflating them would start reporting every
/// broadcast default that failed to answer — one finding per unresponsive host on a /24 sweep —
/// which is exactly what [`issue_for_attempt`] exists to suppress.
///
/// [`issue_for_attempt`]: crate::daemon::discovery::service::warnings::issue_for_attempt
pub struct ApplicableCredential<'a> {
    pub credential: &'a CredentialQueryPayload,
    /// The stored credential row this came from, where there is one.
    pub credential_id: Option<Uuid>,
    /// Whether the user pinned this credential to this address, as opposed to it being a network
    /// broadcast default. This is what decides whether a failure is a finding.
    pub user_assigned: bool,
}

/// Resolve applicable credentials for a target IP from credential mappings.
///
/// Returns credentials in specificity order: every matching IP override first
/// (in `ip_overrides` declaration order), then the network default as fallback.
/// Each entry includes the credential, its optional server-side ID (for
/// auto-assignment tracking and for naming the record in a warning), and whether
/// the user pinned it here. The caller is expected to try them in order and
/// stop at the first successful probe.
pub fn resolve_credentials_for_ip(
    mapping: &CredentialMapping<CredentialQueryPayload>,
    ip: IpAddr,
) -> Vec<ApplicableCredential<'_>> {
    let mut creds = Vec::new();

    // Every IP-specific override, in declaration order. A host assigned
    // multiple credentials for the same IP should have all of them tried.
    for o in mapping.ip_overrides.iter().filter(|o| o.ip == ip) {
        let cred_id = (o.credential_id != Uuid::nil()).then_some(o.credential_id);
        creds.push(ApplicableCredential {
            credential: &o.credential,
            credential_id: cred_id,
            // A nil id is the pre-`IntegrationTarget` sentinel for "no stored credential", and an
            // override without one is not something the user pinned.
            user_assigned: cred_id.is_some(),
        });
    }

    // Network default as fallback — always tried after overrides when present.
    // The probe loop breaks on first success, so a working override short-circuits
    // the default automatically; the default only actually runs when every
    // override failed (wrong community, auth error, timeout, etc.).
    //
    // Its id comes along where the server sent one. A default that fails is deliberately silent,
    // but a *malformed* one is not (see `issue_for_attempt`), so this is the one path on which a
    // broadcast credential reaches a warning — and it is the path that used to arrive anonymous.
    if let Some(default) = &mapping.default_credential {
        creds.push(ApplicableCredential {
            credential: default,
            credential_id: mapping.default_credential_id,
            user_assigned: false,
        });
    }

    creds
}

/// Summarize credential assignments across discovered hosts, grouped by credential type.
///
/// Builds a credential_id → type lookup from the credential mappings, then groups
/// each host's assignments by type label. Returns type_label → list of "cred_id → ip".
/// Summarize the credential mappings the daemon actually used, grouped by credential type.
///
/// Driven by the mappings themselves (not discovered-host assignments): each mapping's
/// `ip_overrides` cover daemon-host (127.0.0.1) and per-host targets, and `default_credential`
/// covers network broadcast. `hosts` is used only to annotate an override IP with a discovered
/// host name when one matches.
pub fn summarize_credential_assignments(
    hosts: &[(IpAddr, Host)],
    credential_mappings: &[CredentialMapping<CredentialQueryPayload>],
) -> HashMap<String, Vec<String>> {
    let host_name_by_ip: HashMap<IpAddr, String> = hosts
        .iter()
        .map(|(ip, host)| (*ip, host.display_name(&[]).unwrap_or_default()))
        .collect();

    let mut by_type: HashMap<String, Vec<String>> = HashMap::new();
    for mapping in credential_mappings {
        // Per-IP overrides: daemon-host (127.0.0.1) and explicit per-host targets.
        for o in &mapping.ip_overrides {
            let label: String =
                Into::<CredentialQueryPayloadDiscriminants>::into(&o.credential).to_string();
            let target = match host_name_by_ip.get(&o.ip) {
                Some(name) => format!("{} ({})", o.ip, name),
                None => o.ip.to_string(),
            };
            by_type
                .entry(label)
                .or_default()
                .push(format!("{} → {}", o.credential_id, target));
        }
        // Network-broadcast default credential (applies to all hosts on the network).
        if let Some(default) = &mapping.default_credential {
            let label: String =
                Into::<CredentialQueryPayloadDiscriminants>::into(default).to_string();
            by_type
                .entry(label)
                .or_default()
                .push("network default".to_string());
        }
    }

    by_type
}

/// Drop every mapping whose daemon-side files or sockets were declared for another OS, and return
/// one warning per dropped credential.
///
/// A path written for Windows names nothing on a Linux daemon, and the reverse, so using the
/// credential here could only fail at every address with a misleading read error. Credentials
/// that read nothing off this machine are kept whatever their declaration.
pub fn take_os_mismatches(
    mappings: &mut Vec<CredentialMapping<CredentialQueryPayload>>,
    actual: OsFamily,
) -> Vec<DiscoveryWarning> {
    let mut warnings = Vec::new();
    mappings.retain(|m| {
        let Some(payload) = m
            .default_credential
            .as_ref()
            .or_else(|| m.ip_overrides.first().map(|o| &o.credential))
        else {
            return true;
        };
        let Some(declared) = m.daemon_os else {
            return true;
        };
        if declared == actual || !payload.reads_daemon_paths() {
            return true;
        }
        warnings.push(DiscoveryWarning::CredentialDaemonOsMismatch {
            integration: payload.into(),
            credential_id: m
                .default_credential_id
                .or_else(|| m.ip_overrides.first().map(|o| o.credential_id)),
            declared,
            actual,
        });
        false
    });
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::credentials::r#impl::mapping::{
        IpOverride, ResolvableSecret, SnmpQueryCredential, SnmpVersion,
    };

    fn snmp(community: &str) -> CredentialQueryPayload {
        CredentialQueryPayload::Snmp(SnmpQueryCredential {
            version: SnmpVersion::V2c,
            community: ResolvableSecret::Value {
                value: community.to_string(),
            },
            v3: None,
        })
    }

    /// A Windows daemon cannot read a path written for Unix, and the reverse. Only credentials that
    /// read off the daemon are judged: an inline credential works whatever it declares.
    #[test]
    fn a_credential_with_paths_for_another_os_is_dropped_once_with_a_warning() {
        let file_backed = CredentialQueryPayload::Snmp(SnmpQueryCredential {
            version: SnmpVersion::V2c,
            community: ResolvableSecret::FilePath {
                path: "C:\\Scanopy\\community".into(),
            },
            v3: None,
        });
        let windows_file = Uuid::new_v4();
        let windows_inline = Uuid::new_v4();
        let mapping = |payload, id, os| CredentialMapping {
            default_credential: Some(payload),
            default_credential_id: Some(id),
            daemon_os: os,
            ip_overrides: vec![],
        };
        let mut mappings = vec![
            mapping(file_backed.clone(), windows_file, Some(OsFamily::Windows)),
            mapping(snmp("public"), windows_inline, Some(OsFamily::Windows)),
            mapping(file_backed, Uuid::new_v4(), Some(OsFamily::Unix)),
        ];

        let warnings = take_os_mismatches(&mut mappings, OsFamily::Unix);

        assert_eq!(mappings.len(), 2);
        assert!(
            mappings
                .iter()
                .all(|m| m.default_credential_id != Some(windows_file))
        );
        assert_eq!(
            warnings,
            vec![DiscoveryWarning::CredentialDaemonOsMismatch {
                integration: CredentialQueryPayloadDiscriminants::Snmp,
                credential_id: Some(windows_file),
                declared: OsFamily::Windows,
                actual: OsFamily::Unix,
            }]
        );
    }

    fn snmp_community(payload: &CredentialQueryPayload) -> &str {
        match payload {
            CredentialQueryPayload::Snmp(s) => match &s.community {
                ResolvableSecret::Value { value } => value,
                ResolvableSecret::FilePath { .. } => panic!("expected inline community"),
            },
            _ => panic!("expected SNMP payload"),
        }
    }

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    /// A mapping with no default and no overrides, to be filled in by each test.
    ///
    /// Built through `Default` rather than a struct literal so a new field on `CredentialMapping`
    /// does not have to be repeated in every fixture below — which is what adding
    /// `default_credential_id` would otherwise have cost.
    fn mapping() -> CredentialMapping<CredentialQueryPayload> {
        CredentialMapping::default()
    }

    fn over(
        addr: &str,
        community: &str,
        credential_id: Uuid,
    ) -> IpOverride<CredentialQueryPayload> {
        IpOverride {
            ip: ip(addr),
            credential: snmp(community),
            credential_id,
            host_id: None,
            mac_address: None,
        }
    }

    #[test]
    fn resolve_credentials_for_ip_single_override_then_default() {
        let override_id = Uuid::new_v4();
        let default_id = Uuid::new_v4();
        let mapping = CredentialMapping {
            daemon_os: Default::default(),
            default_credential: Some(snmp("netdefault")),
            default_credential_id: Some(default_id),
            ip_overrides: vec![over("10.0.0.5", "secret42", override_id)],
        };

        let creds = resolve_credentials_for_ip(&mapping, ip("10.0.0.5"));

        assert_eq!(creds.len(), 2, "expected override + default fallback");
        assert_eq!(snmp_community(creds[0].credential), "secret42");
        assert_eq!(creds[0].credential_id, Some(override_id));
        assert!(creds[0].user_assigned);
        assert_eq!(snmp_community(creds[1].credential), "netdefault");
        // The default now names its credential too, and is still not something the user pinned
        // here — the two facts have come apart and must stay apart.
        assert_eq!(creds[1].credential_id, Some(default_id));
        assert!(!creds[1].user_assigned);
    }

    #[test]
    fn resolve_credentials_for_ip_multiple_overrides_then_default() {
        // Same IP, two host-scoped overrides (e.g., two SNMP creds assigned to
        // the same host via host_credentials) — every one should be tried in
        // declaration order, then fall back to the network default.
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let mapping = CredentialMapping {
            daemon_os: Default::default(),
            default_credential: Some(snmp("netdefault")),
            default_credential_id: Some(Uuid::new_v4()),
            ip_overrides: vec![
                over("10.0.0.5", "override_a", id_a),
                over("10.0.0.5", "override_b", id_b),
                // A different IP's override must NOT be returned here.
                over("10.0.0.99", "other_host", Uuid::new_v4()),
            ],
        };

        let creds = resolve_credentials_for_ip(&mapping, ip("10.0.0.5"));

        assert_eq!(creds.len(), 3, "two overrides + default fallback");
        assert_eq!(snmp_community(creds[0].credential), "override_a");
        assert_eq!(creds[0].credential_id, Some(id_a));
        assert_eq!(snmp_community(creds[1].credential), "override_b");
        assert_eq!(creds[1].credential_id, Some(id_b));
        assert_eq!(snmp_community(creds[2].credential), "netdefault");
    }

    #[test]
    fn resolve_credentials_for_ip_multiple_overrides_no_default() {
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let mapping = CredentialMapping {
            ip_overrides: vec![
                over("10.0.0.5", "override_a", id_a),
                over("10.0.0.5", "override_b", id_b),
            ],
            ..mapping()
        };

        let creds = resolve_credentials_for_ip(&mapping, ip("10.0.0.5"));

        assert_eq!(creds.len(), 2, "both overrides, no default");
        assert_eq!(snmp_community(creds[0].credential), "override_a");
        assert_eq!(snmp_community(creds[1].credential), "override_b");
    }

    #[test]
    fn resolve_credentials_for_ip_default_only_when_no_matching_override() {
        let default_id = Uuid::new_v4();
        let mapping = CredentialMapping {
            daemon_os: Default::default(),
            default_credential: Some(snmp("netdefault")),
            default_credential_id: Some(default_id),
            ip_overrides: vec![over("10.0.0.99", "other_host", Uuid::new_v4())],
        };

        let creds = resolve_credentials_for_ip(&mapping, ip("10.0.0.5"));

        assert_eq!(creds.len(), 1);
        assert_eq!(snmp_community(creds[0].credential), "netdefault");
        assert_eq!(creds[0].credential_id, Some(default_id));
        assert!(!creds[0].user_assigned);
    }

    #[test]
    fn resolve_credentials_for_ip_default_from_an_older_server_carries_no_id() {
        // A server too old to send `default_credential_id` leaves it absent, and the daemon's own
        // injected "public" fallback has no stored row at all. Both have to stay reportable
        // without one rather than becoming un-nameable.
        let mapping = CredentialMapping {
            default_credential: Some(snmp("public")),
            ..mapping()
        };

        let creds = resolve_credentials_for_ip(&mapping, ip("10.0.0.5"));

        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].credential_id, None);
        assert!(!creds[0].user_assigned);
    }

    #[test]
    fn resolve_credentials_for_ip_returns_empty_when_both_absent() {
        let empty = mapping();
        let creds = resolve_credentials_for_ip(&empty, ip("10.0.0.5"));

        assert!(creds.is_empty());
    }

    #[test]
    fn resolve_credentials_for_ip_treats_nil_credential_id_as_none() {
        // Daemon-injected bootstrap creds have Uuid::nil() — they're not tied
        // to a server-side credential entity and shouldn't leak into
        // assignment tracking. The helper maps nil → None.
        let mapping = CredentialMapping {
            ip_overrides: vec![over("10.0.0.5", "bootstrap", Uuid::nil())],
            ..mapping()
        };

        let creds = resolve_credentials_for_ip(&mapping, ip("10.0.0.5"));

        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].credential_id, None);
        // Nothing was pinned here, so a failure is not a finding.
        assert!(!creds[0].user_assigned);
    }
}
