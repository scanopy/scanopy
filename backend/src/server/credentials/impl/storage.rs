use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use strum::IntoDiscriminant;
use uuid::Uuid;

use crate::server::{
    credentials::r#impl::{
        base::{Credential, CredentialBase},
        types::CredentialType,
    },
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::traits::{Entity, SqlValue, Storable},
    },
};

/// CSV row representation for Credential export
#[derive(Serialize)]
pub struct CredentialCsvRow {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub credential_type: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Credential {
    type BaseData = CredentialBase;

    fn table_name() -> &'static str {
        "credentials"
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
        let Self {
            id,
            created_at,
            updated_at,
            base:
                Self::BaseData {
                    organization_id,
                    name,
                    credential_type,
                    target_ips,
                    tags: _, // Stored in entity_tags junction table
                },
        } = self.clone();

        Ok((
            vec![
                "id",
                "organization_id",
                "name",
                "credential_type",
                "target_ips",
                "created_at",
                "updated_at",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Uuid(organization_id),
                SqlValue::String(name),
                SqlValue::CredentialType(credential_type),
                SqlValue::OptionalIpAddrArray(target_ips),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let credential_type_json: serde_json::Value = row.get("credential_type");
        let credential_type: CredentialType = serde_json::from_value(credential_type_json)?;

        // Read target_ips as Vec<IpNetwork> (INET[]) and convert to Vec<IpAddr>
        let target_ips: Option<Vec<ipnetwork::IpNetwork>> = row.get("target_ips");
        let target_ips = target_ips.map(|ips| ips.into_iter().map(|n| n.ip()).collect());

        Ok(Credential {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: CredentialBase {
                organization_id: row.get("organization_id"),
                name: row.get("name"),
                credential_type,
                target_ips,
                tags: Vec::new(), // Hydrated from entity_tags junction table
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

    type CsvRow = CredentialCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        CredentialCsvRow {
            id: self.id,
            organization_id: self.base.organization_id,
            name: self.base.name.clone(),
            credential_type: self.base.credential_type.discriminant().to_string(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Credential
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Credential";
    const ENTITY_NAME_PLURAL: &'static str = "Credentials";
    const ENTITY_DESCRIPTION: &'static str = "Credentials for network device discovery and management. Supports SNMP, Docker proxy, and other credential types.";

    fn entity_category() -> EntityCategory {
        EntityCategory::DiscoveryAndDaemons
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

    fn get_tags(&self) -> Option<&Vec<Uuid>> {
        Some(&self.base.tags)
    }

    fn set_tags(&mut self, tags: Vec<Uuid>) {
        self.base.tags = tags;
    }

    fn preserve_immutable_fields(&mut self, existing: &Self) {
        self.base
            .credential_type
            .merge_redacted_secrets(&existing.base.credential_type);
    }
}
