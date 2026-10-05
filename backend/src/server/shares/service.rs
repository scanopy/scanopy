use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::{
    auth::service::verify_password,
    shared::{
        events::bus::EventBus,
        services::traits::{CrudService, EventBusService},
        storage::generic::GenericPostgresStorage,
        types::api::ApiError,
        validation::validate_csp_domain,
    },
    shares::r#impl::base::Share,
};

const SHARE_TOKEN_TTL_DAYS: i64 = 30;

pub struct ShareService {
    storage: Arc<GenericPostgresStorage<Share>>,
    event_bus: Arc<EventBus>,
}

impl EventBusService<Share> for ShareService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_site_id(&self, entity: &Share) -> Option<Uuid> {
        Some(entity.base.site_id)
    }

    fn get_organization_id(&self, _entity: &Share) -> Option<Uuid> {
        None
    }
}

impl CrudService<Share> for ShareService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Share>> {
        &self.storage
    }

    fn entity_tag_service(
        &self,
    ) -> Option<&Arc<crate::server::tags::entity_tags::EntityTagService>> {
        None
    }
}

/// Issued access token for a password-protected share
#[derive(Debug)]
pub struct ShareAccessToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

/// JWT claims carried by a share access token
#[derive(Debug, Serialize, Deserialize)]
struct ShareAccessTokenClaims {
    sub: String,
    iat: i64,
    exp: i64,
}

/// Mint an HS256 JWT tied to `share.base.password_hash`.
///
/// Changing the share password regenerates the hash and implicitly
/// invalidates all outstanding tokens.
fn issue_token_impl(share: &Share) -> Result<ShareAccessToken, ApiError> {
    let hash = share
        .base
        .password_hash
        .as_ref()
        .ok_or_else(ApiError::share_token_invalid)?;

    let now = Utc::now();
    let expires_at = now + Duration::days(SHARE_TOKEN_TTL_DAYS);
    let claims = ShareAccessTokenClaims {
        sub: share.id.to_string(),
        iat: now.timestamp(),
        exp: expires_at.timestamp(),
    };

    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(hash.as_bytes()),
    )
    .map_err(|_| ApiError::internal_error("Failed to issue share access token"))?;

    Ok(ShareAccessToken { token, expires_at })
}

/// Validate signature + exp + sub=share.id.
fn verify_token_impl(share: &Share, token: &str) -> Result<(), ApiError> {
    let hash = share
        .base
        .password_hash
        .as_ref()
        .ok_or_else(ApiError::share_token_invalid)?;

    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_required_spec_claims(&["sub", "iat", "exp"]);

    let data = decode::<ShareAccessTokenClaims>(
        token,
        &DecodingKey::from_secret(hash.as_bytes()),
        &validation,
    )
    .map_err(|_| ApiError::share_token_invalid())?;

    if data.claims.sub != share.id.to_string() {
        return Err(ApiError::share_token_invalid());
    }

    Ok(())
}

/// Build the per-share `frame-ancestors` CSP directive.
///
/// - If the org lacks the embed feature, framing is blocked (`'none'`).
/// - Otherwise, `allowed_domains` (filtered through `validate_csp_domain`)
///   is used; empty/None means allow all (`*`).
pub fn build_frame_ancestors(share: &Share, has_embeds_feature: bool) -> String {
    if !has_embeds_feature {
        return "frame-ancestors 'none'".to_string();
    }

    let Some(ref domains) = share.base.allowed_domains else {
        return "frame-ancestors *".to_string();
    };

    let safe_domains: Vec<&String> = domains
        .iter()
        .filter(|d| validate_csp_domain(d).is_ok())
        .collect();

    if safe_domains.is_empty() {
        "frame-ancestors *".to_string()
    } else {
        format!(
            "frame-ancestors {}",
            safe_domains
                .iter()
                .map(|d| d.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

impl ShareService {
    pub fn new(storage: Arc<GenericPostgresStorage<Share>>, event_bus: Arc<EventBus>) -> Self {
        Self { storage, event_bus }
    }

    /// Verify password for a password-protected share
    pub fn verify_share_password(
        &self,
        share: &Share,
        password: &str,
    ) -> Result<(), anyhow::Error> {
        let hash = share
            .base
            .password_hash
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Share does not require a password"))?;

        verify_password(password, hash)
    }

    /// Mint a server-signed access token for a password-protected share.
    pub fn issue_access_token(&self, share: &Share) -> Result<ShareAccessToken, ApiError> {
        issue_token_impl(share)
    }

    /// Verify a share access token presented by the client.
    pub fn verify_access_token(&self, share: &Share, token: &str) -> Result<(), ApiError> {
        verify_token_impl(share, token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::shares::r#impl::base::ShareBase;

    fn make_share(password_hash: Option<&str>) -> Share {
        Share {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: ShareBase {
                password_hash: password_hash.map(|s| s.to_string()),
                ..Default::default()
            },
        }
    }

    const TEST_HASH: &str =
        "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHRzYWx0$0123456789abcdefghijklmnopqrstuvwxyzABCD";

    #[test]
    fn issue_then_verify_valid() {
        let share = make_share(Some(TEST_HASH));
        let issued = issue_token_impl(&share).expect("issue");
        verify_token_impl(&share, &issued.token).expect("verify");
        assert!(issued.expires_at > Utc::now());
    }

    #[test]
    fn verify_expired() {
        let share = make_share(Some(TEST_HASH));
        let past = Utc::now() - Duration::days(1);
        let claims = ShareAccessTokenClaims {
            sub: share.id.to_string(),
            iat: (past - Duration::days(1)).timestamp(),
            exp: past.timestamp(),
        };
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(TEST_HASH.as_bytes()),
        )
        .unwrap();
        let err = verify_token_impl(&share, &token).unwrap_err();
        assert_eq!(
            err.error_code.as_ref().map(|c| c.code()),
            Some("share_token_invalid")
        );
    }

    #[test]
    fn verify_wrong_share_id() {
        let share_a = make_share(Some(TEST_HASH));
        let mut share_b = make_share(Some(TEST_HASH));
        share_b.id = Uuid::new_v4();
        // Both shares share the same hash but have different ids. A token
        // issued for A must not verify against B.
        let issued = issue_token_impl(&share_a).unwrap();
        let err = verify_token_impl(&share_b, &issued.token).unwrap_err();
        assert_eq!(
            err.error_code.as_ref().map(|c| c.code()),
            Some("share_token_invalid")
        );
    }

    #[test]
    fn verify_tampered_signature() {
        let share = make_share(Some(TEST_HASH));
        let issued = issue_token_impl(&share).unwrap();
        // Flip the last character of the signature segment.
        let mut parts: Vec<String> = issued.token.split('.').map(String::from).collect();
        assert_eq!(parts.len(), 3);
        let sig = parts.last_mut().unwrap();
        let last = sig.pop().unwrap();
        sig.push(if last == 'A' { 'B' } else { 'A' });
        let tampered = parts.join(".");
        let err = verify_token_impl(&share, &tampered).unwrap_err();
        assert_eq!(
            err.error_code.as_ref().map(|c| c.code()),
            Some("share_token_invalid")
        );
    }

    #[test]
    fn verify_after_password_change() {
        let share = make_share(Some(TEST_HASH));
        let issued = issue_token_impl(&share).unwrap();

        let mut rotated = share.clone();
        rotated.base.password_hash = Some(
            "$argon2id$v=19$m=19456,t=2,p=1$b3RoZXJzYWx0b3Ro$ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abcd"
                .to_string(),
        );

        let err = verify_token_impl(&rotated, &issued.token).unwrap_err();
        assert_eq!(
            err.error_code.as_ref().map(|c| c.code()),
            Some("share_token_invalid")
        );
    }

    #[test]
    fn verify_no_password_hash() {
        let share = make_share(None);
        let err = verify_token_impl(&share, "any.token.here").unwrap_err();
        assert_eq!(
            err.error_code.as_ref().map(|c| c.code()),
            Some("share_token_invalid")
        );
    }

    #[test]
    fn issue_without_password_hash_errors() {
        let share = make_share(None);
        let err = issue_token_impl(&share).unwrap_err();
        assert_eq!(
            err.error_code.as_ref().map(|c| c.code()),
            Some("share_token_invalid")
        );
    }
}
