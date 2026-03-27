use std::fmt::Display;
use std::str::FromStr;

use crate::server::{
    shared::{
        entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants},
        entity_metadata::EntityCategory,
        storage::traits::{Entity, SqlValue, Storable},
    },
    users::r#impl::permissions::UserOrgPermissions,
};
use anyhow::{Error, Result};
use chrono::{DateTime, Utc};
use email_address::EmailAddress;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::postgres::PgRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// CSV row representation for User export (excludes sensitive password/token fields)
#[derive(Serialize)]
pub struct UserCsvRow {
    pub id: Uuid,
    pub email: String,
    pub permissions: String,
    pub organization_id: Uuid,
    pub email_verified: bool,
    pub oidc_provider: Option<String>,
    pub terms_accepted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, PartialEq, Eq, Hash, ToSchema)]
pub struct UserBase {
    #[schema(value_type = String)]
    pub email: EmailAddress,
    pub organization_id: Uuid,
    pub permissions: UserOrgPermissions,
    /// Password hash - None for legacy users created before auth migration or users using OIDC
    #[serde(skip)] // Never send to client, never accept from client
    pub password_hash: Option<String>,
    /// Whether the user has a password set — computed from password_hash, never stored in DB
    #[serde(default)]
    #[schema(read_only)]
    pub has_password: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oidc_provider: Option<String>,
    #[serde(skip)]
    pub oidc_subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oidc_linked_at: Option<DateTime<Utc>>,
    #[serde(default)]
    #[schema(required)]
    pub network_ids: Vec<Uuid>,
    #[serde(default)]
    #[schema(read_only)]
    pub terms_accepted_at: Option<DateTime<Utc>>,
    /// Whether the user has verified their email address
    #[serde(default)]
    pub email_verified: bool,
    /// Token for email verification - never exposed to client
    #[serde(skip)]
    pub email_verification_token: Option<String>,
    /// Expiration time for email verification token
    #[serde(skip)]
    pub email_verification_expires: Option<DateTime<Utc>>,
    /// Token for password reset - never exposed to client
    #[serde(skip)]
    pub password_reset_token: Option<String>,
    /// Expiration time for password reset token
    #[serde(skip)]
    pub password_reset_expires: Option<DateTime<Utc>>,
    /// Pending email address for email change flow - never exposed to client
    #[serde(skip)]
    pub pending_email: Option<EmailAddress>,
}

impl Default for UserBase {
    fn default() -> Self {
        Self {
            email: EmailAddress::new_unchecked("user@example.com"),
            permissions: UserOrgPermissions::Owner,
            organization_id: Uuid::new_v4(),
            password_hash: None,
            has_password: false,
            oidc_linked_at: None,
            oidc_provider: None,
            oidc_subject: None,
            network_ids: vec![],
            terms_accepted_at: None,
            email_verified: false,
            email_verification_token: None,
            email_verification_expires: None,
            password_reset_token: None,
            password_reset_expires: None,
            pending_email: None,
        }
    }
}

impl UserBase {
    pub fn new_oidc(
        email: EmailAddress,
        oidc_subject: String,
        oidc_provider: Option<String>,
        organization_id: Uuid,
        permissions: UserOrgPermissions,
        network_ids: Vec<Uuid>,
        terms_accepted_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            email,
            password_hash: None,
            has_password: false,
            oidc_linked_at: Some(Utc::now()),
            permissions,
            organization_id,
            oidc_provider,
            oidc_subject: Some(oidc_subject),
            network_ids,
            terms_accepted_at,
            // OIDC users are already verified by the identity provider
            email_verified: true,
            email_verification_token: None,
            email_verification_expires: None,
            password_reset_token: None,
            password_reset_expires: None,
            pending_email: None,
        }
    }

    pub fn new_password(
        email: EmailAddress,
        password_hash: String,
        organization_id: Uuid,
        permissions: UserOrgPermissions,
        network_ids: Vec<Uuid>,
        terms_accepted_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            email,
            password_hash: Some(password_hash),
            has_password: true,
            organization_id,
            permissions,
            oidc_linked_at: None,
            oidc_provider: None,
            oidc_subject: None,
            network_ids,
            terms_accepted_at,
            // Email must be verified before login
            email_verified: false,
            email_verification_token: None,
            email_verification_expires: None,
            password_reset_token: None,
            password_reset_expires: None,
            pending_email: None,
        }
    }
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct User {
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
    pub base: UserBase,
}

impl User {
    pub fn set_password(&mut self, password_hash: String) {
        self.base.password_hash = Some(password_hash);
        self.base.has_password = true;
        self.updated_at = Utc::now();
    }
}

impl Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.base.email, self.id)
    }
}

impl ChangeTriggersTopologyStaleness<User> for User {
    fn triggers_staleness(&self, _other: Option<User>) -> bool {
        false
    }
}

impl Storable for User {
    type BaseData = UserBase;

    fn table_name() -> &'static str {
        "users"
    }

    fn new(base: Self::BaseData) -> Self {
        let now = chrono::Utc::now();

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
        let Self {
            id,
            created_at,
            updated_at,
            base:
                Self::BaseData {
                    email,
                    password_hash,
                    oidc_linked_at,
                    permissions,
                    organization_id,
                    oidc_provider,
                    oidc_subject,
                    terms_accepted_at,
                    email_verified,
                    email_verification_token,
                    email_verification_expires,
                    password_reset_token,
                    password_reset_expires,
                    pending_email,
                    ..
                },
        } = self.clone();

        // Note: network_ids is stored in user_network_access junction table, not here
        Ok((
            vec![
                "id",
                "email",
                "password_hash",
                "created_at",
                "updated_at",
                "oidc_linked_at",
                "oidc_provider",
                "oidc_subject",
                "permissions",
                "organization_id",
                "terms_accepted_at",
                "email_verified",
                "email_verification_token",
                "email_verification_expires",
                "password_reset_token",
                "password_reset_expires",
                "pending_email",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Email(email),
                SqlValue::OptionalString(password_hash),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::OptionTimestamp(oidc_linked_at),
                SqlValue::OptionalString(oidc_provider),
                SqlValue::OptionalString(oidc_subject),
                SqlValue::UserOrgPermissions(permissions),
                SqlValue::Uuid(organization_id),
                SqlValue::OptionTimestamp(terms_accepted_at),
                SqlValue::Bool(email_verified),
                SqlValue::OptionalString(email_verification_token),
                SqlValue::OptionTimestamp(email_verification_expires),
                SqlValue::OptionalString(password_reset_token),
                SqlValue::OptionTimestamp(password_reset_expires),
                SqlValue::OptionalString(pending_email.map(|e| e.to_string())),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let email = EmailAddress::from_str(&row.get::<String, _>("email"))
            .map_err(|e| Error::msg(format!("Failed to parse email: {}", e)))?;

        let permissions_str = row.get::<String, _>("permissions");
        let permissions: UserOrgPermissions = permissions_str
            .parse()
            .or(Err(Error::msg("Failed to parse permissions")))?;

        let pending_email: Option<EmailAddress> = row
            .get::<Option<String>, _>("pending_email")
            .and_then(|s| EmailAddress::from_str(&s).ok());

        let password_hash: Option<String> = row.get("password_hash");
        let has_password = password_hash.is_some();

        // Note: network_ids is populated separately from user_network_access junction table
        Ok(User {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: UserBase {
                email,
                password_hash,
                has_password,
                permissions,
                organization_id: row.get("organization_id"),
                oidc_linked_at: row.get("oidc_linked_at"),
                oidc_provider: row.get("oidc_provider"),
                oidc_subject: row.get("oidc_subject"),
                network_ids: vec![],
                terms_accepted_at: row.get("terms_accepted_at"),
                email_verified: row.get("email_verified"),
                email_verification_token: row.get("email_verification_token"),
                email_verification_expires: row.get("email_verification_expires"),
                password_reset_token: row.get("password_reset_token"),
                password_reset_expires: row.get("password_reset_expires"),
                pending_email,
            },
        })
    }
}

impl Entity for User {
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

    type CsvRow = UserCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        UserCsvRow {
            id: self.id,
            email: self.base.email.to_string(),
            permissions: format!("{:?}", self.base.permissions),
            organization_id: self.base.organization_id,
            email_verified: self.base.email_verified,
            oidc_provider: self.base.oidc_provider.clone(),
            terms_accepted_at: self.base.terms_accepted_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::User
    }

    const ENTITY_NAME_SINGULAR: &'static str = "User";
    const ENTITY_NAME_PLURAL: &'static str = "Users";
    const ENTITY_DESCRIPTION: &'static str =
        "User account management. Manage user profiles and permissions within organizations.";

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

    fn preserve_immutable_fields(&mut self, existing: &Self) {
        self.base.terms_accepted_at = existing.base.terms_accepted_at;
    }
}
