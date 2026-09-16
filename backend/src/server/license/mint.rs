//! Minting license keys and entitlements.
//!
//! The one signing path shared by the `license` CLI (hand-issued keys) and the
//! cloud server (keys and entitlements minted from an organization's state).
//! Nothing minted here is stored: every key is rebuilt on demand from the org's
//! plan, `license_paid_through`, and `license_key_version`.

use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use thiserror::Error;

use super::crypto;
use super::online::{ONLINE_KEY_SUBJECT, OnlineKeyClaims};
use super::types::{LicenseClaims, LicensePlan};
use crate::server::organizations::r#impl::base::Organization;

/// `sub` claim of offline keys and entitlements.
pub const LICENSE_SUBJECT: &str = "scanopy-license";

/// `iss` claim of every key and entitlement.
pub const LICENSE_ISSUER: &str = "scanopy";

/// Silent grace window added past the user-visible expiry. Hard-coded —
/// per-tier grace is explicitly out of scope.
pub const GRACE_PERIOD_DAYS: i64 = 7;

/// Days past `license_paid_through` before an org's offline key or
/// entitlement reaches its user-visible expiry, so a renewal paid on its due
/// date reaches an instance before any expiry banner shows.
pub const PAID_THROUGH_BUFFER_DAYS: i64 = 7;

/// Claims of an offline-format license: `exp` is always
/// [`GRACE_PERIOD_DAYS`] past `intended_exp`.
pub fn license_claims(
    now: DateTime<Utc>,
    intended_exp: DateTime<Utc>,
    org_id: Option<String>,
    plan: Option<LicensePlan>,
) -> LicenseClaims {
    LicenseClaims {
        sub: LICENSE_SUBJECT.to_string(),
        iss: LICENSE_ISSUER.to_string(),
        iat: now.timestamp(),
        exp: (intended_exp + Duration::days(GRACE_PERIOD_DAYS)).timestamp(),
        intended_exp: intended_exp.timestamp(),
        org_id,
        plan,
    }
}

pub fn sign_license(
    claims: &LicenseClaims,
    key: &EncodingKey,
) -> jsonwebtoken::errors::Result<String> {
    jsonwebtoken::encode(&Header::new(Algorithm::EdDSA), claims, key)
}

pub fn online_key_claims(now: DateTime<Utc>, org_id: String, key_version: u32) -> OnlineKeyClaims {
    OnlineKeyClaims {
        sub: ONLINE_KEY_SUBJECT.to_string(),
        iss: LICENSE_ISSUER.to_string(),
        iat: now.timestamp(),
        org_id,
        key_version,
    }
}

pub fn sign_online_key(
    claims: &OnlineKeyClaims,
    key: &EncodingKey,
) -> jsonwebtoken::errors::Result<String> {
    jsonwebtoken::encode(&Header::new(Algorithm::EdDSA), claims, key)
}

#[derive(Debug, Error)]
pub enum OnlineKeyError {
    #[error("malformed or badly signed online license key: {0}")]
    Invalid(#[from] jsonwebtoken::errors::Error),
    #[error("not an online license key")]
    WrongSubject,
}

/// Verify an online key's signature and claims. Online keys carry no `exp`,
/// so expiry validation is off; validity comes from the org's current state.
pub fn decode_online_key(
    token: &str,
    key: &DecodingKey,
) -> Result<OnlineKeyClaims, OnlineKeyError> {
    let mut validation = Validation::new(Algorithm::EdDSA);
    validation.set_issuer(&[LICENSE_ISSUER]);
    validation.set_required_spec_claims(&["sub", "iss", "iat"]);
    validation.validate_exp = false;

    let claims = jsonwebtoken::decode::<OnlineKeyClaims>(token, key, &validation)?.claims;
    if claims.sub != ONLINE_KEY_SUBJECT {
        return Err(OnlineKeyError::WrongSubject);
    }
    Ok(claims)
}

#[derive(Debug, Error)]
pub enum MintError {
    #[error("organization is not on a self-hosted license plan")]
    NotLicensed,
    #[error("offline license keys require a plan with air-gapped deployment")]
    OfflineNotIncluded,
    #[error("organization has no paid-through date")]
    NoPaidThrough,
    #[error("license key version is out of range")]
    KeyVersionOutOfRange,
    #[error("failed to sign license: {0}")]
    Signing(#[from] jsonwebtoken::errors::Error),
}

/// Signs keys and entitlements for organizations. Present on the cloud only,
/// built from `ServerConfig::license_signing_key`.
pub struct LicenseIssuer {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl LicenseIssuer {
    pub fn new(encoding: EncodingKey, decoding: DecodingKey) -> Self {
        Self { encoding, decoding }
    }

    /// Issuer that signs with `signing_key_pem` and verifies with the public
    /// key embedded in every Scanopy build.
    pub fn from_pem(signing_key_pem: &str) -> anyhow::Result<Self> {
        Ok(Self::new(
            crypto::encoding_key_from_pem(signing_key_pem)?,
            crypto::decoding_key(),
        ))
    }

    pub fn decode_online_key(&self, token: &str) -> Result<OnlineKeyClaims, OnlineKeyError> {
        decode_online_key(token, &self.decoding)
    }

    /// Mint the entitlement returned to an instance presenting a valid online
    /// key. Skips the air-gap check: entitlements are only ever handed to a
    /// server that phoned home.
    pub fn mint_entitlement(
        &self,
        org: &Organization,
        now: DateTime<Utc>,
    ) -> Result<String, MintError> {
        let plan = licensed_plan(org)?;
        self.mint_paid_through(org, Some(plan), "entitlement", now)
    }

    /// Mint the org's online key. Deterministic: the same `issued_at` and key
    /// version sign to a byte-identical string, so copying the key twice hands
    /// back the same value. `issued_at` comes from
    /// `OrganizationService::license_key_issued_at`, which logs the audit line
    /// when it assigns one; this re-signs an already-issued key, so it logs at
    /// debug rather than repeating that line on every copy.
    pub fn mint_online_key(
        &self,
        org: &Organization,
        issued_at: DateTime<Utc>,
    ) -> Result<String, MintError> {
        let plan = licensed_plan(org)?;
        let key_version = u32::try_from(org.base.license_key_version)
            .map_err(|_| MintError::KeyVersionOutOfRange)?;
        let claims = online_key_claims(issued_at, org.id.to_string(), key_version);
        let token = sign_online_key(&claims, &self.encoding)?;
        tracing::debug!(
            organization_id = %org.id,
            key_type = "online",
            plan = ?plan,
            key_version,
            "Signed online license key"
        );
        Ok(token)
    }

    /// The air-gap check lives here, in the mint path, so it holds whichever
    /// caller asks for an offline key.
    pub fn mint_offline_key(
        &self,
        org: &Organization,
        now: DateTime<Utc>,
    ) -> Result<String, MintError> {
        let plan = org.base.plan.ok_or(MintError::NotLicensed)?;
        if !plan.features().air_gapped_deployment {
            return Err(MintError::OfflineNotIncluded);
        }
        self.mint_paid_through(org, plan.license_plan(), "offline", now)
    }

    fn mint_paid_through(
        &self,
        org: &Organization,
        plan: Option<LicensePlan>,
        key_type: &'static str,
        now: DateTime<Utc>,
    ) -> Result<String, MintError> {
        let paid_through = org
            .base
            .license_paid_through
            .ok_or(MintError::NoPaidThrough)?;
        let intended_exp = paid_through + Duration::days(PAID_THROUGH_BUFFER_DAYS);
        let claims = license_claims(now, intended_exp, Some(org.id.to_string()), plan);
        let token = sign_license(&claims, &self.encoding)?;
        tracing::info!(
            organization_id = %org.id,
            key_type,
            plan = ?plan,
            expires_at = %intended_exp,
            "Minted license key"
        );
        Ok(token)
    }
}

fn licensed_plan(org: &Organization) -> Result<LicensePlan, MintError> {
    org.base
        .plan
        .and_then(|plan| plan.license_plan())
        .ok_or(MintError::NotLicensed)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::server::billing::plans::{
        get_enterprise_plan, get_self_hosted_plus_plan, get_self_hosted_standard_plan,
    };
    use crate::server::billing::types::base::BillingPlan;
    use crate::server::license::key::LicenseKey;
    use crate::server::license::types::LicenseStatus;
    use uuid::Uuid;

    use crate::server::license::crypto::test_keys;

    pub(crate) fn test_decoding_key() -> DecodingKey {
        DecodingKey::from_ed_pem(test_keys::PUBLIC_KEY.as_bytes()).unwrap()
    }

    pub(crate) fn test_issuer() -> LicenseIssuer {
        LicenseIssuer::new(
            crypto::encoding_key_from_pem(test_keys::PRIVATE_KEY).unwrap(),
            test_decoding_key(),
        )
    }

    fn org(plan: BillingPlan, paid_through: Option<DateTime<Utc>>) -> Organization {
        let mut org = Organization {
            id: Uuid::new_v4(),
            ..Default::default()
        };
        org.base.plan = Some(plan);
        org.base.license_paid_through = paid_through;
        org.base.license_key_version = 3;
        org
    }

    #[test]
    fn online_key_round_trips() {
        let issuer = test_issuer();
        let org = org(get_self_hosted_standard_plan(), None);

        let key = issuer.mint_online_key(&org, Utc::now()).unwrap();
        let claims = issuer.decode_online_key(&key).unwrap();

        assert_eq!(claims.org_id, org.id.to_string());
        assert_eq!(claims.key_version, 3);
    }

    #[test]
    fn online_key_is_the_same_string_for_one_issued_at() {
        let issuer = test_issuer();
        let mut org = org(get_self_hosted_standard_plan(), None);
        let issued_at = Utc::now();

        // What "Copy key" does twice: same stamp, same version, same string.
        let first = issuer.mint_online_key(&org, issued_at).unwrap();
        assert_eq!(issuer.mint_online_key(&org, issued_at).unwrap(), first);

        // Regenerating moves both the version and the stamp.
        org.base.license_key_version += 1;
        let regenerated = issuer
            .mint_online_key(&org, issued_at + Duration::seconds(1))
            .unwrap();
        assert_ne!(regenerated, first);
    }

    #[test]
    fn entitlement_validates_with_plan_and_paid_through_expiry() {
        let issuer = test_issuer();
        let paid_through = Utc::now() + Duration::days(30);
        let org = org(get_self_hosted_standard_plan(), Some(paid_through));

        let entitlement = issuer.mint_entitlement(&org, Utc::now()).unwrap();
        let LicenseStatus::Valid(claims) =
            LicenseKey::new(entitlement).validate_with(&test_decoding_key())
        else {
            panic!("entitlement should validate");
        };

        assert_eq!(claims.plan, Some(LicensePlan::Standard));
        assert_eq!(claims.org_id, Some(org.id.to_string()));
        let intended_exp = paid_through + Duration::days(PAID_THROUGH_BUFFER_DAYS);
        assert_eq!(claims.intended_exp, intended_exp.timestamp());
        assert_eq!(
            claims.exp,
            (intended_exp + Duration::days(GRACE_PERIOD_DAYS)).timestamp()
        );
    }

    #[test]
    fn offline_key_requires_air_gapped_plan() {
        let issuer = test_issuer();
        let paid_through = Some(Utc::now() + Duration::days(30));

        let standard = org(get_self_hosted_standard_plan(), paid_through);
        assert!(matches!(
            issuer.mint_offline_key(&standard, Utc::now()),
            Err(MintError::OfflineNotIncluded)
        ));

        let plus = org(get_self_hosted_plus_plan(), paid_through);
        let key = issuer.mint_offline_key(&plus, Utc::now()).unwrap();
        let LicenseStatus::Valid(claims) = LicenseKey::new(key).validate_with(&test_decoding_key())
        else {
            panic!("offline key should validate");
        };
        assert_eq!(claims.plan, Some(LicensePlan::Plus));
    }

    #[test]
    fn keys_refused_for_plans_without_a_license() {
        let issuer = test_issuer();
        let cloud = org(get_enterprise_plan(), Some(Utc::now()));

        assert!(matches!(
            issuer.mint_online_key(&cloud, Utc::now()),
            Err(MintError::NotLicensed)
        ));
        assert!(matches!(
            issuer.mint_entitlement(&cloud, Utc::now()),
            Err(MintError::NotLicensed)
        ));
    }

    #[test]
    fn entitlement_requires_paid_through() {
        let issuer = test_issuer();
        let org = org(get_self_hosted_plus_plan(), None);

        assert!(matches!(
            issuer.mint_entitlement(&org, Utc::now()),
            Err(MintError::NoPaidThrough)
        ));
    }

    #[test]
    fn decode_rejects_garbage_foreign_signatures_and_other_subjects() {
        let issuer = test_issuer();

        assert!(matches!(
            issuer.decode_online_key("not-a-jwt"),
            Err(OnlineKeyError::Invalid(_))
        ));

        // Signed by the test key, verified against the production public key.
        let org = org(get_self_hosted_standard_plan(), None);
        let key = issuer.mint_online_key(&org, Utc::now()).unwrap();
        assert!(matches!(
            decode_online_key(&key, &test_keys::production_decoding_key()),
            Err(OnlineKeyError::Invalid(_))
        ));

        let mut claims = online_key_claims(Utc::now(), org.id.to_string(), 0);
        claims.sub = LICENSE_SUBJECT.to_string();
        let wrong_subject = sign_online_key(
            &claims,
            &crypto::encoding_key_from_pem(test_keys::PRIVATE_KEY).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            issuer.decode_online_key(&wrong_subject),
            Err(OnlineKeyError::WrongSubject)
        ));
    }
}
