use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::{Display, EnumIter, IntoEnumIterator, IntoStaticStr};
use utoipa::ToSchema;

use crate::server::{
    auth::access::{Action, Assignment, AssignmentScope, Keying, Resource},
    shared::{
        entities::EntityDiscriminants,
        types::{
            Color, Icon,
            metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
        },
    },
};

#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Display,
    PartialEq,
    Eq,
    EnumIter,
    IntoStaticStr,
    Hash,
    Default,
    ToSchema,
)]
pub enum UserOrgPermissions {
    Owner,
    Admin,
    Member,
    #[serde(alias = "Visualizer")]
    #[default]
    Viewer,
}

impl UserOrgPermissions {
    pub fn as_str(&self) -> &'static str {
        self.into()
    }

    /// The roles this role can give an API key, by the no-escalation rule.
    pub fn grantable_api_key_permissions(&self) -> Vec<UserOrgPermissions> {
        let grants = self.grants();
        UserOrgPermissions::iter()
            .filter(|target| target.grants().is_subset_of(&grants))
            .collect()
    }

    /// The roles this role can give another user, by the no-escalation rule. Nothing without
    /// `Member` Write, which adding or changing a user takes.
    pub fn grantable_user_permissions(&self) -> Vec<UserOrgPermissions> {
        let grants = self.grants();
        if !grants.allows(Resource::Member, Action::Write) {
            return vec![];
        }
        UserOrgPermissions::iter()
            .filter(|target| grants.may_grant_to_person(&target.grants(), Keying::Organization))
            .collect()
    }

    /// Whether a person holding this role reaches every site, whatever their site list says.
    fn reaches_all_sites(&self) -> bool {
        Assignment::for_person(*self, &[])
            .iter()
            .any(|assignment| assignment.scope == AssignmentScope::AllSites)
    }
}

impl FromStr for UserOrgPermissions {
    type Err = ();

    fn from_str(input: &str) -> Result<UserOrgPermissions, Self::Err> {
        match input {
            "Owner" => Ok(UserOrgPermissions::Owner),
            "Admin" => Ok(UserOrgPermissions::Admin),
            "Member" => Ok(UserOrgPermissions::Member),
            "Viewer" | "Visualizer" | "None" => Ok(UserOrgPermissions::Viewer),
            _ => Err(()),
        }
    }
}

impl HasId for UserOrgPermissions {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for UserOrgPermissions {
    fn color(&self) -> Color {
        EntityDiscriminants::User.color()
    }

    fn icon(&self) -> Icon {
        EntityDiscriminants::User.icon()
    }
}

impl TypeMetadataProvider for UserOrgPermissions {
    fn description(&self) -> &'static str {
        match self {
            UserOrgPermissions::Owner => {
                "Answers for the organization: its billing, licenses and settings"
            }
            UserOrgPermissions::Admin => {
                "Decides how the organization runs: users, sites, daemons and credentials"
            }
            UserOrgPermissions::Member => {
                "Keeps the network documented: hosts, topologies, scans and tags"
            }
            UserOrgPermissions::Viewer => "Sees the network and manages their own API keys",
        }
    }

    fn name(&self) -> &'static str {
        match self {
            UserOrgPermissions::Owner => "Owner",
            UserOrgPermissions::Admin => "Admin",
            UserOrgPermissions::Member => "Member",
            UserOrgPermissions::Viewer => "Viewer",
        }
    }

    fn metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "grantable_api_key_permissions": self.grantable_api_key_permissions(),
            "grantable_user_permissions": self.grantable_user_permissions(),
            "reaches_all_sites": self.reaches_all_sites(),
        })
    }
}
