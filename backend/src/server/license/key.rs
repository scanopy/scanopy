use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

use super::crypto::decoding_key;
use super::mint::{LICENSE_ISSUER, LICENSE_SUBJECT, decode_online_key};
use super::online::OnlineKeyClaims;
use super::types::{LicenseClaims, LicenseStatus};

/// Which kind of key `SCANOPY_LICENSE_KEY` holds, told apart by the `sub` claim.
#[derive(Debug, Clone, strum_macros::EnumDiscriminants)]
#[strum_discriminants(
    derive(Serialize, Deserialize, utoipa::ToSchema),
    serde(rename_all = "lowercase"),
    doc = "Kind of license key configured, as reported by the public config endpoint."
)]
pub enum LicenseKeyType {
    /// Carries its own plan and expiry and is validated locally. Anything that
    /// is not a verified online key takes this path, so a malformed key
    /// validates to `Invalid` exactly as it did before online keys existed.
    Offline,
    /// A permanent credential the instance exchanges for an entitlement at
    /// each check-in.
    Online(OnlineKeyClaims),
}

/// A Scanopy license key: the raw signed JWT configured via
/// `SCANOPY_LICENSE_KEY`, or an entitlement the cloud returned for an online
/// key. This is the one authoritative type for turning that string into
/// license state. (Distinct from the Ed25519 verification keypair in
/// `crypto.rs`.)
///
/// The string stays opaque until [`LicenseKey::validate`] runs; an invalid key
/// is still a `LicenseKey` (it validates to `Invalid` and locks the server).
/// Obtain the key that actually applies to a deployment via
/// `ServerConfig::effective_license_key`, which returns `None` on cloud so a
/// stray key can never validate, lock, or reconfigure a cloud deployment.
#[derive(Debug, Clone)]
pub struct LicenseKey(String);

/// Signature, issuer and required-claim checks shared by both key types.
/// Expiry is checked by the caller, to tell `Expired` from `Invalid`.
fn validation(required_claims: &[&str]) -> Validation {
    let mut validation = Validation::new(Algorithm::EdDSA);
    validation.set_issuer(&[LICENSE_ISSUER]);
    validation.set_required_spec_claims(required_claims);
    validation.validate_exp = false;
    validation
}

impl LicenseKey {
    pub fn new(raw: String) -> Self {
        Self(raw)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Classify the key. Only a correctly signed token with the online subject
    /// is an online key; everything else takes the offline path.
    pub fn key_type(&self) -> LicenseKeyType {
        match decode_online_key(&self.0, &decoding_key()) {
            Ok(claims) => LicenseKeyType::Online(claims),
            Err(_) => LicenseKeyType::Offline,
        }
    }

    /// Verify the JWT (EdDSA signature, `scanopy` issuer, required claims) and
    /// classify it. Expiry is checked manually to distinguish `Expired` (valid
    /// signature, past `exp`) from `Invalid` (bad signature/malformed).
    pub fn validate(&self) -> LicenseStatus {
        self.validate_with(&decoding_key())
    }

    /// [`LicenseKey::validate`] against a caller-supplied verification key.
    pub fn validate_with(&self, key: &DecodingKey) -> LicenseStatus {
        match jsonwebtoken::decode::<LicenseClaims>(
            &self.0,
            key,
            &validation(&["sub", "iss", "iat", "exp"]),
        ) {
            Ok(token_data) => {
                if token_data.claims.sub != LICENSE_SUBJECT {
                    return LicenseStatus::Invalid("Invalid subject claim".to_string());
                }

                let now = chrono::Utc::now().timestamp();
                if token_data.claims.exp < now {
                    LicenseStatus::Expired(token_data.claims)
                } else {
                    LicenseStatus::Valid(token_data.claims)
                }
            }
            Err(e) => LicenseStatus::Invalid(e.to_string()),
        }
    }

    /// Validate an entitlement the cloud returned for the online key `online`:
    /// the same checks as an offline key, and the entitlement must name the
    /// online key's organization.
    pub fn validate_entitlement(&self, online: &OnlineKeyClaims) -> LicenseStatus {
        match self.validate() {
            LicenseStatus::Valid(claims) | LicenseStatus::Expired(claims)
                if claims.org_id.as_deref() != Some(online.org_id.as_str()) =>
            {
                LicenseStatus::Invalid("Entitlement is for a different organization".to_string())
            }
            status => status,
        }
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::super::crypto::test_signing::sign;
    use super::super::online::{ONLINE_KEY_SUBJECT, OnlineKeyClaims};
    use super::super::types::{LicenseClaims, LicensePlan};
    use super::{LICENSE_SUBJECT, LicenseKey};

    pub const ORG_ID: &str = "0b7c1f7e-3d7a-4c43-9a55-6f0f2d1e8a11";
    const DAY: i64 = 86_400;

    pub fn online_claims(org_id: &str) -> OnlineKeyClaims {
        OnlineKeyClaims {
            sub: ONLINE_KEY_SUBJECT.to_string(),
            iss: "scanopy".to_string(),
            iat: chrono::Utc::now().timestamp(),
            org_id: org_id.to_string(),
            key_version: 1,
        }
    }

    pub fn online_key(org_id: &str) -> LicenseKey {
        LicenseKey::new(sign(&online_claims(org_id)))
    }

    /// A signed offline-format token whose user-visible expiry is
    /// `intended_days` from now, with the usual 7-day grace window after it.
    pub fn license_token(
        org_id: Option<&str>,
        plan: Option<LicensePlan>,
        intended_days: i64,
    ) -> String {
        let now = chrono::Utc::now().timestamp();
        let intended_exp = now + intended_days * DAY;
        sign(&LicenseClaims {
            sub: LICENSE_SUBJECT.to_string(),
            iss: "scanopy".to_string(),
            iat: now,
            exp: intended_exp + 7 * DAY,
            intended_exp,
            org_id: org_id.map(str::to_string),
            plan,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{ORG_ID, license_token, online_claims, online_key};
    use super::*;
    use crate::server::license::types::LicensePlan;

    #[test]
    fn online_subject_selects_online_path() {
        let LicenseKeyType::Online(claims) = online_key(ORG_ID).key_type() else {
            panic!("online key classified as offline");
        };
        assert_eq!(claims.org_id, ORG_ID);
    }

    #[test]
    fn offline_key_keeps_offline_path() {
        let key = LicenseKey::new(license_token(None, Some(LicensePlan::Standard), 30));
        assert!(matches!(key.key_type(), LicenseKeyType::Offline));
        assert!(matches!(key.validate(), LicenseStatus::Valid(_)));
    }

    #[test]
    fn garbage_takes_offline_path_and_is_invalid() {
        let key = LicenseKey::new("not-a-jwt".to_string());
        assert!(matches!(key.key_type(), LicenseKeyType::Offline));
        assert!(matches!(key.validate(), LicenseStatus::Invalid(_)));
    }

    #[test]
    fn online_key_is_not_usable_as_an_offline_key() {
        // No `exp` and the wrong subject: the key only works through check-in.
        assert!(matches!(
            online_key(ORG_ID).validate(),
            LicenseStatus::Invalid(_)
        ));
    }

    #[test]
    fn entitlement_for_the_keys_org_is_valid() {
        let entitlement = LicenseKey::new(license_token(Some(ORG_ID), Some(LicensePlan::Plus), 30));
        assert!(matches!(
            entitlement.validate_entitlement(&online_claims(ORG_ID)),
            LicenseStatus::Valid(_)
        ));
    }

    #[test]
    fn entitlement_for_another_org_is_invalid() {
        let entitlement = LicenseKey::new(license_token(
            Some("5d2b3c4e-0000-4000-8000-000000000000"),
            Some(LicensePlan::Plus),
            30,
        ));
        assert!(matches!(
            entitlement.validate_entitlement(&online_claims(ORG_ID)),
            LicenseStatus::Invalid(_)
        ));
    }

    #[test]
    fn entitlement_without_org_is_invalid() {
        let entitlement = LicenseKey::new(license_token(None, Some(LicensePlan::Plus), 30));
        assert!(matches!(
            entitlement.validate_entitlement(&online_claims(ORG_ID)),
            LicenseStatus::Invalid(_)
        ));
    }

    #[test]
    fn lapsed_entitlement_for_the_keys_org_is_expired() {
        let entitlement = LicenseKey::new(license_token(Some(ORG_ID), None, -30));
        assert!(matches!(
            entitlement.validate_entitlement(&online_claims(ORG_ID)),
            LicenseStatus::Expired(_)
        ));
    }
}
