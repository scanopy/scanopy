//! Which part of each credential a daemon is sent, by what that daemon can use.
//!
//! A daemon too old to parse a credential type gets none of it. One that parses the type but lacks
//! something a credential's configuration relies on — an option, a kind of target — gets the rest.
//! Each such requirement is one [`DaemonCapability`]: it says whether a credential relies on it,
//! from which release daemons have it, and how to adapt the credential for a daemon without it.
//! [`retain_capable`] applies one to every mapping built for a dispatch and records a
//! `CredentialNeedsNewerDaemon` warning naming exactly what was withheld.
//!
//! `build_all_credential_mappings` runs [`ParsesCredentialType`] first, so a credential withheld
//! whole is gone before any later capability looks at it, and gets that one warning only.

use super::TypedCredentialMapping;
use crate::daemon::discovery::types::warnings::DiscoveryWarning;
use crate::server::credentials::r#impl::base::Credential;
use crate::server::credentials::r#impl::capability::CapabilityGap;
use crate::server::daemons::r#impl::version::last_without_credential_wire;
use semver::Version;
use std::collections::BTreeMap;
use strum::IntoDiscriminant;
use uuid::Uuid;

mod container_bridge;
mod parses_type;
mod snmp_v3;

pub use container_bridge::DaemonPlacement;
pub(crate) use container_bridge::{
    ContainerBridgeContext, TargetsContainerBridgeWithCredentials, retain_overrides_scanned_here,
    retain_reachable_overrides,
};
pub(crate) use parses_type::ParsesCredentialType;
pub(crate) use snmp_v3::{UsesSnmpV3DesPrivacy, UsesSnmpV3SecurityOptions};

/// Something a credential's configuration can rely on that older daemons lack.
pub(crate) trait DaemonCapability {
    /// What the rule reads, built once per dispatch. `()` for a rule about the credential alone.
    type Context;
    /// What a daemon without it misses of one credential. Its `Into` is the one place naming its
    /// `CapabilityGap` variant, so a capability can only report its own.
    type Gap: Into<CapabilityGap>;

    /// Read-only: whether this credential relies on the capability here, and what is missed.
    fn gap(ctx: &Self::Context, credential: &TypedCredentialMapping) -> Option<Self::Gap>;

    /// The last daemon release without it, for this gap (see `daemons/impl/version.rs`).
    fn last_without(gap: &Self::Gap) -> Version;

    /// Adapt the mapping for a daemon without it, acting on exactly what `gap` names, so the
    /// warning and what was withheld cannot disagree.
    fn withhold(gap: &Self::Gap, credential: &mut TypedCredentialMapping);
}

/// Whether a daemon at `daemon_version` has what a gap needs. A daemon that never reported a
/// version is credited with the credential wire itself and nothing newer, so it still gets the
/// types that shipped with that wire.
fn daemon_has(last_without: &Version, daemon_version: Option<&Version>) -> bool {
    match daemon_version {
        Some(v) => v > last_without,
        None => last_without <= &last_without_credential_wire(),
    }
}

/// Adapt every mapping that relies on `C` for a daemon at `daemon_version` without it, and say
/// what each lost. A mapping left with nothing to send is dropped: it would otherwise still be
/// sent, and ordered as a site default.
pub(crate) fn retain_capable<C: DaemonCapability>(
    ctx: &C::Context,
    mappings: &mut BTreeMap<Uuid, TypedCredentialMapping>,
    daemon_version: Option<&Version>,
) -> Vec<DiscoveryWarning> {
    let mut withheld = Vec::new();
    for (credential_id, typed) in mappings.iter_mut() {
        let Some(gap) = C::gap(ctx, typed) else {
            continue;
        };
        let last_without = C::last_without(&gap);
        if daemon_has(&last_without, daemon_version) {
            continue;
        }
        C::withhold(&gap, typed);
        withheld.push(DiscoveryWarning::CredentialNeedsNewerDaemon {
            integration: typed.discriminant.wire_discriminant(),
            credential_id: Some(*credential_id),
            gap: gap.into(),
            last_unsupported_daemon_version: last_without,
            daemon_version: daemon_version.cloned(),
        });
    }
    mappings.retain(|_, typed| {
        typed.mapping.default_credential.is_some() || !typed.mapping.ip_overrides.is_empty()
    });
    withheld
}

/// Why a daemon cannot use `credential` at all, for refusing to save a configuration that could
/// never run on it. Checks only the capabilities that depend on the credential alone; one that
/// depends on where the daemon runs (a bridge on its own host) is decided at dispatch.
pub(crate) fn refusal_for_daemon(
    credential: &Credential,
    daemon_name: &str,
    daemon_version: Option<&Version>,
) -> Option<String> {
    let mut typed = TypedCredentialMapping::new(credential.base.credential_type.discriminant());
    // The payload only matters to the SNMPv3 check; a credential that runs over a login is judged
    // on its type, which needs no login to read.
    typed.mapping.default_credential = credential.base.credential_type.to_query_payload(None);

    let missing = missing_gap::<ParsesCredentialType>(&(), &typed, daemon_version)
        .or_else(|| missing_gap::<UsesSnmpV3SecurityOptions>(&(), &typed, daemon_version))
        .or_else(|| missing_gap::<UsesSnmpV3DesPrivacy>(&(), &typed, daemon_version))?;
    let (gap, last_without) = missing;
    Some(format!(
        "The \"{}\" credential {}. That needs a daemon newer than {last_without}, and \"{daemon_name}\" \
         runs {}. Upgrade the daemon or choose another credential.",
        credential.base.name,
        gap.describe(),
        daemon_version
            .map(Version::to_string)
            .unwrap_or_else(|| "an unknown version".to_string()),
    ))
}

/// The gap `C` reports for `credential` when the daemon lacks it.
fn missing_gap<C: DaemonCapability>(
    ctx: &C::Context,
    credential: &TypedCredentialMapping,
    daemon_version: Option<&Version>,
) -> Option<(CapabilityGap, Version)> {
    let gap = C::gap(ctx, credential)?;
    let last_without = C::last_without(&gap);
    (!daemon_has(&last_without, daemon_version)).then(|| (gap.into(), last_without))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::credentials::r#impl::types::CredentialTypeDiscriminants;

    fn v(major: u64, minor: u64, patch: u64) -> Version {
        Version::new(major, minor, patch)
    }

    #[test]
    fn a_daemon_has_a_capability_only_above_its_last_release_without() {
        assert!(!daemon_has(&v(0, 18, 0), Some(&v(0, 18, 0))));
        assert!(daemon_has(&v(0, 18, 0), Some(&v(0, 18, 1))));
    }

    /// A daemon with no version still receives what shipped with the credential wire, as it did
    /// before capabilities, and nothing newer.
    #[test]
    fn a_versionless_daemon_is_credited_with_the_credential_wire_only() {
        let wire = last_without_credential_wire();
        assert!(daemon_has(&wire, None));
        assert!(!daemon_has(
            &v(wire.major, wire.minor, wire.patch + 1),
            None
        ));
    }

    fn credential(disc: CredentialTypeDiscriminants) -> Credential {
        let mut credential = Credential::default();
        credential.base.name = "Lab".to_string();
        credential.base.credential_type = disc.to_credential_type();
        credential
    }

    #[test]
    fn a_save_is_refused_only_for_a_daemon_that_cannot_use_the_credential() {
        let ssh = credential(CredentialTypeDiscriminants::SshKey);
        let too_old = v(0, 17, 18);
        assert!(refusal_for_daemon(&ssh, "lab", Some(&too_old)).is_some());
        assert!(refusal_for_daemon(&ssh, "lab", Some(&v(0, 17, 19))).is_none());
    }

    /// An option of a type the daemon otherwise parses refuses the save the same way: the
    /// credential could never run there.
    #[test]
    fn a_save_is_refused_for_a_daemon_too_old_for_des_privacy() {
        use crate::server::credentials::r#impl::types::{
            CredentialType, SecretValue, SnmpV3AuthProtocol, SnmpV3PrivProtocol, SnmpV3Security,
        };
        let secret = || SecretValue::Inline {
            value: secrecy::SecretString::from("lab-secret-12345".to_string()),
        };
        let mut des = Credential::default();
        des.base.name = "Lab".to_string();
        des.base.credential_type = CredentialType::SnmpV3 {
            security_name: "scanopy".into(),
            security: SnmpV3Security::AuthPriv {
                auth_protocol: SnmpV3AuthProtocol::Sha1,
                auth_password: secret(),
                priv_protocol: SnmpV3PrivProtocol::Des,
                priv_password: secret(),
            },
            context_name: None,
        };

        assert!(refusal_for_daemon(&des, "lab", Some(&v(0, 19, 6))).is_some());
        assert!(refusal_for_daemon(&des, "lab", Some(&v(0, 19, 7))).is_none());
    }
}
