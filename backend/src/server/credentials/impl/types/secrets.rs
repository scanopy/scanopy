use std::cell::Cell;

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use utoipa::ToSchema;

/// Sentinel emitted in place of a secret when secret-exposure is off.
/// The frontend also hardcodes this value for show/hide toggle logic.
pub const REDACTED_SECRET_SENTINEL: &str = "********";

thread_local! {
    /// When set, secret-bearing serializers emit the real value instead of the
    /// redacted sentinel. Off by default so any stray serialization — API
    /// responses, logs, events — is redacted; the storage layer opts in via
    /// [`ExposeSecretsGuard`] for the duration of a single DB write.
    static EXPOSE_SECRETS: Cell<bool> = const { Cell::new(false) };
}

/// RAII guard that exposes secrets on the current thread until dropped. Wrap it
/// around a synchronous serialize call only (never hold it across `.await`).
/// Restores the prior state on drop, so nesting is safe.
pub struct ExposeSecretsGuard {
    prev: bool,
}

impl ExposeSecretsGuard {
    pub fn new() -> Self {
        let prev = EXPOSE_SECRETS.with(|e| e.replace(true));
        Self { prev }
    }
}

impl Default for ExposeSecretsGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ExposeSecretsGuard {
    fn drop(&mut self) {
        EXPOSE_SECRETS.with(|e| e.set(self.prev));
    }
}

/// Serialize a secret: the real value when secret-exposure is active (storage
/// writes, via [`ExposeSecretsGuard`]), otherwise the redacted sentinel — the
/// default for API responses and logs.
pub fn serialize_secret_value<S>(secret: &SecretString, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if EXPOSE_SECRETS.with(Cell::get) {
        serializer.serialize_str(secret.expose_secret())
    } else {
        serializer.serialize_str(REDACTED_SECRET_SENTINEL)
    }
}

/// Secret value that can be either inline content or a file path on the daemon host.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "mode")]
pub enum SecretValue {
    #[schema(title = "Inline")]
    Inline {
        /// The secret itself. Write-only — reads return a redacted placeholder.
        #[serde(serialize_with = "serialize_secret_value")]
        #[schema(value_type = String)]
        value: SecretString,
    },
    #[schema(title = "FilePath")]
    FilePath {
        /// Path to a file on the daemon host holding the secret.
        path: String,
    },
}

/// Compares inline secrets by their exposed value; file paths by path.
/// `SecretString` intentionally omits `PartialEq`, so this is implemented
/// explicitly rather than derived.
impl PartialEq for SecretValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SecretValue::Inline { value: a }, SecretValue::Inline { value: b }) => {
                a.expose_secret() == b.expose_secret()
            }
            (SecretValue::FilePath { path: a }, SecretValue::FilePath { path: b }) => a == b,
            _ => false,
        }
    }
}

impl SecretValue {
    /// Returns true if this secret value contains the redacted sentinel.
    pub fn is_redacted_sentinel(&self) -> bool {
        match self {
            SecretValue::Inline { value } => value.expose_secret() == REDACTED_SECRET_SENTINEL,
            SecretValue::FilePath { .. } => false,
        }
    }
}

/// Non-secret value that can be inline content or a file path on daemon host.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(tag = "mode")]
pub enum FileOrInline {
    #[schema(title = "Inline")]
    Inline {
        /// The value itself.
        value: String,
    },
    #[schema(title = "FilePath")]
    FilePath {
        /// Path to a file on the daemon host holding the value.
        path: String,
    },
}

/// Deserialize `Option<FileOrInline>`, normalizing empty inline/path values to `None`.
/// Prevents empty-string values (e.g. `{"mode":"Inline","value":""}`) from being treated as present.
pub fn deserialize_optional_file_or_inline<'de, D>(
    deserializer: D,
) -> Result<Option<FileOrInline>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<FileOrInline>::deserialize(deserializer)?;
    Ok(opt.and_then(|v| match &v {
        FileOrInline::Inline { value } if value.trim().is_empty() => None,
        FileOrInline::FilePath { path } if path.trim().is_empty() => None,
        _ => Some(v),
    }))
}

/// Deserialize `Option<SecretValue>`, normalizing empty inline/path values to `None`.
pub fn deserialize_optional_secret_value<'de, D>(
    deserializer: D,
) -> Result<Option<SecretValue>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<SecretValue>::deserialize(deserializer)?;
    Ok(opt.and_then(|v| match &v {
        SecretValue::Inline { value } if value.expose_secret().trim().is_empty() => None,
        SecretValue::FilePath { path } if path.trim().is_empty() => None,
        _ => Some(v),
    }))
}
