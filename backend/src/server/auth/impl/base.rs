use crate::server::{
    config::DeploymentType, organizations::r#impl::base::UseCase,
    users::r#impl::permissions::UserOrgPermissions,
};
use chrono::{DateTime, Utc};
use email_address::EmailAddress;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use uuid::Uuid;

pub struct LoginRegisterParams {
    pub provision_org: ProvisionOrg,
    pub permissions: Option<UserOrgPermissions>,
    pub ip: IpAddr,
    pub user_agent: Option<String>,
    pub site_ids: Vec<Uuid>,
    /// The address an accepted invite was emailed to. Registering with it counts as proof
    /// of the address.
    pub invited_as: Option<EmailAddress>,
}

#[derive(Clone)]
pub enum ProvisionOrg {
    New(PendingSetup),
    Existing(Uuid),
}

impl ProvisionOrg {
    pub fn is_new(&self) -> bool {
        matches!(self, ProvisionOrg::New(_))
    }

    pub fn is_existing(&self) -> bool {
        matches!(self, ProvisionOrg::Existing(_))
    }
}

pub struct ProvisionUserParams {
    pub email: EmailAddress,
    pub password_hash: Option<String>,
    pub oidc_subject: Option<String>,
    pub oidc_provider: Option<String>,
    pub provision_org: ProvisionOrg,
    pub permissions: Option<UserOrgPermissions>,
    pub site_ids: Vec<Uuid>,
    pub terms_accepted_at: Option<DateTime<Utc>>,
    pub email_verified: bool,
    /// Where this server runs. A self-hosted server gives a new org its
    /// license plan and caps how many orgs exist; cloud leaves the plan to
    /// checkout.
    pub deployment_type: DeploymentType,
}

/// The first site requested at signup. `provision_user` creates it; it is
/// also carried on `OrgCreated` for telemetry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
pub struct PendingSiteSetup {
    /// Name the user gave the site.
    pub name: String,
    /// Id assigned at setup, before the site exists.
    #[serde(alias = "network_id")]
    pub site_id: Uuid,
}

/// Setup data collected before registration (org name, site, use case)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingSetup {
    pub org_name: String,
    /// `None` when the signup is a self-hosted license buyer.
    #[serde(alias = "network")]
    pub site: Option<PendingSiteSetup>,
    /// Use case selection (homelab, company, msp)
    pub use_case: UseCase,
}
