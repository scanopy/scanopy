//! What a daemon too old for part of a credential's configuration misses.
//!
//! A credential reaches a daemon only as far as that daemon can use it. One that can't parse the
//! credential type gets none of it; one that can parse the type but lacks something this
//! credential's configuration relies on (an option, a kind of target) gets what it can use. Each
//! way a daemon can fall short is one variant here, carrying what that shortfall withheld, so the
//! scan's warning names it rather than calling the daemon "too old" for the whole credential.
//!
//! Deciding the shortfall lives in `credentials/service/capability`; this is the record of it.

use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use utoipa::ToSchema;

/// What a daemon missed of one credential because it is too old for part of it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", content = "details")]
pub enum CapabilityGap {
    /// The daemon cannot parse this credential type, so none of it was sent.
    #[schema(title = "ParsesCredentialType")]
    ParsesCredentialType,
    /// The credential uses SNMPv3 options the daemon cannot parse, so none of it was sent.
    #[schema(title = "UsesSnmpV3SecurityOptions")]
    UsesSnmpV3SecurityOptions(SnmpV3SecurityOptionsGap),
    /// The credential targets containers on Docker or Podman bridge networks of the daemon's own
    /// host, which the daemon does not try credentials at. Those addresses were not sent; the
    /// rest of the credential was.
    #[schema(title = "TargetsContainerBridgeWithCredentials")]
    TargetsContainerBridgeWithCredentials(ContainerBridgeGap),
}

impl CapabilityGap {
    /// The shortfall as a clause following "the {credential} credential", for refusals the server
    /// writes in English. The UI renders its own from the variant.
    pub fn describe(&self) -> String {
        match self {
            Self::ParsesCredentialType => "cannot be read by this daemon".to_string(),
            Self::UsesSnmpV3SecurityOptions(gap) => format!(
                "uses {}, which this daemon cannot read",
                gap.options
                    .iter()
                    .map(SnmpV3SecurityOption::label)
                    .collect::<Vec<_>>()
                    .join(" and ")
            ),
            Self::TargetsContainerBridgeWithCredentials(gap) => format!(
                "targets containers on bridge networks ({}), which this daemon does not try \
                 credentials at",
                gap.addresses
                    .iter()
                    .map(IpAddr::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

/// The SNMPv3 options a credential uses that the daemon is too old to parse.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct SnmpV3SecurityOptionsGap {
    /// The options the credential uses that this daemon rejects.
    pub options: Vec<SnmpV3SecurityOption>,
}

/// One SNMPv3 option added after v3 first shipped (AuthPriv with SHA and AES only). The levels
/// and MD5 need a daemon newer than 0.18.0, DES one newer than 0.19.6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum SnmpV3SecurityOption {
    /// The `NoAuthNoPriv` security level.
    NoAuthNoPriv,
    /// The `AuthNoPriv` security level.
    AuthNoPriv,
    /// MD5 authentication.
    Md5Auth,
    /// DES privacy.
    DesPrivacy,
}

impl SnmpV3SecurityOption {
    pub fn label(&self) -> &'static str {
        match self {
            Self::NoAuthNoPriv => "the noAuthNoPriv security level",
            Self::AuthNoPriv => "the authNoPriv security level",
            Self::Md5Auth => "MD5 authentication",
            Self::DesPrivacy => "DES privacy",
        }
    }
}

/// A credential's addresses on bridge networks of the daemon's own host.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct ContainerBridgeGap {
    /// The credential's target addresses on bridge networks of the daemon's own host.
    #[schema(value_type = Vec<String>)]
    pub addresses: Vec<IpAddr>,
}
