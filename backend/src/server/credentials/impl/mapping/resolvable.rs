//! Credential values that are either inline or a file path on the daemon host, how they resolve,
//! and how they render on the daemon's startup banner.

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use utoipa::ToSchema;

/// A credential field that could not be turned into the value the wire needs.
///
/// Typed rather than a formatted string so a caller can tell "the configuration is wrong" from
/// "the network is wrong" without matching on message text — the same reason
/// `snmp::queries::is_desync` downcasts `snmp2::Error` rather than reading its `Display`. Until
/// this existed the two were the same `anyhow::Error` to every probe, and a community string typed
/// into a file-path field was reported at every host in the scan as an unreachable device
/// (GH #668).
///
/// Only a *read* failure. A temp-file write failing while handing a value to a client library that
/// wants a path is our disk, not the operator's credential, and telling them to re-enter it would
/// send them to fix something that is not broken.
///
/// `Display` reproduces the previous message verbatim, so daemon logs are unchanged.
#[derive(Debug, thiserror::Error)]
#[error("Failed to read {field} from {path} for {label}: {source}")]
pub struct UnresolvableCredential {
    /// The field on the credential, e.g. `community` or `auth_password`.
    pub field: String,
    /// The path that could not be read. Never a secret — a path is what makes this debuggable.
    pub path: String,
    /// Which credential type asked, e.g. `SNMP`.
    pub label: String,
    pub source: std::io::Error,
}

impl UnresolvableCredential {
    fn read(field: &str, path: &str, label: &str) -> Result<String, anyhow::Error> {
        std::fs::read_to_string(path).map_err(|source| {
            anyhow::Error::new(Self {
                field: field.to_string(),
                path: path.to_string(),
                label: label.to_string(),
                source,
            })
        })
    }
}

/// Whether this error is a credential field we could not read, wherever in the chain it sits.
///
/// Chain-walking rather than a top-level downcast because every caller adds context on the way up:
/// `create_session` wraps, the probe wraps again, and a check on the outermost error would see
/// none of them.
pub fn is_unresolvable_credential(error: &anyhow::Error) -> bool {
    error
        .chain()
        .any(|cause| cause.is::<UnresolvableCredential>())
}

/// Non-secret value — inline or file path. Daemon can log freely.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(tag = "mode")]
pub enum ResolvableValue {
    Value { value: String },
    FilePath { path: String },
}

/// Secret value — inline or file path. Daemon wraps resolved value in Secret<String>.
/// Never logged in plaintext.
///
/// Custom Deserialize accepts both the current tagged-enum format
/// (`{"mode":"Value","value":"..."}`) and legacy plain strings (`"********"`)
/// from pre-v0.15.0 discovery_type JSONB. Legacy strings deserialize as
/// `Value { value: string }`.
#[derive(Clone, Serialize, Eq, PartialEq, Hash, ToSchema)]
#[serde(tag = "mode")]
pub enum ResolvableSecret {
    Value { value: String },
    FilePath { path: String },
}

/// Redacts the secret rather than deriving `Debug`, so *holding* one of these is enough to be
/// safe in a log line. `SnmpV3Params` and `SnmpQueryCredential` hand-write redacting impls for
/// the same reason; doing it here as well means a payload that forgets to — as
/// `UnifiQueryCredential` did, and as the Instant On payload would have — cannot leak. A file
/// path is not a secret and stays legible, which is what makes a misconfigured path debuggable.
impl std::fmt::Debug for ResolvableSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Value { value } => f
                .debug_struct("Value")
                .field("value", &format_args!("******** ({} chars)", value.len()))
                .finish(),
            Self::FilePath { path } => f.debug_struct("FilePath").field("path", path).finish(),
        }
    }
}

impl<'de> Deserialize<'de> for ResolvableSecret {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match &value {
            serde_json::Value::String(s) => Ok(ResolvableSecret::Value { value: s.clone() }),
            serde_json::Value::Object(_) => {
                #[derive(Deserialize)]
                #[serde(tag = "mode")]
                enum Tagged {
                    Value { value: String },
                    FilePath { path: String },
                }
                let tagged: Tagged =
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?;
                Ok(match tagged {
                    Tagged::Value { value } => ResolvableSecret::Value { value },
                    Tagged::FilePath { path } => ResolvableSecret::FilePath { path },
                })
            }
            _ => Err(serde::de::Error::custom(
                "expected string or object for ResolvableSecret",
            )),
        }
    }
}

impl ResolvableValue {
    /// Resolve to a string value. FilePath variant reads from disk.
    pub fn resolve(&self, field_name: &str, label: &str) -> Result<String, anyhow::Error> {
        match self {
            Self::Value { value } => Ok(value.clone()),
            Self::FilePath { path } => {
                tracing::info!("Read {} from {} for {}", field_name, path, label);
                UnresolvableCredential::read(field_name, path, label)
            }
        }
    }

    /// Read FilePath from disk and return Value. Value variants pass through.
    pub fn resolve_to_value(&self, field_name: &str, label: &str) -> Result<Self, anyhow::Error> {
        match self {
            Self::Value { .. } => Ok(self.clone()),
            Self::FilePath { path } => {
                tracing::info!("Read {} from {} for {}", field_name, path, label);
                let contents = UnresolvableCredential::read(field_name, path, label)?;
                Ok(Self::Value { value: contents })
            }
        }
    }

    /// Resolve to a filesystem path. FilePath returns the path directly.
    /// Value writes content to a temp file (caller must hold the handle to keep it alive).
    pub fn resolve_to_path(
        &self,
        field_name: &str,
        label: &str,
    ) -> Result<(PathBuf, Option<NamedTempFile>), anyhow::Error> {
        match self {
            Self::FilePath { path } => Ok((PathBuf::from(path), None)),
            Self::Value { value } => {
                let mut tmp = NamedTempFile::new().map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to create temp file for {} ({}): {}",
                        field_name,
                        label,
                        e
                    )
                })?;
                tmp.write_all(value.as_bytes()).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to write {} to temp file for {}: {}",
                        field_name,
                        label,
                        e
                    )
                })?;
                tmp.flush()?;
                let path = tmp.path().to_path_buf();
                Ok((path, Some(tmp)))
            }
        }
    }
}

impl ResolvableSecret {
    /// Resolve to a Secret<String>. FilePath variant reads from disk.
    pub fn resolve(
        &self,
        field_name: &str,
        label: &str,
    ) -> Result<redact::Secret<String>, anyhow::Error> {
        match self {
            Self::Value { value } => Ok(redact::Secret::from(value.clone())),
            Self::FilePath { path } => {
                tracing::info!("Read {} (********) from {} for {}", field_name, path, label);
                let contents = UnresolvableCredential::read(field_name, path, label)?;
                Ok(redact::Secret::from(contents))
            }
        }
    }

    /// Read FilePath from disk and return Value. Value variants pass through.
    pub fn resolve_to_value(&self, field_name: &str, label: &str) -> Result<Self, anyhow::Error> {
        match self {
            Self::Value { .. } => Ok(self.clone()),
            Self::FilePath { path } => {
                tracing::info!("Read {} (********) from {} for {}", field_name, path, label);
                let contents = UnresolvableCredential::read(field_name, path, label)?;
                Ok(Self::Value { value: contents })
            }
        }
    }

    /// Resolve to a filesystem path. FilePath returns the path directly.
    /// Value writes content to a temp file (caller must hold the handle to keep it alive).
    pub fn resolve_to_path(
        &self,
        field_name: &str,
        label: &str,
    ) -> Result<(PathBuf, Option<NamedTempFile>), anyhow::Error> {
        match self {
            Self::FilePath { path } => Ok((PathBuf::from(path), None)),
            Self::Value { value } => {
                let mut tmp = NamedTempFile::new().map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to create temp file for {} ({}): {}",
                        field_name,
                        label,
                        e
                    )
                })?;
                tmp.write_all(value.as_bytes()).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to write {} to temp file for {}: {}",
                        field_name,
                        label,
                        e
                    )
                })?;
                tmp.flush()?;
                let path = tmp.path().to_path_buf();
                Ok((path, Some(tmp)))
            }
        }
    }
}

// ============================================================================
// Banner display types for credential logging
// ============================================================================

/// One line in the credential banner.
pub struct BannerField {
    pub label: &'static str,
    pub value: BannerFieldValue,
}

pub enum BannerFieldValue {
    /// Non-secret inline value — show directly (e.g., port "2376", version "v2c")
    Plain(String),
    /// Long inline value — show "<inline, N chars>" instead of dumping content
    InlineSummary(usize),
    /// Inline secret — show "******** (N chars)"
    RedactedInline(usize),
    /// File path that exists — show "successfully read from /path"
    FileOk(String),
    /// File path that doesn't exist — show "failed to read from /path"
    FileFailed(String),
}

impl BannerFieldValue {
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::FileFailed(_))
    }
}

impl std::fmt::Display for BannerFieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Plain(v) => write!(f, "{}", v),
            Self::InlineSummary(len) => write!(f, "<inline, {} chars>", len),
            Self::RedactedInline(len) => write!(f, "******** ({} chars)", len),
            Self::FileOk(path) => write!(f, "successfully read from {}", path),
            Self::FileFailed(path) => write!(f, "failed to read from {}", path),
        }
    }
}

impl ResolvableValue {
    pub fn banner_value(&self) -> BannerFieldValue {
        match self {
            Self::Value { value } => {
                if value.len() > 64 {
                    BannerFieldValue::InlineSummary(value.len())
                } else {
                    BannerFieldValue::Plain(value.clone())
                }
            }
            Self::FilePath { path } => {
                if Path::new(path).exists() {
                    BannerFieldValue::FileOk(path.clone())
                } else {
                    BannerFieldValue::FileFailed(path.clone())
                }
            }
        }
    }
}

impl ResolvableSecret {
    pub fn banner_value(&self) -> BannerFieldValue {
        match self {
            Self::Value { value } => BannerFieldValue::RedactedInline(value.len()),
            Self::FilePath { path } => {
                if Path::new(path).exists() {
                    BannerFieldValue::FileOk(path.clone())
                } else {
                    BannerFieldValue::FileFailed(path.clone())
                }
            }
        }
    }
}
