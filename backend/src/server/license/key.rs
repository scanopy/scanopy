use jsonwebtoken::{Algorithm, DecodingKey, Validation};

use super::crypto::decoding_key;
use super::mint::{LICENSE_ISSUER, LICENSE_SUBJECT};
use super::types::{LicenseClaims, LicenseStatus};
use crate::server::billing::plans::plan_for_license;
use crate::server::billing::types::base::BillingPlan;

/// A Scanopy license key: the raw signed JWT configured via
/// `SCANOPY_LICENSE_KEY`. This is the one authoritative type for turning that
/// string into license state — validating its signature/expiry and resolving
/// the plan it entitles. (Distinct from the Ed25519 verification keypair in
/// `crypto.rs`.)
///
/// The string stays opaque until [`LicenseKey::validate`] runs; an invalid key
/// is still a `LicenseKey` (it validates to `Invalid` and locks the server).
/// Obtain the key that actually applies to a deployment via
/// `ServerConfig::effective_license_key`, which returns `None` on cloud so a
/// stray key can never validate, lock, or reconfigure a cloud deployment.
#[derive(Debug, Clone)]
pub struct LicenseKey(String);

impl LicenseKey {
    pub fn new(raw: String) -> Self {
        Self(raw)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Verify the JWT (EdDSA signature, `scanopy` issuer, required claims) and
    /// classify it. Expiry is checked manually to distinguish `Expired` (valid
    /// signature, past `exp`) from `Invalid` (bad signature/malformed).
    pub fn validate(&self) -> LicenseStatus {
        self.validate_with(&decoding_key())
    }

    /// [`LicenseKey::validate`] against a caller-supplied verification key.
    pub fn validate_with(&self, key: &DecodingKey) -> LicenseStatus {
        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.set_issuer(&[LICENSE_ISSUER]);
        validation.set_required_spec_claims(&["sub", "iss", "iat", "exp"]);
        validation.validate_exp = false;

        match jsonwebtoken::decode::<LicenseClaims>(&self.0, key, &validation) {
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

    /// The self-hosted plan this key entitles: the licensed tier for a valid
    /// key, else the community default (an invalid/expired key resolves to the
    /// default and also locks the server via the license-guard middleware).
    pub fn self_hosted_plan(&self) -> BillingPlan {
        match self.validate() {
            LicenseStatus::Valid(claims) => plan_for_license(&claims),
            _ => BillingPlan::default(),
        }
    }
}
