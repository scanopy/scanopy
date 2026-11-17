use crate::server::{
    auth::access::Assignment,
    config::DeploymentType,
    organizations::r#impl::{base::UseCase, intent::SignupIntent},
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
    pub site_assignments: Vec<Assignment>,
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
    pub site_assignments: Vec<Assignment>,
    pub terms_accepted_at: Option<DateTime<Utc>>,
    pub email_verified: bool,
    /// Where this server runs. A self-hosted server gives a new org its
    /// license plan and caps how many orgs exist; cloud leaves the plan to
    /// checkout.
    pub deployment_type: DeploymentType,
    /// The signup request's address and agent, for the `OrgCreated` event a new org publishes.
    pub ip_address: std::net::IpAddr,
    pub user_agent: Option<String>,
}

/// The first site requested at signup. `provision_user` creates it; it is
/// also carried on `OrgCreated` for telemetry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
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
    /// The signup's other answers. Defaulted so a session saved before they existed still loads.
    #[serde(default)]
    pub answers: SignupAnswers,
}

/// The signup's answers beside the use case, saved by `/onboarding-step` and stored on the org.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignupAnswers {
    /// The text typed for "Something else" as the use case.
    pub use_case_other: Option<String>,
    /// `None` when the intent step was never answered, empty when it was skipped.
    pub intents: Option<Vec<SignupIntent>>,
    /// The text typed for "Something else" as an intent.
    pub intent_other: Option<String>,
}

impl SignupAnswers {
    /// Overwrite each answer `update` carries, keeping the rest.
    pub fn merge(&mut self, update: SignupAnswers) {
        if update.use_case_other.is_some() {
            self.use_case_other = update.use_case_other;
        }
        if update.intents.is_some() {
            self.intents = update.intents;
        }
        if update.intent_other.is_some() {
            self.intent_other = update.intent_other;
        }
    }

    /// The answers as stored on the org: free text only where "Something else" was picked, so a
    /// signup that typed text and then chose another option keeps none of it.
    pub fn for_use_case(self, use_case: UseCase) -> SignupAnswers {
        let picked_other_intent = self
            .intents
            .as_ref()
            .is_some_and(|intents| intents.contains(&SignupIntent::Other));
        SignupAnswers {
            use_case_other: self.use_case_other.filter(|_| use_case == UseCase::Other),
            intent_other: self.intent_other.filter(|_| picked_other_intent),
            intents: self.intents,
        }
    }
}
