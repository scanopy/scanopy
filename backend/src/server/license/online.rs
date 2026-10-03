//! Wire contract for online (phone-home) license keys.
//!
//! An online key is a permanent credential: it names the organization and a
//! key version, and carries no plan or expiry. A self-hosted instance presents
//! it to the cloud entitlement endpoint and receives an entitlement, which is
//! an offline-format license key ([`super::types::LicenseClaims`]) minted from
//! the organization's current plan and paid-through date. The instance
//! validates and caches the entitlement exactly as it would an offline key.
//!
//! These types are shared by the cloud side (minting online keys, serving
//! entitlements) and the instance side (requesting and caching entitlements).

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// `sub` claim of an online key. Offline keys and entitlements use
/// `"scanopy-license"`; the subject is how a parser tells the two apart.
pub const ONLINE_KEY_SUBJECT: &str = "scanopy-license-online";

/// Path of the cloud entitlement endpoint, relative to the cloud base URL.
pub const ENTITLEMENT_PATH: &str = "/api/v1/licenses/entitlement";

/// Header carrying the checking-in server's version, as daemons send
/// `X-Daemon-Version`. Absent from servers older than the header.
pub const SERVER_VERSION_HEADER: &str = "X-Server-Version";

/// The version a check-in reported in [`SERVER_VERSION_HEADER`], or `None`
/// when it is missing, not semver, or newer than this server. The cloud runs
/// the newest release, so a higher version is made up; storing one would
/// hold the org's highest-version mark above every real upgrade.
///
/// Read only once the key has been accepted: an unauthenticated caller never
/// gets as far as having its header parsed.
pub fn reported_server_version(raw: Option<&str>) -> Option<semver::Version> {
    let version = semver::Version::parse(raw?.trim()).ok()?;
    let own = semver::Version::parse(crate::server::openapi::SERVER_VERSION)
        .expect("CARGO_PKG_VERSION is semver");
    (version <= own).then_some(version)
}

/// JWT claims of an online license key, signed with the same Ed25519 key as
/// offline keys. There is no `exp`: validity comes from the entitlement the
/// cloud returns, and a leaked key is retired by bumping the organization's
/// key version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnlineKeyClaims {
    /// Subject — always [`ONLINE_KEY_SUBJECT`]
    pub sub: String,
    /// Issuer — always "scanopy"
    pub iss: String,
    /// Issued-at (unix timestamp)
    pub iat: i64,
    /// Organization the key belongs to.
    pub org_id: String,
    /// Must equal the organization's current key version for the cloud to
    /// return an entitlement. Regenerating the key increments it.
    pub key_version: u32,
}

/// Body the instance POSTs to [`ENTITLEMENT_PATH`].
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EntitlementRequest {
    /// The online key configured via `SCANOPY_LICENSE_KEY`.
    pub key: String,
}

/// Data returned inside `ApiResponse` by [`ENTITLEMENT_PATH`].
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EntitlementResponse {
    /// An offline-format license key (claims: [`super::types::LicenseClaims`])
    /// with `org_id` and `plan` set from the organization's current state.
    pub entitlement: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_semver_no_newer_than_this_server_counts_as_reported() {
        let own = crate::server::openapi::SERVER_VERSION;
        let own_version = semver::Version::parse(own).unwrap();
        let newer = semver::Version::new(own_version.major + 1, 0, 0).to_string();

        assert_eq!(reported_server_version(Some(own)), Some(own_version));
        assert_eq!(
            reported_server_version(Some("0.1.0")),
            Some(semver::Version::new(0, 1, 0))
        );
        assert_eq!(reported_server_version(None), None);
        assert_eq!(reported_server_version(Some("not-a-version")), None);
        assert_eq!(reported_server_version(Some(&newer)), None);
    }
}
