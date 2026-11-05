use std::fmt::Display;

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use sqlx::{Row, postgres::PgRow};
use uuid::Uuid;

use crate::server::{
    auth::access::{Assignment, AssignmentScope},
    shared::{
        entities::EntityDiscriminants,
        storage::{
            child::{ChildStorableEntity, ParentRef},
            filter::StorableFilter,
            traits::{SqlValue, Storable},
        },
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
    pub fn parent(self) -> ParentRef {
        match self {
            Principal::User(id) => ParentRef::new(EntityDiscriminants::User, id),
            Principal::ApiKey(id) => ParentRef::new(EntityDiscriminants::UserApiKey, id),
            Principal::Invite(id) => ParentRef::new(EntityDiscriminants::Invite, id),
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

/// Rows belong to their organization, and a write replaces one principal's set of them.
impl ChildStorableEntity for SiteAssignment {
    fn parent_column() -> &'static str {
        "organization_id"
    }

    fn parent_id(&self) -> Uuid {
        self.base.organization_id
    }

    fn parent_filter(parent: &ParentRef) -> StorableFilter<Self> {
        let column = match parent.entity_type {
            EntityDiscriminants::UserApiKey => "api_key_id",
            EntityDiscriminants::Invite => "invite_id",
            _ => "user_id",
        };
        StorableFilter::new_from_uuid_column(column, &parent.id)
    }
}
