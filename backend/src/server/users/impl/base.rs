use std::fmt::Display;
use std::str::FromStr;

use crate::server::auth::access::{Grants, Keying};
use crate::server::{
    shared::{
        entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants},
        product_area::ProductArea,
        storage::traits::{Entity, SqlValue, Storable},
    },
    users::r#impl::{
        display_settings::DisplaySettings, email_settings::EmailSettings,
        permissions::UserOrgPermissions,
    },
};
use anyhow::{Error, Result};
use chrono::{DateTime, NaiveDate, Utc};
use email_address::EmailAddress;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::postgres::PgRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// The version this binary was built as.
const RUNNING_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize, Validate, PartialEq, Eq, Hash, ToSchema)]
pub struct UserBase {
    /// The user's email address, also their login identifier.
    #[schema(value_type = String, format = "email")]
    pub email: EmailAddress,
    /// The organization that owns this record.
    pub organization_id: Uuid,
    /// The user's role within the organization.
    pub permissions: UserOrgPermissions,
    /// Password hash - None for legacy users created before auth migration or users using OIDC
    #[serde(skip)] // Never send to client, never accept from client
    pub password_hash: Option<String>,
    /// Whether the user has a password set — computed from password_hash, never stored in DB
    #[serde(
        default,
        deserialize_with = "crate::server::shared::types::api::ignored"
    )]
    #[schema(read_only)]
    pub has_password: bool,
    /// Slug of the identity provider this account signs in through, when linked.
    #[serde(
        default,
        deserialize_with = "crate::server::shared::types::api::ignored",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(read_only)]
    pub oidc_provider: Option<String>,
    /// The provider's stable identifier for this user.
    #[serde(skip)]
    pub oidc_subject: Option<String>,
    /// When the account was linked to the identity provider.
    #[serde(
        default,
        deserialize_with = "crate::server::shared::types::api::ignored",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(read_only)]
    pub oidc_linked_at: Option<DateTime<Utc>>,
    /// The sites this entity applies to.
    #[serde(default)]
    #[schema(required)]
    pub site_ids: Vec<Uuid>,
    /// When the user accepted the terms of service. Server-owned and read only by the
    /// terms gate, so never sent to the client.
    #[serde(skip)]
    pub terms_accepted_at: Option<DateTime<Utc>>,
    /// Whether the user must accept the current terms before the API serves
    /// their session. Computed, never stored: set only on the signed-in user's
    /// own record (`/api/auth/me`, login, accept), false everywhere else.
    #[serde(
        default,
        deserialize_with = "crate::server::shared::types::api::ignored"
    )]
    #[schema(read_only)]
    pub terms_acceptance_required: bool,
    /// Whether the user must verify their email before the API serves their
    /// session (an org on a license plan, see `auth::verification_gate`).
    /// Computed and set like `terms_acceptance_required`.
    #[serde(
        default,
        deserialize_with = "crate::server::shared::types::api::ignored"
    )]
    #[schema(read_only)]
    pub email_verification_required: bool,
    /// Whether the user has verified their email address
    #[serde(
        default,
        deserialize_with = "crate::server::shared::types::api::ignored"
    )]
    #[schema(read_only)]
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
    /// Opaque token in the one-click unsubscribe link of pausable emails. Minted
    /// on first need, never exposed to the client.
    #[serde(skip)]
    pub email_unsubscribe_token: Option<Uuid>,
    /// Per-user email preferences
    #[serde(default)]
    pub email_settings: EmailSettings,
    /// Per-user date and time display preferences
    #[serde(default)]
    pub display_settings: DisplaySettings,
    /// Monotonic session epoch. Every active session records the epoch it was
    /// created under; the auth extractor rejects a session whose epoch is below
    /// the user's current value. Bumping this (on password change/reset)
    /// invalidates all other sessions ("log out everywhere"). Never client-facing.
    #[serde(skip)]
    pub session_epoch: i64,
    /// The newest release whose notes the user has dismissed. Server-owned: a
    /// new user starts at the running version, and only `POST /release-notes/seen`
    /// moves it. `None` for a user created before release notes existed.
    #[serde(skip)]
    pub release_notes_seen_version: Option<String>,
    /// The newest terms version (its effective date) this user was emailed
    /// notice of. Server-owned; `None` until the first notice.
    #[serde(skip)]
    pub terms_notice_version: Option<NaiveDate>,
}

impl Default for UserBase {
    fn default() -> Self {
        Self {
            email: EmailAddress::new_unchecked("user@example.com"),
            permissions: UserOrgPermissions::Owner,
            organization_id: Uuid::new_v4(),
            password_hash: None,
            has_password: false,
            terms_acceptance_required: false,
            email_verification_required: false,
            oidc_linked_at: None,
            oidc_provider: None,
            oidc_subject: None,
            site_ids: vec![],
            terms_accepted_at: None,
            email_verified: false,
            email_verification_token: None,
            email_verification_expires: None,
            password_reset_token: None,
            password_reset_expires: None,
            pending_email: None,
            email_unsubscribe_token: None,
            email_settings: EmailSettings::default(),
            display_settings: DisplaySettings::default(),
            session_epoch: 0,
            release_notes_seen_version: None,
            terms_notice_version: None,
        }
    }
}

impl UserBase {
    #[allow(clippy::too_many_arguments)]
    pub fn new_oidc(
        email: EmailAddress,
        email_verified: bool,
        oidc_subject: String,
        oidc_provider: Option<String>,
        organization_id: Uuid,
        permissions: UserOrgPermissions,
        site_ids: Vec<Uuid>,
        terms_accepted_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            email,
            password_hash: None,
            has_password: false,
            terms_acceptance_required: false,
            email_verification_required: false,
            oidc_linked_at: Some(Utc::now()),
            permissions,
            organization_id,
            oidc_provider,
            oidc_subject: Some(oidc_subject),
            site_ids,
            terms_accepted_at,
            email_verified,
            email_verification_token: None,
            email_verification_expires: None,
            password_reset_token: None,
            password_reset_expires: None,
            pending_email: None,
            email_unsubscribe_token: None,
            email_settings: EmailSettings::default(),
            display_settings: DisplaySettings::default(),
            session_epoch: 0,
            release_notes_seen_version: Some(RUNNING_VERSION.to_string()),
            terms_notice_version: None,
        }
    }

    pub fn new_password(
        email: EmailAddress,
        email_verified: bool,
        password_hash: String,
        organization_id: Uuid,
        permissions: UserOrgPermissions,
        site_ids: Vec<Uuid>,
        terms_accepted_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            email,
            password_hash: Some(password_hash),
            has_password: true,
            terms_acceptance_required: false,
            email_verification_required: false,
            organization_id,
            permissions,
            oidc_linked_at: None,
            oidc_provider: None,
            oidc_subject: None,
            site_ids,
            terms_accepted_at,
            email_verified,
            email_verification_token: None,
            email_verification_expires: None,
            password_reset_token: None,
            password_reset_expires: None,
            pending_email: None,
            email_unsubscribe_token: None,
            email_settings: EmailSettings::default(),
            display_settings: DisplaySettings::default(),
            session_epoch: 0,
            release_notes_seen_version: Some(RUNNING_VERSION.to_string()),
            terms_notice_version: None,
        }
    }
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct User {
    /// Server-assigned unique identifier.
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    /// When this record was first created.
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    /// When this record was last modified.
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: UserBase,
}

impl User {
    /// Whether a member with these organization grants sees this user in the users list: the
    /// users whose role they could grant, and themselves.
    pub fn is_listed_for(&self, viewer: &Grants, viewer_id: Uuid) -> bool {
        self.id == viewer_id
            || viewer.may_grant_to_person(&self.base.permissions.grants(), Keying::Organization)
    }

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

impl crate::server::shared::events::log::EntityLog for User {}

impl ChangeTriggersTopologyStaleness<User> for User {
    fn triggers_staleness(&self, _other: Option<User>) -> bool {
        false
    }
}

impl Storable for User {
    const HAS_SCD2: bool = false;
    type BaseData = UserBase;

    fn table_name() -> &'static str {
        "users"
    }

    fn search_predicates() -> &'static [&'static str] {
        &["users.email ILIKE {}"]
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
                    email_unsubscribe_token,
                    email_settings,
                    display_settings,
                    session_epoch,
                    release_notes_seen_version,
                    terms_notice_version,
                    ..
                },
        } = self.clone();

        // Note: site_ids is stored in user_site_access junction table, not here
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
                "email_unsubscribe_token",
                "email_settings",
                "display_settings",
                "session_epoch",
                "release_notes_seen_version",
                "terms_notice_version",
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
                SqlValue::OptionalUuid(email_unsubscribe_token),
                SqlValue::EmailSettings(email_settings),
                SqlValue::DisplaySettings(display_settings),
                SqlValue::I64(session_epoch),
                SqlValue::OptionalString(release_notes_seen_version),
                SqlValue::OptionDate(terms_notice_version),
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

        // Note: site_ids is populated separately from user_site_access junction table
        Ok(User {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: UserBase {
                email,
                password_hash,
                has_password,
                terms_acceptance_required: false,
                email_verification_required: false,
                permissions,
                organization_id: row.get("organization_id"),
                oidc_linked_at: row.get("oidc_linked_at"),
                oidc_provider: row.get("oidc_provider"),
                oidc_subject: row.get("oidc_subject"),
                site_ids: vec![],
                terms_accepted_at: row.get("terms_accepted_at"),
                email_verified: row.get("email_verified"),
                email_verification_token: row.get("email_verification_token"),
                email_verification_expires: row.get("email_verification_expires"),
                password_reset_token: row.get("password_reset_token"),
                password_reset_expires: row.get("password_reset_expires"),
                pending_email,
                email_unsubscribe_token: row.get("email_unsubscribe_token"),
                email_settings: row
                    .try_get::<serde_json::Value, _>("email_settings")
                    .ok()
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default(),
                display_settings: row
                    .try_get::<serde_json::Value, _>("display_settings")
                    .ok()
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default(),
                session_epoch: row.try_get("session_epoch").unwrap_or(0),
                release_notes_seen_version: row.get("release_notes_seen_version"),
                terms_notice_version: row.get("terms_notice_version"),
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

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::User
    }

    const ENTITY_NAME_SINGULAR: &'static str = "User";
    const ENTITY_NAME_PLURAL: &'static str = "Users";
    const ENTITY_DESCRIPTION: &'static str =
        "User account management. Manage user profiles and permissions within organizations.";

    fn product_area() -> ProductArea {
        ProductArea::Platform
    }

    fn site_id(&self) -> Option<Uuid> {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A stored user mid-flow: logged out everywhere twice, a password reset
    /// and an email change pending, not yet verified.
    fn stored_user() -> User {
        let now = Utc::now();
        User {
            id: Uuid::new_v4(),
            created_at: now - chrono::Duration::days(30),
            updated_at: now,
            base: UserBase {
                email: EmailAddress::new_unchecked("owner@example.com"),
                permissions: UserOrgPermissions::Admin,
                password_hash: Some("argon2-hash".to_string()),
                has_password: true,
                oidc_provider: Some("google".to_string()),
                oidc_subject: Some("subject-1".to_string()),
                oidc_linked_at: Some(now),
                terms_accepted_at: Some(now),
                email_verified: false,
                email_verification_token: Some("verify".to_string()),
                email_verification_expires: Some(now),
                password_reset_token: Some("reset".to_string()),
                password_reset_expires: Some(now),
                pending_email: Some(EmailAddress::new_unchecked("new@example.com")),
                email_unsubscribe_token: Some(Uuid::new_v4()),
                session_epoch: 2,
                release_notes_seen_version: Some("0.18.0".to_string()),
                terms_notice_version: NaiveDate::from_ymd_opt(2026, 12, 1),
                ..UserBase::default()
            },
        }
    }

    /// What `update_user` / `admin_update_user` do to a request body: only the
    /// client-editable fields survive, everything server-owned comes from the
    /// stored user, and the write is pinned to the stored user's id.
    #[test]
    fn a_request_body_changes_only_client_editable_fields() {
        let existing = stored_user();

        let mut body = serde_json::to_value(&existing).unwrap();
        body["id"] = serde_json::json!(Uuid::new_v4());
        body["created_at"] = serde_json::json!(Utc::now());
        body["email"] = serde_json::json!("attacker@example.com");
        body["email_verified"] = serde_json::json!(true);
        body["has_password"] = serde_json::json!(false);
        body["permissions"] = serde_json::json!(UserOrgPermissions::Member);
        body["email_settings"]["discovery_digest"] = serde_json::json!(false);
        let mut request: User = serde_json::from_value(body).unwrap();

        request.set_id(existing.id);
        request.set_created_at(existing.created_at);
        request.preserve_immutable_fields(&existing);

        let mut expected = existing.base.clone();
        expected.permissions = UserOrgPermissions::Member;
        expected.email_settings.discovery_digest = false;

        assert_eq!(request.id, existing.id);
        assert_eq!(request.created_at, existing.created_at);
        assert_eq!(request.base, expected);
    }
}
