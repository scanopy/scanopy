//! SNMPv3 security levels and MD5, which daemons parse only from 0.18.1.
//!
//! SNMPv3 shipped in 0.17.0 as AuthPriv over SHA and AES. 0.18.1 added the `security_level` field,
//! the AuthNoPriv and NoAuthNoPriv levels, and MD5 authentication. A daemon from 0.17.0 to 0.18.0
//! still reads an AuthPriv credential over SHA and AES, ignoring the `security_level` key it does
//! not know, so that credential is sent to it unchanged. It cannot read the other levels (missing
//! required fields) or MD5 (unknown variant), and because the payload's tag is the familiar
//! `Snmp`, `serde(other)` does not catch the error: it fails the whole request. Those credentials
//! are withheld from it whole.
//!
//! DES privacy came later and fails the same way on every daemon up to 0.19.6 (unknown variant).
//! A capability carries one release, so DES has its own, [`UsesSnmpV3DesPrivacy`]. Both report
//! through the same gap, so a daemon is told only the options it is itself too old for: 0.18.0
//! is told MD5 and DES, 0.19.0 is told DES.

use super::DaemonCapability;
use crate::server::credentials::r#impl::capability::{
    CapabilityGap, SnmpV3SecurityOption, SnmpV3SecurityOptionsGap,
};
use crate::server::credentials::r#impl::mapping::CredentialQueryPayload;
use crate::server::credentials::r#impl::types::{
    CredentialTypeDiscriminants, SnmpV3AuthProtocol, SnmpV3PrivProtocol, SnmpV3Security,
};
use crate::server::credentials::service::TypedCredentialMapping;
use semver::Version;

/// An SNMPv3 credential using anything a 0.18.0 daemon cannot parse: a security level other
/// than AuthPriv, MD5 authentication, or DES privacy.
pub(crate) struct UsesSnmpV3SecurityOptions;

/// An SNMPv3 credential using DES privacy, which daemons up to 0.19.6 cannot parse.
pub(crate) struct UsesSnmpV3DesPrivacy;

impl From<SnmpV3SecurityOptionsGap> for CapabilityGap {
    fn from(gap: SnmpV3SecurityOptionsGap) -> Self {
        CapabilityGap::UsesSnmpV3SecurityOptions(gap)
    }
}

/// The options added after 0.18.0 that `security` uses.
fn options_of<S>(security: &SnmpV3Security<S>) -> Vec<SnmpV3SecurityOption> {
    let md5 = |auth: &SnmpV3AuthProtocol| {
        matches!(auth, SnmpV3AuthProtocol::Md5).then_some(SnmpV3SecurityOption::Md5Auth)
    };
    match security {
        SnmpV3Security::NoAuthNoPriv => vec![SnmpV3SecurityOption::NoAuthNoPriv],
        SnmpV3Security::AuthNoPriv { auth_protocol, .. } => {
            std::iter::once(SnmpV3SecurityOption::AuthNoPriv)
                .chain(md5(auth_protocol))
                .collect()
        }
        SnmpV3Security::AuthPriv {
            auth_protocol,
            priv_protocol,
            ..
        } => md5(auth_protocol)
            .into_iter()
            .chain(
                matches!(priv_protocol, SnmpV3PrivProtocol::Des)
                    .then_some(SnmpV3SecurityOption::DesPrivacy),
            )
            .collect(),
    }
}

/// Every option added after 0.18.0 that any payload of `credential` uses, each once.
fn options_used(credential: &TypedCredentialMapping) -> Vec<SnmpV3SecurityOption> {
    if credential.discriminant != CredentialTypeDiscriminants::SnmpV3 {
        return Vec::new();
    }
    let mapping = &credential.mapping;
    let mut options: Vec<SnmpV3SecurityOption> = Vec::new();
    let payloads = mapping
        .default_credential
        .iter()
        .chain(mapping.ip_overrides.iter().map(|o| &o.credential));
    for payload in payloads {
        if let CredentialQueryPayload::Snmp(query) = payload
            && let Some(v3) = &query.v3
        {
            for option in options_of(&v3.security) {
                if !options.contains(&option) {
                    options.push(option);
                }
            }
        }
    }
    options
}

/// The payload's tag is the familiar `Snmp`, so a daemon that cannot parse one option fails the
/// whole request. The credential is withheld whole.
fn withhold_whole(credential: &mut TypedCredentialMapping) {
    credential.mapping.default_credential = None;
    credential.mapping.ip_overrides.clear();
}

impl DaemonCapability for UsesSnmpV3SecurityOptions {
    type Context = ();
    type Gap = SnmpV3SecurityOptionsGap;

    fn gap(_: &(), credential: &TypedCredentialMapping) -> Option<Self::Gap> {
        let options = options_used(credential);
        (!options.is_empty()).then_some(SnmpV3SecurityOptionsGap { options })
    }

    fn last_without(_: &Self::Gap) -> Version {
        Version::new(0, 18, 0)
    }

    fn withhold(_: &Self::Gap, credential: &mut TypedCredentialMapping) {
        withhold_whole(credential);
    }
}

impl DaemonCapability for UsesSnmpV3DesPrivacy {
    type Context = ();
    type Gap = SnmpV3SecurityOptionsGap;

    /// DES alone: a daemon this capability reaches is newer than 0.18.0, or
    /// [`UsesSnmpV3SecurityOptions`] would already have withheld the credential, so it has
    /// every other option.
    fn gap(_: &(), credential: &TypedCredentialMapping) -> Option<Self::Gap> {
        options_used(credential)
            .contains(&SnmpV3SecurityOption::DesPrivacy)
            .then(|| SnmpV3SecurityOptionsGap {
                options: vec![SnmpV3SecurityOption::DesPrivacy],
            })
    }

    fn last_without(_: &Self::Gap) -> Version {
        Version::new(0, 19, 6)
    }

    fn withhold(_: &Self::Gap, credential: &mut TypedCredentialMapping) {
        withhold_whole(credential);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::credentials::r#impl::mapping::{ResolvableSecret, SnmpQueryCredential};
    use crate::server::credentials::r#impl::types::snmp::{
        SnmpV3Params, SnmpV3PrivProtocol, SnmpVersion,
    };
    use serde::Deserialize;

    /// The v3 parameters exactly as a 0.18.0 daemon reads them, from
    /// `git show v0.18.0:backend/src/server/credentials/impl/types/snmp.rs`. Secrets are left
    /// untyped: only their presence matters to whether the struct parses.
    #[allow(dead_code)]
    #[derive(Deserialize)]
    struct V0_18_0Params {
        security_name: String,
        auth_protocol: V0_18_0AuthProtocol,
        auth_password: serde_json::Value,
        priv_protocol: V0_18_0PrivProtocol,
        priv_password: serde_json::Value,
        #[serde(default)]
        context_name: Option<String>,
    }

    #[derive(Deserialize)]
    enum V0_18_0AuthProtocol {
        Sha1,
        Sha256,
    }

    #[derive(Deserialize)]
    enum V0_18_0PrivProtocol {
        Aes128,
        Aes256,
    }

    fn secret() -> ResolvableSecret {
        ResolvableSecret::Value {
            value: "s".to_string(),
        }
    }

    fn every_security() -> Vec<SnmpV3Security<ResolvableSecret>> {
        let auths = [
            SnmpV3AuthProtocol::Md5,
            SnmpV3AuthProtocol::Sha1,
            SnmpV3AuthProtocol::Sha256,
        ];
        let privs = [
            SnmpV3PrivProtocol::Aes128,
            SnmpV3PrivProtocol::Aes256,
            SnmpV3PrivProtocol::Des,
        ];
        let mut all = vec![SnmpV3Security::NoAuthNoPriv];
        for auth_protocol in auths {
            all.push(SnmpV3Security::AuthNoPriv {
                auth_protocol,
                auth_password: secret(),
            });
            for priv_protocol in privs {
                all.push(SnmpV3Security::AuthPriv {
                    auth_protocol,
                    auth_password: secret(),
                    priv_protocol,
                    priv_password: secret(),
                });
            }
        }
        all
    }

    fn mapping_for(
        security: SnmpV3Security<ResolvableSecret>,
    ) -> (TypedCredentialMapping, SnmpV3Params) {
        let params = SnmpV3Params {
            security_name: "scanopy".to_string(),
            security,
            context_name: None,
        };
        let mut typed = TypedCredentialMapping::new(CredentialTypeDiscriminants::SnmpV3);
        typed.mapping.default_credential =
            Some(CredentialQueryPayload::Snmp(SnmpQueryCredential {
                version: SnmpVersion::V3,
                community: secret(),
                v3: Some(params.clone()),
            }));
        (typed, params)
    }

    /// The gap is checked against what a 0.18.0 daemon actually parses rather than against a
    /// restated table: every combination of level, authentication and privacy protocol is sent
    /// to that daemon's struct, and the credential is withheld exactly when it fails to parse.
    /// A protocol added later fails here until it is classified.
    #[test]
    fn withheld_exactly_when_a_0_18_0_daemon_cannot_parse_it() {
        for security in every_security() {
            let (typed, params) = mapping_for(security.clone());
            let wire = serde_json::to_value(&params).unwrap();
            let parses = serde_json::from_value::<V0_18_0Params>(wire).is_ok();
            let gap = UsesSnmpV3SecurityOptions::gap(&(), &typed);
            assert_eq!(
                gap.is_none(),
                parses,
                "{security:?}: a 0.18.0 daemon {} it, but the capability {}",
                if parses { "parses" } else { "cannot parse" },
                if gap.is_none() {
                    "sends it"
                } else {
                    "withholds it"
                }
            );
        }
    }

    /// DES is the one option a daemon between 0.18.1 and 0.19.6 lacks. It is withheld from that
    /// daemon with a warning naming DES alone, whatever else the credential uses, and sent to a
    /// newer one. A credential without DES is not this capability's business at any version.
    #[test]
    fn des_is_withheld_from_a_daemon_up_to_0_19_6_and_named_alone() {
        use crate::daemon::discovery::types::warnings::DiscoveryWarning;
        use crate::server::credentials::service::capability::retain_capable;
        use std::collections::BTreeMap;
        use uuid::Uuid;

        let with_priv = |auth_protocol, priv_protocol| SnmpV3Security::AuthPriv {
            auth_protocol,
            auth_password: secret(),
            priv_protocol,
            priv_password: secret(),
        };
        let des = Uuid::new_v4();
        let aes = Uuid::new_v4();
        let mappings = || {
            BTreeMap::from([
                (
                    des,
                    mapping_for(with_priv(SnmpV3AuthProtocol::Md5, SnmpV3PrivProtocol::Des)).0,
                ),
                (
                    aes,
                    mapping_for(with_priv(
                        SnmpV3AuthProtocol::Md5,
                        SnmpV3PrivProtocol::Aes256,
                    ))
                    .0,
                ),
            ])
        };

        let mut sent = mappings();
        let warnings =
            retain_capable::<UsesSnmpV3DesPrivacy>(&(), &mut sent, Some(&Version::new(0, 19, 6)));
        assert_eq!(sent.keys().collect::<Vec<_>>(), vec![&aes]);
        match warnings.as_slice() {
            [
                DiscoveryWarning::CredentialNeedsNewerDaemon {
                    credential_id,
                    gap: CapabilityGap::UsesSnmpV3SecurityOptions(gap),
                    ..
                },
            ] => {
                assert_eq!(*credential_id, Some(des));
                assert_eq!(gap.options, vec![SnmpV3SecurityOption::DesPrivacy]);
            }
            other => panic!("expected one warning for the DES credential, got {other:?}"),
        }

        let mut sent = mappings();
        let warnings =
            retain_capable::<UsesSnmpV3DesPrivacy>(&(), &mut sent, Some(&Version::new(0, 19, 7)));
        assert_eq!(sent.len(), 2);
        assert!(warnings.is_empty());
    }

    #[test]
    fn a_non_v3_credential_never_relies_on_it() {
        let ct = CredentialTypeDiscriminants::SnmpCommunity.to_credential_type();
        let mut typed = TypedCredentialMapping::new(CredentialTypeDiscriminants::SnmpCommunity);
        typed.mapping.default_credential = Some(ct.to_query_payload(None).unwrap());
        assert!(UsesSnmpV3SecurityOptions::gap(&(), &typed).is_none());
    }
}
