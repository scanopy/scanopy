use semver::Version;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Version policy for daemons
///
/// Defines which versions are supported, recommended, and deprecated.
/// Used to evaluate daemon version status and generate warnings.
pub struct DaemonVersionPolicy {
    pub minimum_supported: Version,
    pub recommended: Version,
    pub latest: Version,
}

impl Default for DaemonVersionPolicy {
    fn default() -> Self {
        // Use CARGO_PKG_VERSION for both recommended and latest
        // so the current release is always considered "Current"
        let current = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        Self {
            minimum_supported: Version::new(0, 12, 0),
            recommended: current.clone(),
            latest: current,
        }
    }
}

/// Minimum daemon version required for unified discovery support.
pub fn minimum_unified_discovery() -> Version {
    Version::new(0, 15, 0)
}

/// Returns true if the daemon version supports unified discovery (>= 0.15.0).
pub fn supports_unified_discovery(version: Option<&Version>) -> bool {
    version.is_some_and(|v| v >= &minimum_unified_discovery())
}

impl DaemonVersionPolicy {
    pub fn evaluate(&self, version: Option<&Version>) -> DaemonVersionStatus {
        match version {
            None => self.evaluate_unknown(),
            Some(v) => self.evaluate_known(v),
        }
    }

    /// During migration period: unknown = outdated
    fn evaluate_unknown(&self) -> DaemonVersionStatus {
        DaemonVersionStatus {
            version: None,
            status: VersionHealthStatus::Outdated,
            warnings: vec![DeprecationWarning {
                message: format!(
                    "Daemon version unknown. Update to {} or later.",
                    self.recommended
                ),
                sunset_date: None,
                severity: DeprecationSeverity::Warning,
            }],
            supports_unified_discovery: false,
        }
    }

    fn evaluate_known(&self, v: &Version) -> DaemonVersionStatus {
        let supports_unified = supports_unified_discovery(Some(v));
        if v < &self.minimum_supported {
            DaemonVersionStatus {
                version: Some(v.to_string()),
                status: VersionHealthStatus::Deprecated,
                warnings: vec![DeprecationWarning {
                    message: format!(
                        "Daemon {} is deprecated. Update to {} or later.",
                        v, self.recommended
                    ),
                    sunset_date: Some("2025-02-01".into()),
                    severity: DeprecationSeverity::Critical,
                }],
                supports_unified_discovery: supports_unified,
            }
        } else if v < &self.recommended {
            DaemonVersionStatus {
                version: Some(v.to_string()),
                status: VersionHealthStatus::Outdated,
                warnings: vec![DeprecationWarning {
                    message: format!(
                        "Daemon {} is outdated. Update to {} for latest features.",
                        v, self.recommended
                    ),
                    sunset_date: None,
                    severity: DeprecationSeverity::Warning,
                }],
                supports_unified_discovery: supports_unified,
            }
        } else {
            DaemonVersionStatus {
                version: Some(v.to_string()),
                status: VersionHealthStatus::Current,
                warnings: vec![],
                supports_unified_discovery: supports_unified,
            }
        }
    }
}

/// Deprecation warning for daemon version
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeprecationWarning {
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sunset_date: Option<String>,
    pub severity: DeprecationSeverity,
}

/// Severity level for deprecation warnings
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, Default, PartialEq, Eq)]
pub enum DeprecationSeverity {
    #[default]
    Info,
    Warning,
    Critical,
}

/// Daemon version status including health and any warnings
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DaemonVersionStatus {
    pub version: Option<String>,
    pub status: VersionHealthStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<DeprecationWarning>,
    #[serde(default)]
    pub supports_unified_discovery: bool,
}

/// Health status for daemon versions
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub enum VersionHealthStatus {
    Current,
    Outdated,
    Deprecated,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_policy() -> DaemonVersionPolicy {
        // Use fixed versions for predictable tests
        DaemonVersionPolicy {
            minimum_supported: Version::new(0, 12, 0),
            recommended: Version::new(0, 12, 8),
            latest: Version::new(0, 12, 8),
        }
    }

    #[test]
    fn test_unknown_version_is_outdated() {
        let policy = test_policy();
        let status = policy.evaluate(None);

        assert_eq!(status.status, VersionHealthStatus::Outdated);
        assert!(status.version.is_none());
        assert_eq!(status.warnings.len(), 1);
        assert_eq!(status.warnings[0].severity, DeprecationSeverity::Warning);
        assert!(!status.supports_unified_discovery);
    }

    #[test]
    fn test_deprecated_version() {
        let policy = test_policy();
        let old_version = Version::new(0, 11, 0);
        let status = policy.evaluate(Some(&old_version));

        assert_eq!(status.status, VersionHealthStatus::Deprecated);
        assert_eq!(status.version, Some("0.11.0".to_string()));
        assert_eq!(status.warnings.len(), 1);
        assert_eq!(status.warnings[0].severity, DeprecationSeverity::Critical);
        assert!(status.warnings[0].sunset_date.is_some());
        assert!(!status.supports_unified_discovery);
    }

    #[test]
    fn test_outdated_version() {
        let policy = test_policy();
        let outdated_version = Version::new(0, 12, 5);
        let status = policy.evaluate(Some(&outdated_version));

        assert_eq!(status.status, VersionHealthStatus::Outdated);
        assert_eq!(status.version, Some("0.12.5".to_string()));
        assert_eq!(status.warnings.len(), 1);
        assert_eq!(status.warnings[0].severity, DeprecationSeverity::Warning);
        assert!(!status.supports_unified_discovery);
    }

    #[test]
    fn test_current_version() {
        let policy = test_policy();
        let current_version = Version::new(0, 12, 8);
        let status = policy.evaluate(Some(&current_version));

        assert_eq!(status.status, VersionHealthStatus::Current);
        assert_eq!(status.version, Some("0.12.8".to_string()));
        assert!(status.warnings.is_empty());
        assert!(!status.supports_unified_discovery);
    }

    #[test]
    fn test_newer_than_recommended_is_current() {
        let policy = test_policy();
        let future_version = Version::new(0, 14, 0);
        let status = policy.evaluate(Some(&future_version));

        assert_eq!(status.status, VersionHealthStatus::Current);
        assert!(status.warnings.is_empty());
        assert!(!status.supports_unified_discovery);
    }

    #[test]
    fn test_supports_unified_discovery() {
        assert!(!supports_unified_discovery(None));
        assert!(!supports_unified_discovery(Some(&Version::new(0, 14, 0))));
        assert!(supports_unified_discovery(Some(&Version::new(0, 15, 0))));
        assert!(supports_unified_discovery(Some(&Version::new(0, 16, 0))));
        assert!(supports_unified_discovery(Some(&Version::new(1, 0, 0))));
    }

    #[test]
    fn test_version_status_supports_unified_at_015() {
        let policy = test_policy();
        let v015 = Version::new(0, 15, 0);
        let status = policy.evaluate(Some(&v015));
        assert!(status.supports_unified_discovery);
    }
}
