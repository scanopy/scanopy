//! A daemon too old to parse a credential type is sent none of it.

use super::DaemonCapability;
use crate::server::credentials::r#impl::capability::CapabilityGap;
use crate::server::credentials::r#impl::types::CredentialTypeDiscriminants;
use crate::server::credentials::service::TypedCredentialMapping;
use semver::Version;

/// Parsing the credential type at all. Every credential relies on it.
///
/// Gated on the `CredentialType` discriminant kept beside each mapping, not the collapsed wire tag,
/// since SnmpCommunity and SnmpV3 share one `Snmp` tag with different gates. Older daemons
/// predate `serde(other)`, so sending them a type they can't parse would fail the whole request.
pub(crate) struct ParsesCredentialType;

/// The credential's type, for its gate.
pub(crate) struct ParsesCredentialTypeGap(CredentialTypeDiscriminants);

impl From<ParsesCredentialTypeGap> for CapabilityGap {
    fn from(_: ParsesCredentialTypeGap) -> Self {
        CapabilityGap::ParsesCredentialType
    }
}

impl DaemonCapability for ParsesCredentialType {
    type Context = ();
    type Gap = ParsesCredentialTypeGap;

    fn gap(_: &(), credential: &TypedCredentialMapping) -> Option<Self::Gap> {
        Some(ParsesCredentialTypeGap(credential.discriminant))
    }

    fn last_without(gap: &Self::Gap) -> Version {
        gap.0.last_unsupported_daemon_version()
    }

    fn withhold(_: &Self::Gap, credential: &mut TypedCredentialMapping) {
        credential.mapping.default_credential = None;
        credential.mapping.ip_overrides.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::discovery::types::warnings::DiscoveryWarning;
    use crate::server::credentials::service::capability::retain_capable;
    use crate::server::credentials::service::mapping_for;
    use std::collections::BTreeMap;
    use uuid::Uuid;

    /// SnmpCommunity and SnmpV3 share the `Snmp` wire tag with different gates, so a 0.16.2 daemon gets
    /// the community credential and loses only the v3 one, with one warning naming it.
    #[test]
    fn only_the_type_the_daemon_cannot_read_is_withheld() {
        let mut map = BTreeMap::new();
        let community_id = Uuid::from_u128(1);
        let v3_id = Uuid::from_u128(2);
        for (id, disc) in [
            (community_id, CredentialTypeDiscriminants::SnmpCommunity),
            (v3_id, CredentialTypeDiscriminants::SnmpV3),
        ] {
            let ct = disc.to_credential_type();
            mapping_for(&mut map, id, &ct).default_credential =
                Some(ct.to_query_payload(None).unwrap());
        }

        let withheld =
            retain_capable::<ParsesCredentialType>(&(), &mut map, Some(&Version::new(0, 16, 2)));

        assert!(map.contains_key(&community_id));
        assert!(!map.contains_key(&v3_id));
        assert!(matches!(
            withheld.as_slice(),
            [DiscoveryWarning::CredentialNeedsNewerDaemon {
                credential_id: Some(id),
                gap: CapabilityGap::ParsesCredentialType,
                ..
            }] if *id == v3_id
        ));
    }
}
