use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, str::FromStr};
use strum::{Display, EnumIter, IntoEnumIterator, IntoStaticStr};
use utoipa::ToSchema;

use crate::server::shared::{
    entities::EntityDiscriminants,
    types::{
        Color, Icon,
        metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
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

    /// Returns permission levels this user can assign to API keys.
    /// Users can grant their own level or below.
    pub fn grantable_api_key_permissions(&self) -> Vec<UserOrgPermissions> {
        UserOrgPermissions::iter().filter(|p| p <= self).collect()
    }

    /// Returns permission levels this user can assign to other users.
    /// Only Owners can create Admins; Admins can only create Member/Viewer.
    pub fn grantable_user_permissions(&self) -> Vec<UserOrgPermissions> {
        match self {
            UserOrgPermissions::Owner => UserOrgPermissions::iter().collect(),
            UserOrgPermissions::Admin => UserOrgPermissions::iter().filter(|p| p < self).collect(),
            _ => vec![],
        }
    }

    /// Check if this permission level can grant a specific API key permission
    pub fn can_grant_api_key_permission(&self, target: &UserOrgPermissions) -> bool {
        target <= self
    }

    /// Check if this permission level can grant a specific user permission
    pub fn can_grant_user_permission(&self, target: &UserOrgPermissions) -> bool {
        self.grantable_user_permissions().contains(target)
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

impl PartialOrd for UserOrgPermissions {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for UserOrgPermissions {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_rank = match self {
            UserOrgPermissions::Owner => 4,
            UserOrgPermissions::Admin => 3,
            UserOrgPermissions::Member => 2,
            UserOrgPermissions::Viewer => 1,
        };

        let other_rank = match other {
            UserOrgPermissions::Owner => 4,
            UserOrgPermissions::Admin => 3,
            UserOrgPermissions::Member => 2,
            UserOrgPermissions::Viewer => 1,
        };

        self_rank.cmp(&other_rank)
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
                "Full organization control: manage billing, invite any role, and access all administrative features"
            }
            UserOrgPermissions::Admin => {
                "Manage users and invites, create and modify all entities, but cannot access billing"
            }
            UserOrgPermissions::Member => "Create and modify entities for specific sites",
            UserOrgPermissions::Viewer => "View entities.",
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
        let manage_org_entities: bool =
            matches!(self, UserOrgPermissions::Owner | UserOrgPermissions::Admin);

        serde_json::json!({
            "grantable_api_key_permissions": self.grantable_api_key_permissions(),
            "grantable_user_permissions": self.grantable_user_permissions(),
            "manage_org_entities": manage_org_entities,
        })
    }
}
