use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer, Serialize, Serializer, ser::SerializeMap};
use utoipa::ToSchema;

use super::CredentialType;

/// Sentinel value used by the `redact_secret` serializer.
/// The frontend also hardcodes this value for show/hide toggle logic.
pub const REDACTED_SECRET_SENTINEL: &str = "********";

/// Serializer that redacts the secret value
fn redact_secret<S>(_secret: &SecretString, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(REDACTED_SECRET_SENTINEL)
}

/// Secret value that can be either inline content or a file path on the daemon host.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "mode")]
pub enum SecretValue {
    Inline {
        #[serde(serialize_with = "redact_secret")]
        #[schema(value_type = String)]
        value: SecretString,
    },
    FilePath {
        path: String,
    },
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
    Inline { value: String },
    FilePath { path: String },
}

/// Wrapper that serializes `SecretValue` with secrets exposed (for DB storage).
struct StorageSecretValue<'a>(&'a SecretValue);

impl Serialize for StorageSecretValue<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        match self.0 {
            SecretValue::Inline { value } => {
                map.serialize_entry("mode", "Inline")?;
                map.serialize_entry("value", value.expose_secret())?;
            }
            SecretValue::FilePath { path } => {
                map.serialize_entry("mode", "FilePath")?;
                map.serialize_entry("path", path)?;
            }
        }
        map.end()
    }
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

/// Newtype that serializes `CredentialType` with all secret fields exposed.
/// Use this for database storage only — the default `Serialize` impl redacts secrets.
pub struct StorageCredentialType<'a>(pub &'a CredentialType);

impl Serialize for StorageCredentialType<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            CredentialType::SnmpV2c { community } => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("type", "SnmpV2c")?;
                map.serialize_entry("community", &StorageSecretValue(community))?;
                map.end()
            }
            CredentialType::DockerProxy {
                port,
                path,
                ssl_cert,
                ssl_key,
                ssl_chain,
            } => {
                let mut map = serializer.serialize_map(Some(6))?;
                map.serialize_entry("type", "DockerProxy")?;
                map.serialize_entry("port", port)?;
                map.serialize_entry("path", path)?;
                map.serialize_entry("ssl_cert", ssl_cert)?;
                match ssl_key {
                    Some(sv) => map.serialize_entry("ssl_key", &StorageSecretValue(sv))?,
                    None => map.serialize_entry("ssl_key", &None::<()>)?,
                }
                map.serialize_entry("ssl_chain", ssl_chain)?;
                map.end()
            }
        }
    }
}
