use std::fmt::Display;

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use sqlx::{Row, postgres::PgRow};
use uuid::Uuid;

use crate::server::{
    auth::access::{Assignment, AssignmentScope},
    shared::storage::{
        relation::RelationKey,
        traits::{SqlValue, Storable},
    },
};

/// Who holds access: the three things `site_assignments` can belong to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Principal {
    User(Uuid),
    ApiKey(Uuid),
    Invite(Uuid),
}

impl Default for Principal {
    fn default() -> Self {
        Principal::User(Uuid::nil())
    }
}

impl Principal {
    /// The set of rows this principal owns, which a write to them locks on.
    pub fn assignments(self) -> RelationKey<SiteAssignment> {
        use crate::server::shared::relations as R;
        match self {
            Principal::User(id) => R::SITE_ASSIGNMENT_USER.at(id),
            Principal::ApiKey(id) => R::SITE_ASSIGNMENT_API_KEY.at(id),
            Principal::Invite(id) => R::SITE_ASSIGNMENT_INVITE.at(id),
        }
    }
}

/// The base data for one row of `site_assignments`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SiteAssignmentBase {
    pub organization_id: Uuid,
    pub principal: Principal,
    pub assignment: Assignment,
}

/// One site role held by one principal, on one site or on all of them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SiteAssignment {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub base: SiteAssignmentBase,
}

impl SiteAssignment {
    /// The site this row names. `None` for All sites.
    pub fn site_id(&self) -> Option<Uuid> {
        match self.base.assignment.scope {
            AssignmentScope::AllSites => None,
            AssignmentScope::Site(site_id) => Some(site_id),
        }
    }
}

impl Display for SiteAssignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SiteAssignment({:?}, {:?})",
            self.base.principal, self.base.assignment
        )
    }
}

impl Storable for SiteAssignment {
    const HAS_SCD2: bool = false;
    type BaseData = SiteAssignmentBase;

    fn table_name() -> &'static str {
        "site_assignments"
    }

    fn relations() -> &'static [&'static crate::server::shared::storage::relation::Relation<Self>] {
        use crate::server::shared::relations as R;
        &[
            &R::SITE_ASSIGNMENT_ORGANIZATION,
            &R::SITE_ASSIGNMENT_USER,
            &R::SITE_ASSIGNMENT_API_KEY,
            &R::SITE_ASSIGNMENT_INVITE,
            &R::SITE_ASSIGNMENT_SITE,
        ]
    }

    fn new(base: Self::BaseData) -> Self {
        Self {
            id: Uuid::new_v4(),
            created_at: crate::server::shared::clock::now(),
            base,
        }
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>)> {
        let (user_id, api_key_id, invite_id) = match self.base.principal {
            Principal::User(id) => (Some(id), None, None),
            Principal::ApiKey(id) => (None, Some(id), None),
            Principal::Invite(id) => (None, None, Some(id)),
        };
        Ok((
            vec![
                "id",
                "organization_id",
                "user_id",
                "api_key_id",
                "invite_id",
                "role",
                "site_id",
                "created_at",
            ],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.organization_id),
                SqlValue::OptionalUuid(user_id),
                SqlValue::OptionalUuid(api_key_id),
                SqlValue::OptionalUuid(invite_id),
                SqlValue::String(self.base.assignment.role.to_string()),
                SqlValue::OptionalUuid(self.site_id()),
                SqlValue::Timestamp(self.created_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self> {
        let user_id: Option<Uuid> = row.get("user_id");
        let api_key_id: Option<Uuid> = row.get("api_key_id");
        let invite_id: Option<Uuid> = row.get("invite_id");
        let principal = match (user_id, api_key_id, invite_id) {
            (Some(id), None, None) => Principal::User(id),
            (None, Some(id), None) => Principal::ApiKey(id),
            (None, None, Some(id)) => Principal::Invite(id),
            _ => return Err(anyhow!("A site assignment needs exactly one principal")),
        };
        let role: String = row.get("role");
        let site_id: Option<Uuid> = row.get("site_id");
        Ok(Self {
            id: row.get("id"),
            created_at: row.get("created_at"),
            base: SiteAssignmentBase {
                organization_id: row.get("organization_id"),
                principal,
                assignment: Assignment {
                    role: role.parse()?,
                    scope: site_id.map_or(AssignmentScope::AllSites, AssignmentScope::Site),
                },
            },
        })
    }
}
