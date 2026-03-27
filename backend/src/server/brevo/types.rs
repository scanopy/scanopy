use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Brevo contact attributes for Scanopy users.
/// Brevo uses UPPERCASE attribute names in an `attributes` map.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContactAttributes {
    pub email: Option<String>,
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub job_title: Option<String>,
    pub scanopy_user_id: Option<String>,
    pub scanopy_role: Option<String>,
    pub scanopy_referral_source: Option<String>,
    pub scanopy_use_case: Option<String>,
    pub scanopy_signup_date: Option<String>,
    pub scanopy_last_login_date: Option<String>,
    /// Brevo built-in field (not a custom attribute) — controls email campaign eligibility.
    /// true = blocklisted from campaigns, false = can receive campaigns.
    pub email_blacklisted: Option<bool>,
    pub scanopy_marketing_opt_in: Option<bool>,
    pub scanopy_marketing_opt_in_date: Option<String>,
}

impl ContactAttributes {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    pub fn with_user_id(mut self, user_id: Uuid) -> Self {
        self.scanopy_user_id = Some(user_id.to_string());
        self
    }

    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.scanopy_role = Some(role.into());
        self
    }

    pub fn with_referral_source(mut self, source: impl Into<String>) -> Self {
        self.scanopy_referral_source = Some(source.into());
        self
    }

    pub fn with_use_case(mut self, use_case: impl Into<String>) -> Self {
        self.scanopy_use_case = Some(use_case.into());
        self
    }

    pub fn with_signup_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_signup_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_last_login_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_last_login_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_job_title(mut self, title: impl Into<String>) -> Self {
        self.job_title = Some(title.into());
        self
    }

    pub fn with_email_blacklisted(mut self, blacklisted: bool) -> Self {
        self.email_blacklisted = Some(blacklisted);
        self
    }

    pub fn with_marketing_opt_in(mut self, opt_in: bool) -> Self {
        self.scanopy_marketing_opt_in = Some(opt_in);
        self
    }

    pub fn with_marketing_opt_in_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_marketing_opt_in_date = Some(date.to_rfc3339());
        self
    }

    /// Convert to Brevo API attributes map (UPPERCASE keys)
    pub fn to_attributes(&self) -> HashMap<String, serde_json::Value> {
        let mut attrs = HashMap::new();

        if let Some(v) = &self.firstname {
            attrs.insert("FIRSTNAME".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.lastname {
            attrs.insert("LASTNAME".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.job_title {
            attrs.insert("JOB_TITLE".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_user_id {
            attrs.insert("SCANOPY_USER_ID".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_role {
            attrs.insert("SCANOPY_ROLE".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_referral_source {
            attrs.insert("SCANOPY_REFERRAL_SOURCE".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_use_case {
            attrs.insert("SCANOPY_USE_CASE".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_signup_date {
            attrs.insert("SCANOPY_SIGNUP_DATE".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_last_login_date {
            attrs.insert("SCANOPY_LAST_LOGIN_DATE".to_string(), serde_json::json!(v));
        }
        if let Some(v) = self.scanopy_marketing_opt_in {
            attrs.insert("SCANOPY_MARKETING_OPT_IN".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_marketing_opt_in_date {
            attrs.insert(
                "SCANOPY_MARKETING_OPT_IN_DATE".to_string(),
                serde_json::json!(v),
            );
        }
        attrs
    }
}

/// Brevo company attributes for Scanopy organizations.
/// Attributes are free-form key/value in an `attributes` map.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompanyAttributes {
    pub name: Option<String>,
    pub scanopy_org_id: Option<String>,
    pub scanopy_org_type: Option<String>,
    pub scanopy_company_size: Option<String>,
    pub scanopy_plan_type: Option<String>,
    pub scanopy_plan_status: Option<String>,
    pub scanopy_mrr: Option<i64>,
    pub scanopy_network_count: Option<i64>,
    pub scanopy_host_count: Option<i64>,
    pub scanopy_user_count: Option<i64>,
    pub scanopy_network_limit: Option<i64>,
    pub scanopy_seat_limit: Option<i64>,
    pub scanopy_created_date: Option<String>,
    pub scanopy_last_discovery_date: Option<String>,
    pub scanopy_discovery_count: Option<i64>,
    pub scanopy_first_daemon_date: Option<String>,
    pub scanopy_trial_started_date: Option<String>,
    pub scanopy_checkout_completed_date: Option<String>,
    pub scanopy_second_network_date: Option<String>,
    pub scanopy_first_tag_date: Option<String>,
    pub scanopy_first_group_date: Option<String>,
    pub scanopy_first_api_key_date: Option<String>,
    pub scanopy_first_credential_date: Option<String>,
    pub scanopy_first_snmp_credential_date: Option<String>,
    pub scanopy_first_invite_sent_date: Option<String>,
    pub scanopy_first_invite_accepted_date: Option<String>,
    pub scanopy_first_discovery_completed_date: Option<String>,
    pub scanopy_first_host_discovered_date: Option<String>,
    pub scanopy_first_topology_rebuild_date: Option<String>,
}

impl CompanyAttributes {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_org_id(mut self, org_id: Uuid) -> Self {
        self.scanopy_org_id = Some(org_id.to_string());
        self
    }

    pub fn with_org_type(mut self, org_type: impl Into<String>) -> Self {
        self.scanopy_org_type = Some(org_type.into());
        self
    }

    pub fn with_company_size(mut self, size: impl Into<String>) -> Self {
        self.scanopy_company_size = Some(size.into());
        self
    }

    pub fn with_plan_type(mut self, plan_type: impl Into<String>) -> Self {
        self.scanopy_plan_type = Some(plan_type.into());
        self
    }

    pub fn with_plan_status(mut self, status: impl Into<String>) -> Self {
        self.scanopy_plan_status = Some(status.into());
        self
    }

    pub fn with_mrr(mut self, mrr_cents: i64) -> Self {
        self.scanopy_mrr = Some(mrr_cents);
        self
    }

    pub fn with_network_count(mut self, count: i64) -> Self {
        self.scanopy_network_count = Some(count);
        self
    }

    pub fn with_host_count(mut self, count: i64) -> Self {
        self.scanopy_host_count = Some(count);
        self
    }

    pub fn with_user_count(mut self, count: i64) -> Self {
        self.scanopy_user_count = Some(count);
        self
    }

    pub fn with_network_limit(mut self, limit: i64) -> Self {
        self.scanopy_network_limit = Some(limit);
        self
    }

    pub fn with_seat_limit(mut self, limit: i64) -> Self {
        self.scanopy_seat_limit = Some(limit);
        self
    }

    pub fn with_created_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_created_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_last_discovery_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_last_discovery_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_discovery_count(mut self, count: i64) -> Self {
        self.scanopy_discovery_count = Some(count);
        self
    }

    pub fn with_first_discovery_completed_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_first_discovery_completed_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_first_host_discovered_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_first_host_discovered_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_first_daemon_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_first_daemon_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_first_topology_rebuild_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_first_topology_rebuild_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_trial_started_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_trial_started_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_checkout_completed_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_checkout_completed_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_second_network_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_second_network_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_first_tag_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_first_tag_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_first_group_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_first_group_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_first_api_key_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_first_api_key_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_first_credential_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_first_credential_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_first_snmp_credential_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_first_snmp_credential_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_first_invite_sent_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_first_invite_sent_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_first_invite_accepted_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_first_invite_accepted_date = Some(date.to_rfc3339());
        self
    }

    /// Convert to Brevo API attributes map
    pub fn to_attributes(&self) -> HashMap<String, serde_json::Value> {
        let mut attrs = HashMap::new();

        if let Some(v) = &self.scanopy_org_id {
            attrs.insert("scanopy_org_id".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_org_type {
            attrs.insert("scanopy_org_type".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_company_size {
            attrs.insert("scanopy_company_size".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_plan_type {
            attrs.insert("scanopy_plan_type".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_plan_status {
            attrs.insert("scanopy_plan_status".to_string(), serde_json::json!(v));
        }
        if let Some(v) = self.scanopy_mrr {
            attrs.insert("scanopy_mrr".to_string(), serde_json::json!(v));
        }
        if let Some(v) = self.scanopy_network_count {
            attrs.insert("scanopy_network_count".to_string(), serde_json::json!(v));
        }
        if let Some(v) = self.scanopy_host_count {
            attrs.insert("scanopy_host_count".to_string(), serde_json::json!(v));
        }
        if let Some(v) = self.scanopy_user_count {
            attrs.insert("scanopy_user_count".to_string(), serde_json::json!(v));
        }
        if let Some(v) = self.scanopy_network_limit {
            attrs.insert("scanopy_network_limit".to_string(), serde_json::json!(v));
        }
        if let Some(v) = self.scanopy_seat_limit {
            attrs.insert("scanopy_seat_limit".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_created_date {
            attrs.insert("scanopy_created_date".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_last_discovery_date {
            attrs.insert(
                "scanopy_last_discovery_date".to_string(),
                serde_json::json!(v),
            );
        }
        if let Some(v) = self.scanopy_discovery_count {
            attrs.insert("scanopy_discovery_count".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_first_daemon_date {
            attrs.insert(
                "scanopy_first_daemon_date".to_string(),
                serde_json::json!(v),
            );
        }
        if let Some(v) = &self.scanopy_trial_started_date {
            attrs.insert(
                "scanopy_trial_started_date".to_string(),
                serde_json::json!(v),
            );
        }
        if let Some(v) = &self.scanopy_checkout_completed_date {
            attrs.insert(
                "scanopy_checkout_completed_date".to_string(),
                serde_json::json!(v),
            );
        }
        if let Some(v) = &self.scanopy_second_network_date {
            attrs.insert(
                "scanopy_second_network_date".to_string(),
                serde_json::json!(v),
            );
        }
        if let Some(v) = &self.scanopy_first_tag_date {
            attrs.insert("scanopy_first_tag_date".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_first_group_date {
            attrs.insert("scanopy_first_group_date".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_first_api_key_date {
            attrs.insert(
                "scanopy_first_api_key_date".to_string(),
                serde_json::json!(v),
            );
        }
        if let Some(v) = &self.scanopy_first_credential_date {
            attrs.insert(
                "scanopy_first_credential_date".to_string(),
                serde_json::json!(v),
            );
        }
        if let Some(v) = &self.scanopy_first_snmp_credential_date {
            attrs.insert(
                "scanopy_first_snmp_credential_date".to_string(),
                serde_json::json!(v),
            );
        }
        if let Some(v) = &self.scanopy_first_invite_sent_date {
            attrs.insert(
                "scanopy_first_invite_sent_date".to_string(),
                serde_json::json!(v),
            );
        }
        if let Some(v) = &self.scanopy_first_invite_accepted_date {
            attrs.insert(
                "scanopy_first_invite_accepted_date".to_string(),
                serde_json::json!(v),
            );
        }
        if let Some(v) = &self.scanopy_first_discovery_completed_date {
            attrs.insert(
                "scanopy_first_discovery_date".to_string(),
                serde_json::json!(v),
            );
        }
        if let Some(v) = &self.scanopy_first_host_discovered_date {
            attrs.insert(
                "scanopy_first_host_discovered_date".to_string(),
                serde_json::json!(v),
            );
        }
        if let Some(v) = &self.scanopy_first_topology_rebuild_date {
            attrs.insert(
                "scanopy_first_topology_rebuild_date".to_string(),
                serde_json::json!(v),
            );
        }
        attrs
    }
}

// ============================================================================
// Brevo API request/response types
// ============================================================================

/// POST /contacts - create or update contact
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateContactRequest {
    pub email: String,
    pub attributes: HashMap<String, serde_json::Value>,
    pub update_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_blacklisted: Option<bool>,
}

/// PUT /contacts/{email} - update contact attributes
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateContactRequest {
    pub attributes: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_blacklisted: Option<bool>,
}

/// Response from POST /contacts (201 on create)
#[derive(Debug, Clone, Deserialize)]
pub struct CreateContactResponse {
    pub id: i64,
}

/// POST /companies - create company
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCompanyRequest {
    pub name: String,
    pub attributes: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_contacts_ids: Option<Vec<i64>>,
}

/// PATCH /companies/{id} - update company
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCompanyRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<HashMap<String, serde_json::Value>>,
}

/// Response from POST /companies
#[derive(Debug, Clone, Deserialize)]
pub struct CompanyResponse {
    pub id: String,
}

/// Response from GET /companies (list/filter)
#[derive(Debug, Clone, Deserialize)]
pub struct CompanyListResponse {
    pub items: Option<Vec<CompanyItem>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompanyItem {
    pub id: String,
}

/// PATCH /companies/link-unlink/{id}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkUnlinkRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_contacts_ids: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unlink_contacts_ids: Option<Vec<i64>>,
}

/// POST /events - track event
#[derive(Debug, Clone, Serialize)]
pub struct TrackEventRequest {
    pub event_name: String,
    pub identifiers: EventIdentifiers,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_properties: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_properties: Option<serde_json::Value>,
}

/// POST /crm/deals - create deal
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDealRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_contacts_ids: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_companies_ids: Option<Vec<String>>,
}

/// Response from POST /crm/deals
#[derive(Debug, Clone, Deserialize)]
pub struct CreateDealResponse {
    pub id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventIdentifiers {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_id: Option<String>,
}

/// POST /contacts/lists/{listId}/contacts/add
#[derive(Debug, Clone, Serialize)]
pub struct AddContactsToListRequest {
    pub emails: Vec<String>,
}

/// POST /contacts/lists/{listId}/contacts/remove
#[derive(Debug, Clone, Serialize)]
pub struct RemoveContactsFromListRequest {
    pub emails: Vec<String>,
}
