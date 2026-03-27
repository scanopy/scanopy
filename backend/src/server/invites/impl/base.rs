use std::fmt::Display;
use std::str::FromStr;

use anyhow::Error;
use chrono::{DateTime, Utc};
use email_address::EmailAddress;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::postgres::PgRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::{
    shared::{
        entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants},
        entity_metadata::EntityCategory,
        storage::traits::{Entity, SqlValue, Storable},
    },
    users::r#impl::permissions::UserOrgPermissions,
};

/// CSV row representation for Invite export (excludes sensitive url field)
#[derive(Serialize)]
pub struct InviteCsvRow {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub permissions: String,
    pub created_by: Uuid,
    pub send_to: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct InviteBase {
    pub organization_id: Uuid,
    pub permissions: UserOrgPermissions,
    pub network_ids: Vec<Uuid>,
    pub url: String,
    pub created_by: Uuid,
    pub expires_at: DateTime<Utc>,
    #[schema(value_type = Option<String>, required)]
    /// Optional email address to send the invite to
    pub send_to: Option<EmailAddress>,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct Invite {
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: InviteBase,
}

impl Display for Invite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invite {}", self.id)
    }
}

impl Invite {
    /// Create a new invite with the specified expiration
    pub fn with_expiration(
        organization_id: Uuid,
        url: String,
        created_by: Uuid,
        expiration_hours: i64,
        permissions: UserOrgPermissions,
        network_ids: Vec<Uuid>,
        send_to: Option<EmailAddress>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InviteBase {
                organization_id,
                permissions,
                network_ids,
                url,
                created_by,
                expires_at: now + chrono::Duration::hours(expiration_hours),
                send_to,
            },
        }
    }

    pub fn is_valid(&self) -> bool {
        Utc::now() < self.base.expires_at
    }
}

impl ChangeTriggersTopologyStaleness<Invite> for Invite {
    fn triggers_staleness(&self, _other: Option<Invite>) -> bool {
        false
    }
}

impl Storable for Invite {
    type BaseData = InviteBase;

    fn table_name() -> &'static str {
        "invites"
    }

    fn new(base: Self::BaseData) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base,
        }
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error> {
        Ok((
            vec![
                "id",
                "organization_id",
                "permissions",
                "network_ids",
                "url",
                "created_by",
                "created_at",
                "updated_at",
                "expires_at",
                "send_to",
            ],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.organization_id),
                SqlValue::UserOrgPermissions(self.base.permissions),
                SqlValue::UuidArray(self.base.network_ids.clone()),
                SqlValue::String(self.base.url.clone()),
                SqlValue::Uuid(self.base.created_by),
                SqlValue::Timestamp(self.created_at),
                SqlValue::Timestamp(self.updated_at),
                SqlValue::Timestamp(self.base.expires_at),
                SqlValue::OptionalString(self.base.send_to.as_ref().map(|e| e.to_string())),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let send_to: Option<String> = row.get("send_to");
        let send_to = send_to
            .map(|s| EmailAddress::from_str(&s))
            .transpose()
            .map_err(|e| Error::msg(format!("Failed to parse email: {}", e)))?;

        let permissions_str: String = row.get("permissions");
        let permissions: UserOrgPermissions = permissions_str
            .parse()
            .map_err(|_| Error::msg("Failed to parse permissions"))?;

        Ok(Invite {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: InviteBase {
                organization_id: row.get("organization_id"),
                permissions,
                network_ids: row.get("network_ids"),
                url: row.get("url"),
                created_by: row.get("created_by"),
                expires_at: row.get("expires_at"),
                send_to,
            },
        })
    }
}

impl Entity for Invite {
    fn id(&self) -> Uuid {
        self.id
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn set_id(&mut self, id: Uuid) {
        self.id = id;
    }

    fn set_created_at(&mut self, time: DateTime<Utc>) {
        self.created_at = time;
    }

    type CsvRow = InviteCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        InviteCsvRow {
            id: self.id,
            organization_id: self.base.organization_id,
            permissions: format!("{:?}", self.base.permissions),
            created_by: self.base.created_by,
            send_to: self.base.send_to.as_ref().map(|e| e.to_string()),
            expires_at: self.base.expires_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Invite
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Invite";
    const ENTITY_NAME_PLURAL: &'static str = "Invites";
    const ENTITY_DESCRIPTION: &'static str =
        "Organization invitations. Invite users to join your organization.";

    fn entity_category() -> EntityCategory {
        EntityCategory::OrganizationsAndUsers
    }

    fn network_id(&self) -> Option<Uuid> {
        None
    }

    fn organization_id(&self) -> Option<Uuid> {
        Some(self.base.organization_id)
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn set_updated_at(&mut self, time: DateTime<Utc>) {
        self.updated_at = time;
    }
}
