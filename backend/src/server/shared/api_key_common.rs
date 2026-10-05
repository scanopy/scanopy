use std::net::IpAddr;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use strum::IntoStaticStr;
use uuid::Uuid;

use std::fmt::Display;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    shared::{
        entities::{ChangeTriggersTopologyStaleness, Entity as EntityEnum},
        events::{
            bus::EventBus,
            traits::{AuthScope, Event},
            types::AuthOperation,
        },
        services::traits::{CrudService, EventBusService},
        storage::{
            lock::{DEFAULT_LOCK_TIMEOUT, LockKey},
            traits::Entity,
        },
        types::api::ApiError,
    },
};

/// The type of API key, used for prefix generation and routing
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    IntoStaticStr,
    Serialize,
    Deserialize,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyType {
    Daemon,
    User,
}

impl ApiKeyType {
    /// Get the prefix for this key type
    pub fn prefix(&self) -> &'static str {
        match self {
            ApiKeyType::Daemon => "scp_d_",
            ApiKeyType::User => "scp_u_",
        }
    }

    /// Detect key type from a key string
    /// Returns (key_type, is_prefixed) - legacy keys without prefix default to Daemon
    pub fn from_key(key: &str) -> (Self, bool) {
        if key.starts_with("scp_u_") {
            (ApiKeyType::User, true)
        } else if key.starts_with("scp_d_") {
            (ApiKeyType::Daemon, true)
        } else {
            // Legacy key without prefix - assume daemon
            (ApiKeyType::Daemon, false)
        }
    }
}

/// Common behavior for both daemon and user API keys.
/// This trait provides shared functionality for key validation, expiration checking,
/// and access to common fields.
pub trait ApiKeyCommon {
    /// The type of API key (Daemon or User) - used for prefix generation
    const KEY_TYPE: ApiKeyType;

    // Getters
    fn key(&self) -> &str;
    fn name(&self) -> &str;
    fn is_enabled(&self) -> bool;
    fn expires_at(&self) -> Option<DateTime<Utc>>;
    fn last_used(&self) -> Option<DateTime<Utc>>;
    fn tags(&self) -> &[Uuid];

    // Setters
    fn set_key(&mut self, key: String);
    fn set_is_enabled(&mut self, enabled: bool);
    fn set_last_used(&mut self, time: Option<DateTime<Utc>>);

    /// Refresh the server-held plaintext copy after a rotation, for key types
    /// that keep one (ServerPoll daemon keys, so the poller can present the key).
    /// Default: no-op — user and DaemonPoll keys never store plaintext.
    fn refresh_stored_plaintext(&mut self, _plaintext: &str) {}

    /// Check if the key has expired
    fn is_expired(&self) -> bool {
        self.expires_at()
            .map(|exp| Utc::now() > exp)
            .unwrap_or(false)
    }

    /// Check if the key is valid (enabled and not expired)
    fn is_valid(&self) -> bool {
        self.is_enabled() && !self.is_expired()
    }
}

/// Base62 alphabet for key generation
const BASE62_ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Hash an API key using SHA-256
pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

/// Generate a new API key with prefix and its hash for storage
/// Returns (plaintext, hashed) - plaintext is shown once to user, hash is stored in DB
pub fn generate_api_key_for_storage(key_type: ApiKeyType) -> (String, String) {
    let plaintext = generate_prefixed_api_key(key_type);
    let hashed = hash_api_key(&plaintext);
    (plaintext, hashed)
}

/// Generate a new API key without prefix (for legacy compatibility)
/// Returns (plaintext, hashed)
#[allow(dead_code)]
pub fn generate_api_key_for_storage_legacy() -> (String, String) {
    let plaintext = Uuid::new_v4().simple().to_string();
    let hashed = hash_api_key(&plaintext);
    (plaintext, hashed)
}

/// Generate a prefixed API key with 32 characters of base62 random data
fn generate_prefixed_api_key(key_type: ApiKeyType) -> String {
    let random_part = generate_base62_string(32);
    format!("{}{}", key_type.prefix(), random_part)
}

/// Generate a random base62 string of the specified length
fn generate_base62_string(length: usize) -> String {
    let mut rng = rand::rng();
    (0..length)
        .map(|_| {
            let idx = rng.random_range(0..BASE62_ALPHABET.len());
            BASE62_ALPHABET[idx] as char
        })
        .collect()
}

/// Check if an API key is valid for authentication
/// Returns Ok(()) if valid, or an appropriate error
pub fn check_key_validity<K: Entity + ApiKeyCommon>(key: &K) -> Result<(), ApiError> {
    if key.is_expired() {
        // Use specific error for daemon keys for backward compatibility with daemons < v0.13.5
        return Err(match K::KEY_TYPE {
            ApiKeyType::Daemon => ApiError::daemon_api_key_expired(),
            ApiKeyType::User => ApiError::entity_expired::<K>(),
        });
    }
    if !key.is_enabled() {
        // Use specific error for daemon keys for backward compatibility with daemons < v0.13.5
        return Err(match K::KEY_TYPE {
            ApiKeyType::Daemon => ApiError::daemon_api_key_disabled(),
            ApiKeyType::User => ApiError::entity_disabled::<K>(),
        });
    }
    Ok(())
}

/// Trait for API key service operations shared between daemon and user API keys.
/// Provides a default implementation for `rotate_key` that uses `validate_access`
/// for authorization checks.
#[async_trait]
pub trait ApiKeyService: CrudService<Self::Key> + EventBusService<Self::Key> {
    /// The API key type this service manages
    type Key: ApiKeyCommon
        + Entity
        + Clone
        + Send
        + Sync
        + Default
        + Display
        + Into<EntityEnum>
        + ChangeTriggersTopologyStaleness<Self::Key>;

    /// Get a reference to the event bus
    fn api_key_event_bus(&self) -> &Arc<EventBus>;

    /// Validate that the user has access to perform operations on this key.
    /// - For daemon keys: user must have access to the key's site
    /// - For user keys: user must own the key
    fn validate_access(&self, key: &Self::Key, entity: &AuthenticatedEntity) -> Result<()>;

    /// Rotate the API key, generating a new key value.
    /// Uses `validate_access` to verify the user can rotate this key.
    async fn rotate_key(
        &self,
        api_key_id: Uuid,
        ip_address: IpAddr,
        user_agent: Option<String>,
        entity: AuthenticatedEntity,
    ) -> Result<String> {
        // Serialize concurrent rotations of one key: without this, last
        // write wins and the losing caller is handed a plaintext that
        // authenticates nothing. Error paths release via Drop.
        let lock_guard = self
            .storage()
            .session_lock(LockKey::ApiKey(api_key_id), DEFAULT_LOCK_TIMEOUT)
            .await?;

        let mut api_key = self
            .get_by_id(&api_key_id)
            .await?
            .ok_or_else(|| anyhow!("API key '{}' not found", api_key_id))?;

        // Validate access before rotating
        self.validate_access(&api_key, &entity)?;

        // Generate new key with correct prefix based on key type
        let (plaintext, hashed) = generate_api_key_for_storage(Self::Key::KEY_TYPE);
        api_key.set_key(hashed);
        // Keep the server-held plaintext in sync for ServerPoll keys, or the poller
        // would present the old (now invalid) key after a rotation.
        api_key.refresh_stored_plaintext(&plaintext);

        // Publish auth event for audit trail
        self.api_key_event_bus()
            .publish(Event::new(
                AuthScope {
                    user_id: entity.user_id(),
                    organization_id: entity.organization_id(),
                    ip_address,
                    user_agent,
                },
                AuthOperation::RotateKey {
                    api_key_id,
                    key_type: Self::Key::KEY_TYPE,
                },
                entity.clone(),
            ))
            .await?;

        // Update the key in storage
        self.update(&mut api_key, entity).await?;

        lock_guard.release().await?;
        Ok(plaintext)
    }
}
