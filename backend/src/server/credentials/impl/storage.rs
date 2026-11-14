use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    credentials::r#impl::{
        base::{Credential, CredentialBase},
        types::{CredentialType, OsFamily},
    },
    shared::{
        entities::EntityDiscriminants,
        product_area::ProductArea,
        storage::traits::{Entity, SqlValue, Storable},
        types::entities::EntitySource,
    },
};

impl Storable for Credential {
    const HAS_SCD2: bool = false;
    type BaseData = CredentialBase;

    fn table_name() -> &'static str {
        "credentials"
    }

    fn search_predicates() -> &'static [&'static str] {
        &[
            "credentials.name ILIKE {}",
            "credentials.description ILIKE {}",
        ]
    }

    fn new(base: Self::BaseData) -> Self {
        Credential::new(base)
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error> {
        let Self {
            id,
            created_at,
            updated_at,
            is_default: _, // Computed from credential_type
            base:
                Self::BaseData {
                    organization_id,
                    name,
                    description,
                    credential_type,
                    daemon_os,
                    login_credential_id,
                    tags: _,              // Stored in entity_tags junction table
                    assigned_site_ids: _, // Stored in site_credentials junction table
                    host_assignments: _,  // Stored in host_credentials junction table
                    used_by: _,           // Derived from other rows' login_credential_id
                    source,
                },
        } = self.clone();

        Ok((
            vec![
                "id",
                "organization_id",
                "name",
                "description",
                "credential_type",
                "daemon_os",
                "login_credential_id",
                "source",
                "created_at",
                "updated_at",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Uuid(organization_id),
                SqlValue::String(name),
                SqlValue::OptionalString(description),
                SqlValue::CredentialType(credential_type),
                SqlValue::OptionalOsFamily(daemon_os),
                SqlValue::OptionalUuid(login_credential_id),
                SqlValue::EntitySource(source),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let credential_type_json: serde_json::Value = row.get("credential_type");
        let credential_type: CredentialType = serde_json::from_value(credential_type_json)?;
        let daemon_os: Option<OsFamily> = row
            .get::<Option<String>, _>("daemon_os")
            .map(|s| serde_json::from_str(&s))
            .transpose()?;

        let is_default = credential_type.is_default();
        Ok(Credential {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            is_default,
            base: CredentialBase {
                organization_id: row.get("organization_id"),
                name: row.get("name"),
                description: row.get("description"),
                credential_type,
                daemon_os,
                login_credential_id: row.get("login_credential_id"),
                tags: Vec::new(), // Hydrated from entity_tags junction table
                assigned_site_ids: Vec::new(), // Hydrated from site_credentials junction table
                host_assignments: Vec::new(), // Hydrated from host_credentials junction table
                used_by: Vec::new(), // Hydrated from login_credential_id
                source: serde_json::from_value(row.get::<serde_json::Value, _>("source"))
                    .map_err(|e| anyhow::anyhow!("Failed to deserialize source: {}", e))?,
            },
        })
    }
}

impl Entity for Credential {
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

    const ENTITY_TYPE: EntityDiscriminants = EntityDiscriminants::Credential;

    const ENTITY_NAME_SINGULAR: &'static str = "Credential";
    const ENTITY_NAME_PLURAL: &'static str = "Credentials";
    const ENTITY_DESCRIPTION: &'static str =
        "Credentials the daemon uses to collect from integrations during discovery.";

    fn product_area() -> ProductArea {
        ProductArea::Discovery
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

    fn get_tags(&self) -> Option<&Vec<Uuid>> {
        Some(&self.base.tags)
    }

    fn set_tags(&mut self, tags: Vec<Uuid>) {
        self.base.tags = tags;
    }

    fn set_source(&mut self, source: EntitySource) {
        self.base.source = source;
    }
}

impl Credential {
    pub(crate) fn merge_credential_type(&mut self, existing: &Self) {
        self.base
            .credential_type
            .merge_redacted_secrets(&existing.base.credential_type);
    }
}
