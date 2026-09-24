use serde::{Deserialize, Serialize};

/// The self-hosted commercial tier a license key entitles. Absent on legacy
/// keys (issued before tiers existed) and on custom/grandfathered keys — those
/// resolve to `CommercialSelfHosted` via `plan_for_license`. Enterprise deals
/// stay off this enum: they are hand-issued and mapped separately.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
#[clap(rename_all = "lowercase")]
pub enum LicensePlan {
    Standard,
    Plus,
}

/// The two keys an org owner can copy for a self-hosted server. An
/// organization has one of these issued at a time, stored on the org row.
///
/// Serde keeps the API spelling (`"Online"` / `"Offline"`); strum supplies the
/// snake_case text the column is stored as, the way `PlanStatus` does.
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    Default,
    strum_macros::Display,
    strum_macros::EnumString,
    utoipa::ToSchema,
)]
#[strum(serialize_all = "snake_case")]
pub enum LicenseKeyType {
    /// Permanent credential; the server fetches its entitlement from the cloud.
    #[default]
    Online,
    /// Self-contained key with its expiry baked in, for air-gapped servers.
    Offline,
}

/// JWT claims encoded in a Scanopy license key.
///
/// The license key is an authorization gate that also names the licensed tier
/// (`plan`). Plan entitlements come from the resolved `BillingPlan`, not from
/// arbitrary fields on the key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseClaims {
    /// Subject — always "scanopy-license"
    pub sub: String,
    /// Issuer — always "scanopy"
    pub iss: String,
    /// Issued-at (unix timestamp)
    pub iat: i64,
    /// Hard expiry (unix timestamp). The verifier rejects the key once
    /// `now > exp`. Always 7 days past `intended_exp` — the extra week
    /// is a silent runway during which the UI warns the user to rotate.
    pub exp: i64,
    /// User-visible expiry (unix timestamp). CLI output and the UI show
    /// this as the license "expires on" date. When `intended_exp < now <= exp`
    /// the key is in its grace window.
    pub intended_exp: i64,
    /// Organization ID — populated when Cloud-provisioned (future)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub org_id: Option<String>,
    /// Licensed self-hosted tier. Absent on legacy/custom keys, which resolve
    /// to `CommercialSelfHosted`. See `plan_for_license`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<LicensePlan>,
}

/// Runtime license state of a configured key, checked by middleware on every
/// request. A deployment with no license key has no `LicenseService` at all
/// (the community/cloud case), so there is no "not required" variant here.
#[derive(Debug, Clone, strum_macros::EnumDiscriminants)]
#[strum_discriminants(
    derive(Serialize, Deserialize, utoipa::ToSchema),
    serde(rename_all = "lowercase"),
    doc = "Runtime license state as reported by the public config endpoint."
)]
pub enum LicenseStatus {
    /// Valid commercial license
    Valid(LicenseClaims),
    /// Valid signature but past expiry date
    Expired(LicenseClaims),
    /// Invalid key (bad signature, malformed, or missing when required), or an
    /// online key the cloud rejected
    Invalid(String),
    /// Online key with no entitlement yet (first boot, cloud unreachable).
    /// Locks the server until the first successful check-in. The guard still
    /// allows auth and reads, so a new instance can register a user, see the
    /// banner, and recover once it reaches the cloud.
    Pending,
}

impl LicenseStatus {
    /// Whether the server should be in read-only locked state.
    pub fn is_locked(&self) -> bool {
        matches!(
            self,
            LicenseStatus::Expired(_) | LicenseStatus::Invalid(_) | LicenseStatus::Pending
        )
    }

    /// Status string for the public config API response.
    pub fn as_api_string(&self) -> &'static str {
        match self {
            LicenseStatus::Valid(_) => "valid",
            LicenseStatus::Expired(_) => "expired",
            LicenseStatus::Invalid(_) => "invalid",
            LicenseStatus::Pending => "pending",
        }
    }

    /// Data-free view of the status, as published by the public config endpoint.
    pub fn kind(&self) -> LicenseStatusDiscriminants {
        LicenseStatusDiscriminants::from(self)
    }

    /// Hard expiry date as ISO date string (e.g. "2027-04-11"), if available.
    pub fn expiry_date(&self) -> Option<String> {
        let claims = match self {
            LicenseStatus::Valid(c) | LicenseStatus::Expired(c) => c,
            _ => return None,
        };
        chrono::DateTime::from_timestamp(claims.exp, 0).map(|d| d.format("%Y-%m-%d").to_string())
    }

    /// User-visible expiry date as ISO date string, if available.
    pub fn intended_expiry_date(&self) -> Option<String> {
        let claims = match self {
            LicenseStatus::Valid(c) | LicenseStatus::Expired(c) => c,
            _ => return None,
        };
        chrono::DateTime::from_timestamp(claims.intended_exp, 0)
            .map(|d| d.format("%Y-%m-%d").to_string())
    }

    /// Whether the license is currently in its grace window —
    /// `intended_exp < now <= exp`. The server still accepts the key
    /// but the UI should warn the user to rotate it.
    pub fn in_grace_period(&self) -> bool {
        self.in_grace_period_at(chrono::Utc::now().timestamp())
    }

    /// Grace-period check against a caller-supplied `now`, for tests.
    pub fn in_grace_period_at(&self, now: i64) -> bool {
        let LicenseStatus::Valid(claims) = self else {
            return false;
        };
        claims.intended_exp < now && now <= claims.exp
    }
}
