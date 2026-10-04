use crate::server::{
    organizations::r#impl::base::UseCase, users::r#impl::permissions::UserOrgPermissions,
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
    pub network_ids: Vec<Uuid>,
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
    pub network_ids: Vec<Uuid>,
    pub terms_accepted_at: Option<DateTime<Utc>>,
    pub email_verified: bool,
    /// Whether billing is enabled (if false, sets default billing plan for self-hosted)
    pub billing_enabled: bool,
}

/// The first network requested at signup. Carried on `OrgCreated`, whose
/// subscriber creates it, so it survives past the session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
pub struct PendingNetworkSetup {
    /// Name the user gave the network.
    pub name: String,
    /// Id assigned at setup, before the network exists.
    pub network_id: Uuid,
}

/// Setup data collected before registration (org name, network, use case)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingSetup {
    pub org_name: String,
    /// `None` when the signup is a self-hosted license buyer.
    pub network: Option<PendingNetworkSetup>,
    /// Use case selection (homelab, company, msp)
    pub use_case: UseCase,
}
