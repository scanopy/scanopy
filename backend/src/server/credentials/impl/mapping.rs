//! Generic credential mapping for discovery dispatch.
//!
//! The mapping types define how credentials are resolved per-IP during discovery.
//! `CredentialMapping<T>` is generic over the query credential type.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use utoipa::ToSchema;
use uuid::Uuid;

// Re-export type-specific types so external imports don't break
pub use super::types::docker_proxy::DockerProxyQueryCredential;
pub use super::types::snmp::{
    SnmpCredentialMapping, SnmpCredentialMappingExposed, SnmpIpOverrideExposed,
    SnmpQueryCredential, SnmpQueryCredentialExposed, SnmpVersion,
};

// ============================================================================
// Generic Credential Mapping
// ============================================================================

/// Generic credential mapping: a default credential for the network
/// plus per-IP overrides for specific hosts.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct CredentialMapping<T> {
    #[serde(default)]
    pub default_credential: Option<T>,
    #[serde(default)]
    pub ip_overrides: Vec<IpOverride<T>>,
}

/// IP-specific credential override
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct IpOverride<T> {
    pub ip: IpAddr,
    pub credential: T,
    /// Credential ID for tracking which credential was used during discovery.
    #[serde(default)]
    pub credential_id: Uuid,
}

impl<T> IpOverride<T> {
    /// Check if this override targets localhost (127.0.0.1 or ::1).
    pub fn is_localhost(&self) -> bool {
        self.ip == IpAddr::V4(Ipv4Addr::LOCALHOST) || self.ip == IpAddr::V6(Ipv6Addr::LOCALHOST)
    }
}

impl<T> CredentialMapping<T> {
    /// Check if any credentials are configured
    pub fn is_enabled(&self) -> bool {
        self.default_credential.is_some() || !self.ip_overrides.is_empty()
    }

    /// Get credential for a specific IP, falling back to default
    pub fn get_credential_for_ip(&self, ip: &IpAddr) -> Option<&T> {
        self.ip_overrides
            .iter()
            .find(|o| &o.ip == ip)
            .map(|o| &o.credential)
            .or(self.default_credential.as_ref())
    }

    /// Collect all unique credential IDs referenced in this mapping's IP overrides.
    /// Excludes nil UUIDs (which indicate no server-side credential).
    pub fn credential_ids(&self) -> Vec<Uuid> {
        self.ip_overrides
            .iter()
            .map(|o| o.credential_id)
            .filter(|id| *id != Uuid::nil())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect()
    }
}

/// A credential payload paired with its server-side ID (if host-assignable).
/// `credential_id` is Some for host-scoped credentials (IP overrides from host assignments
/// or target_ips). None for network-level defaults and fallbacks — those don't get auto-assigned
/// to discovered hosts because they're already available network-wide.
#[derive(Debug, Clone)]
pub struct ResolvedCredential<T> {
    pub credential: T,
    pub credential_id: Option<Uuid>,
}

// ============================================================================
// Generic Credential Query Types (wire format for unified discovery)
// ============================================================================

/// Credential payload sent to daemon with secrets exposed.
/// Each variant corresponds to a CredentialType variant.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(tag = "type")]
pub enum CredentialQueryPayload {
    Snmp(SnmpQueryCredential),
    DockerProxy(DockerProxyQueryCredential),
}

impl Default for CredentialQueryPayload {
    fn default() -> Self {
        Self::Snmp(SnmpQueryCredential::default())
    }
}

impl CredentialQueryPayload {
    pub fn discovery_label(&self) -> &'static str {
        match self {
            Self::Snmp(_) => "SNMP queries",
            Self::DockerProxy(_) => "Docker proxy connection",
        }
    }

    /// Resolve all FilePath fields to Value by reading from disk,
    /// then validate PEM contents for fields that require it.
    pub fn resolve_file_paths(&self) -> Result<Self, anyhow::Error> {
        use super::types::InlineFormat;

        let label = self.discovery_label();
        match self {
            Self::Snmp(snmp) => Ok(Self::Snmp(SnmpQueryCredential {
                version: snmp.version,
                community: snmp.community.resolve_to_value("community", label)?,
            })),
            Self::DockerProxy(d) => {
                let ssl_cert = d
                    .ssl_cert
                    .as_ref()
                    .map(|v| v.resolve_to_value("ssl_cert", label))
                    .transpose()?;
                let ssl_key = d
                    .ssl_key
                    .as_ref()
                    .map(|v| v.resolve_to_value("ssl_key", label))
                    .transpose()?;
                let ssl_chain = d
                    .ssl_chain
                    .as_ref()
                    .map(|v| v.resolve_to_value("ssl_chain", label))
                    .transpose()?;

                // Validate resolved PEM contents
                if let Some(ResolvableValue::Value { value }) = &ssl_cert {
                    InlineFormat::PemCertificate.validate(value, "SSL Certificate")?;
                }
                if let Some(ResolvableSecret::Value { value }) = &ssl_key {
                    InlineFormat::PemPrivateKey.validate(value, "SSL Private Key")?;
                }
                if let Some(ResolvableValue::Value { value }) = &ssl_chain {
                    InlineFormat::PemCertificate.validate(value, "SSL CA Chain")?;
                }

                Ok(Self::DockerProxy(DockerProxyQueryCredential {
                    port: d.port,
                    path: d.path.clone(),
                    ssl_cert,
                    ssl_key,
                    ssl_chain,
                }))
            }
        }
    }

    pub fn banner_lines(&self) -> Vec<BannerField> {
        match self {
            Self::Snmp(snmp) => snmp.banner_lines(),
            Self::DockerProxy(docker) => docker.banner_lines(),
        }
    }
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
#[derive(Debug, Clone, Serialize, Eq, PartialEq, Hash, ToSchema)]
#[serde(tag = "mode")]
pub enum ResolvableSecret {
    Value { value: String },
    FilePath { path: String },
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
                std::fs::read_to_string(path).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to read {} from {} for {}: {}",
                        field_name,
                        path,
                        label,
                        e
                    )
                })
            }
        }
    }

    /// Read FilePath from disk and return Value. Value variants pass through.
    pub fn resolve_to_value(&self, field_name: &str, label: &str) -> Result<Self, anyhow::Error> {
        match self {
            Self::Value { .. } => Ok(self.clone()),
            Self::FilePath { path } => {
                tracing::info!("Read {} from {} for {}", field_name, path, label);
                let contents = std::fs::read_to_string(path).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to read {} from {} for {}: {}",
                        field_name,
                        path,
                        label,
                        e
                    )
                })?;
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
                let contents = std::fs::read_to_string(path).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to read {} from {} for {}: {}",
                        field_name,
                        path,
                        label,
                        e
                    )
                })?;
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
                let contents = std::fs::read_to_string(path).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to read {} from {} for {}: {}",
                        field_name,
                        path,
                        label,
                        e
                    )
                })?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snmp_cred(community: &str) -> SnmpQueryCredential {
        SnmpQueryCredential {
            version: SnmpVersion::V2c,
            community: ResolvableSecret::Value {
                value: community.to_string(),
            },
        }
    }

    fn make_override(ip: IpAddr, cred_id: Uuid) -> IpOverride<SnmpQueryCredential> {
        IpOverride {
            ip,
            credential: make_snmp_cred("public"),
            credential_id: cred_id,
        }
    }

    // -- credential_ids --

    #[test]
    fn credential_ids_filters_nil_uuids() {
        let mapping = CredentialMapping {
            default_credential: Some(make_snmp_cred("public")),
            ip_overrides: vec![
                make_override("10.0.0.1".parse().unwrap(), Uuid::nil()),
                make_override("10.0.0.2".parse().unwrap(), Uuid::new_v4()),
            ],
        };
        let ids = mapping.credential_ids();
        assert_eq!(ids.len(), 1);
        assert_ne!(ids[0], Uuid::nil());
    }

    #[test]
    fn credential_ids_deduplicates() {
        let shared_id = Uuid::new_v4();
        let mapping = CredentialMapping {
            default_credential: None,
            ip_overrides: vec![
                make_override("10.0.0.1".parse().unwrap(), shared_id),
                make_override("10.0.0.2".parse().unwrap(), shared_id),
            ],
        };
        let ids = mapping.credential_ids();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], shared_id);
    }

    #[test]
    fn credential_ids_empty_when_no_overrides() {
        let mapping: CredentialMapping<SnmpQueryCredential> = CredentialMapping {
            default_credential: Some(make_snmp_cred("public")),
            ip_overrides: vec![],
        };
        assert!(mapping.credential_ids().is_empty());
    }

    // -- is_enabled --

    #[test]
    fn is_enabled_default_only() {
        let mapping = CredentialMapping {
            default_credential: Some(make_snmp_cred("public")),
            ip_overrides: vec![],
        };
        assert!(mapping.is_enabled());
    }

    #[test]
    fn is_enabled_overrides_only() {
        let mapping = CredentialMapping {
            default_credential: None,
            ip_overrides: vec![make_override("10.0.0.1".parse().unwrap(), Uuid::new_v4())],
        };
        assert!(mapping.is_enabled());
    }

    #[test]
    fn is_enabled_empty() {
        let mapping: CredentialMapping<SnmpQueryCredential> = CredentialMapping::default();
        assert!(!mapping.is_enabled());
    }

    // -- get_credential_for_ip --

    #[test]
    fn get_credential_for_ip_override_match() {
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let mapping = CredentialMapping {
            default_credential: Some(make_snmp_cred("default")),
            ip_overrides: vec![IpOverride {
                ip,
                credential: make_snmp_cred("override"),
                credential_id: Uuid::new_v4(),
            }],
        };
        let cred = mapping.get_credential_for_ip(&ip).unwrap();
        assert_eq!(
            cred.community,
            ResolvableSecret::Value {
                value: "override".to_string()
            }
        );
    }

    #[test]
    fn get_credential_for_ip_fallback_to_default() {
        let mapping = CredentialMapping {
            default_credential: Some(make_snmp_cred("default")),
            ip_overrides: vec![make_override("10.0.0.1".parse().unwrap(), Uuid::new_v4())],
        };
        let other_ip: IpAddr = "10.0.0.99".parse().unwrap();
        let cred = mapping.get_credential_for_ip(&other_ip).unwrap();
        assert_eq!(
            cred.community,
            ResolvableSecret::Value {
                value: "default".to_string()
            }
        );
    }

    #[test]
    fn get_credential_for_ip_no_match() {
        let mapping: CredentialMapping<SnmpQueryCredential> = CredentialMapping {
            default_credential: None,
            ip_overrides: vec![make_override("10.0.0.1".parse().unwrap(), Uuid::new_v4())],
        };
        let other_ip: IpAddr = "10.0.0.99".parse().unwrap();
        assert!(mapping.get_credential_for_ip(&other_ip).is_none());
    }

    // -- is_localhost --

    #[test]
    fn is_localhost_v4() {
        let o = make_override("127.0.0.1".parse().unwrap(), Uuid::new_v4());
        assert!(o.is_localhost());
    }

    #[test]
    fn is_localhost_v6() {
        let o = make_override("::1".parse().unwrap(), Uuid::new_v4());
        assert!(o.is_localhost());
    }

    #[test]
    fn is_localhost_non_local() {
        let o = make_override("10.0.0.1".parse().unwrap(), Uuid::new_v4());
        assert!(!o.is_localhost());
    }
}
