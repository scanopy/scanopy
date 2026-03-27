use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    billing::types::base::BillingPlan,
    organizations::r#impl::base::{Organization, OrganizationBase},
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        events::types::OnboardingOperation,
        storage::traits::{Entity, SqlValue, Storable},
    },
};

/// CSV row representation for Organization export (excludes sensitive billing data)
#[derive(Serialize)]
pub struct OrganizationCsvRow {
    pub id: Uuid,
    pub name: String,
    pub plan_status: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Organization {
    type BaseData = OrganizationBase;

    fn table_name() -> &'static str {
        "organizations"
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
                    name,
                    stripe_customer_id,
                    plan,
                    plan_status,
                    onboarding,
                    has_payment_method,
                    trial_end_date,
                    brevo_company_id,
                    plan_limit_notifications,
                    use_case,
                },
        } = self.clone();

        Ok((
            vec![
                "id",
                "created_at",
                "updated_at",
                "name",
                "stripe_customer_id",
                "plan",
                "plan_status",
                "onboarding",
                "has_payment_method",
                "trial_end_date",
                "brevo_company_id",
                "plan_limit_notifications",
                "use_case",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::String(name),
                SqlValue::OptionalString(stripe_customer_id),
                SqlValue::OptionBillingPlan(plan),
                SqlValue::OptionalString(plan_status),
                SqlValue::OnboardingOperation(onboarding),
                SqlValue::Bool(has_payment_method),
                SqlValue::OptionTimestamp(trial_end_date),
                SqlValue::OptionalString(brevo_company_id),
                SqlValue::PlanLimitNotifications(plan_limit_notifications),
                SqlValue::OptionalString(use_case),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let plan: Option<BillingPlan> = row
            .try_get::<Option<serde_json::Value>, _>("plan")
            .unwrap_or(None)
            .and_then(|v| serde_json::from_value(v).ok());

        let raw: Vec<serde_json::Value> =
            serde_json::from_value(row.get::<serde_json::Value, _>("onboarding"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize onboarding: {}", e))?;
        let onboarding: Vec<OnboardingOperation> = raw
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();

        Ok(Organization {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: OrganizationBase {
                name: row.get("name"),
                stripe_customer_id: row.get("stripe_customer_id"),
                plan,
                plan_status: row.get("plan_status"),
                onboarding,
                has_payment_method: row.get("has_payment_method"),
                trial_end_date: row.get("trial_end_date"),
                brevo_company_id: row.get("brevo_company_id"),
                plan_limit_notifications: row
                    .try_get::<serde_json::Value, _>("plan_limit_notifications")
                    .ok()
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default(),
                use_case: row.try_get("use_case").ok().flatten(),
            },
        })
    }
}

impl Entity for Organization {
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

    type CsvRow = OrganizationCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        OrganizationCsvRow {
            id: self.id,
            name: self.base.name.clone(),
            plan_status: self.base.plan_status.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Organization
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Organization";
    const ENTITY_NAME_PLURAL: &'static str = "Organizations";
    const ENTITY_DESCRIPTION: &'static str = "Manage organization settings.";

    fn entity_category() -> EntityCategory {
        EntityCategory::OrganizationsAndUsers
    }

    fn network_id(&self) -> Option<Uuid> {
        None
    }

    fn organization_id(&self) -> Option<Uuid> {
        None
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn set_updated_at(&mut self, time: DateTime<Utc>) {
        self.updated_at = time;
    }

    fn preserve_immutable_fields(&mut self, existing: &Self) {
        // Billing fields are managed by Stripe integration, not user-editable
        self.base.stripe_customer_id = existing.base.stripe_customer_id.clone();
        self.base.plan = existing.base.plan;
        self.base.plan_status = existing.base.plan_status.clone();
        // Onboarding state is server-managed
        self.base.onboarding = existing.base.onboarding.clone();
        // Brevo company ID is server-managed
        self.base.brevo_company_id = existing.base.brevo_company_id.clone();
    }
}
