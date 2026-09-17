use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::license::types::LicenseKeyType;
use crate::server::{
    billing::types::base::{BillingPlan, PlanStatus},
    organizations::r#impl::base::{Organization, OrganizationBase},
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        events::types::OnboardingOperationDiscriminants,
        storage::traits::{Entity, SqlValue, Storable},
    },
};

/// CSV row representation for Organization export (excludes sensitive billing data)
#[derive(Serialize)]
pub struct OrganizationCsvRow {
    pub id: Uuid,
    pub name: String,
    pub plan_status: Option<PlanStatus>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Organization {
    const HAS_SCD2: bool = false;
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
                    last_paused_at,
                    trial_extended_used,
                    last_downgrade_at,
                    last_downgrade_from_plan,
                    last_discount_at,
                    discount_save_offer_percent_off,
                    discount_save_offer_active_until,
                    next_renewal_at,
                    brevo_company_id,
                    notifications,
                    use_case,
                    license_paid_through,
                    license_last_checked_in_at,
                    license_key_version,
                    license_key_issued_at,
                    license_key_type,
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
                "last_paused_at",
                "trial_extended_used",
                "last_downgrade_at",
                "last_downgrade_from_plan",
                "last_discount_at",
                "discount_save_offer_percent_off",
                "discount_save_offer_active_until",
                "next_renewal_at",
                "brevo_company_id",
                "notifications",
                "use_case",
                "license_paid_through",
                "license_last_checked_in_at",
                "license_key_version",
                "license_key_issued_at",
                "license_key_type",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::String(name),
                SqlValue::OptionalString(stripe_customer_id),
                SqlValue::OptionBillingPlan(plan),
                SqlValue::OptionalString(plan_status.map(|s| s.to_string())),
                SqlValue::OnboardingOperation(onboarding),
                SqlValue::Bool(has_payment_method),
                SqlValue::OptionTimestamp(trial_end_date),
                SqlValue::OptionTimestamp(last_paused_at),
                SqlValue::Bool(trial_extended_used),
                SqlValue::OptionTimestamp(last_downgrade_at),
                SqlValue::OptionBillingPlan(last_downgrade_from_plan),
                SqlValue::OptionTimestamp(last_discount_at),
                SqlValue::OptionalI64(discount_save_offer_percent_off),
                SqlValue::OptionTimestamp(discount_save_offer_active_until),
                SqlValue::OptionTimestamp(next_renewal_at),
                SqlValue::OptionalString(brevo_company_id),
                SqlValue::OrgNotifications(notifications),
                SqlValue::OptionalString(Some(
                    serde_json::to_value(use_case)
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_else(|| "other".to_string()),
                )),
                SqlValue::OptionTimestamp(license_paid_through),
                SqlValue::OptionTimestamp(license_last_checked_in_at),
                SqlValue::I64(license_key_version),
                SqlValue::OptionTimestamp(license_key_issued_at),
                SqlValue::OptionalString(license_key_type.map(|t| t.to_string())),
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
        let onboarding: Vec<OnboardingOperationDiscriminants> = raw
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();

        let last_downgrade_from_plan: Option<BillingPlan> = row
            .try_get::<Option<serde_json::Value>, _>("last_downgrade_from_plan")
            .unwrap_or(None)
            .and_then(|v| serde_json::from_value(v).ok());

        Ok(Organization {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: OrganizationBase {
                name: row.get("name"),
                stripe_customer_id: row.get("stripe_customer_id"),
                plan,
                plan_status: row
                    .try_get::<Option<String>, _>("plan_status")
                    .ok()
                    .flatten()
                    .and_then(|s| s.parse().ok()),
                onboarding,
                has_payment_method: row.get("has_payment_method"),
                trial_end_date: row.get("trial_end_date"),
                last_paused_at: row.try_get("last_paused_at").unwrap_or(None),
                trial_extended_used: row.try_get("trial_extended_used").unwrap_or(false),
                last_downgrade_at: row.try_get("last_downgrade_at").unwrap_or(None),
                last_downgrade_from_plan,
                last_discount_at: row.try_get("last_discount_at").unwrap_or(None),
                discount_save_offer_percent_off: row
                    .try_get("discount_save_offer_percent_off")
                    .unwrap_or(None),
                discount_save_offer_active_until: row
                    .try_get("discount_save_offer_active_until")
                    .unwrap_or(None),
                next_renewal_at: row.try_get("next_renewal_at").unwrap_or(None),
                brevo_company_id: row.get("brevo_company_id"),
                notifications: row
                    .try_get::<serde_json::Value, _>("notifications")
                    .ok()
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default(),
                use_case: row
                    .try_get::<Option<String>, _>("use_case")
                    .ok()
                    .flatten()
                    .and_then(|s| serde_json::from_value(serde_json::json!(s)).ok())
                    .unwrap_or_default(),
                license_paid_through: row.try_get("license_paid_through").unwrap_or(None),
                license_last_checked_in_at: row
                    .try_get("license_last_checked_in_at")
                    .unwrap_or(None),
                license_key_version: row.try_get("license_key_version").unwrap_or(0),
                license_key_issued_at: row.try_get("license_key_issued_at").unwrap_or(None),
                license_key_type: row
                    .try_get::<Option<String>, _>("license_key_type")
                    .ok()
                    .flatten()
                    .and_then(|s| s.parse::<LicenseKeyType>().ok()),
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
            plan_status: self.base.plan_status,
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
        self.base.plan_status = existing.base.plan_status;
        self.base.next_renewal_at = existing.base.next_renewal_at;
        // Onboarding state is server-managed
        self.base.onboarding = existing.base.onboarding.clone();
        // Brevo company ID is server-managed
        self.base.brevo_company_id = existing.base.brevo_company_id.clone();
        // License state is server-managed. The key version is never
        // serialized, so a PUT body always carries the default and would
        // otherwise reset it (reviving retired online keys).
        self.base.license_paid_through = existing.base.license_paid_through;
        self.base.license_last_checked_in_at = existing.base.license_last_checked_in_at;
        self.base.license_key_version = existing.base.license_key_version;
        self.base.license_key_issued_at = existing.base.license_key_issued_at;
        self.base.license_key_type = existing.base.license_key_type;
    }
}
