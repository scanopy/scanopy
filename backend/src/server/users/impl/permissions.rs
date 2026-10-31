use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::{Display, EnumIter, IntoEnumIterator, IntoStaticStr};
use utoipa::ToSchema;

use crate::server::{
    auth::access::{AccessLevel, Action, Assignment, AssignmentScope, Keying, Resource},
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

    /// The role one step below this one. Viewer is the baseline.
    pub fn previous_role(&self) -> Option<UserOrgPermissions> {
        match self {
            UserOrgPermissions::Owner => Some(UserOrgPermissions::Admin),
            UserOrgPermissions::Admin => Some(UserOrgPermissions::Member),
            UserOrgPermissions::Member => Some(UserOrgPermissions::Viewer),
            UserOrgPermissions::Viewer => None,
        }
    }

    /// What this role adds over [`Self::previous_role`], one entry per level, the way
    /// `BillingPlan::incremental_features` does for plans. Viewer lists all of its access.
    pub fn incremental_access(&self) -> Vec<serde_json::Value> {
        let inherited = self.previous_role();
        AccessLevel::iter()
            .filter_map(|level| {
                let resources: Vec<Resource> = Resource::iter()
                    .filter(|resource| {
                        resource.level(*self) == Some(level)
                            && inherited.and_then(|role| resource.level(role)) != Some(level)
                    })
                    .collect();
                (!resources.is_empty())
                    .then(|| serde_json::json!({ "level": level, "resources": resources }))
            })
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
            "previous_role": self.previous_role(),
            "incremental_access": self.incremental_access(),
            "reaches_all_sites": self.reaches_all_sites(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A role's level on a resource is what it adds, or what the role below already holds.
    #[test]
    fn incremental_access_is_exactly_what_the_previous_role_lacks() {
        for role in UserOrgPermissions::iter() {
            let added = role.incremental_access();
            for resource in Resource::iter() {
                let listed = added.iter().any(|entry| {
                    entry["level"] == serde_json::json!(resource.level(role))
                        && entry["resources"]
                            .as_array()
                            .is_some_and(|r| r.contains(&serde_json::json!(resource)))
                });
                let unchanged =
                    role.previous_role().and_then(|p| resource.level(p)) == resource.level(role);
                assert_eq!(
                    listed,
                    resource.level(role).is_some() && !unchanged,
                    "{role} on {resource}"
                );
            }
        }
    }
}
