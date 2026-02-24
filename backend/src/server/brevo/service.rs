use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    brevo::{
        client::BrevoClient,
        types::{CompanyAttributes, ContactAttributes},
    },
    daemons::{r#impl::base::Daemon, service::DaemonService},
    hosts::{r#impl::base::Host, service::HostService},
    networks::{r#impl::Network, service::NetworkService},
    organizations::{r#impl::base::Organization, service::OrganizationService},
    shared::{
        events::types::{AuthOperation, Event, TelemetryEvent, TelemetryOperation},
        services::traits::CrudService,
        storage::filter::StorableFilter,
    },
    snmp_credentials::{r#impl::base::SnmpCredential, service::SnmpCredentialService},
    tags::{r#impl::base::Tag, service::TagService},
    user_api_keys::{r#impl::base::UserApiKey, service::UserApiKeyService},
    users::{r#impl::base::User, r#impl::permissions::UserOrgPermissions, service::UserService},
};
use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

/// Brevo list ID for "App Users - Product Updates" (all app signups)
const BREVO_PRODUCT_UPDATES_LIST_ID: i64 = 9;
/// Brevo list ID for "App Users - Marketing" (explicit opt-in only)
const BREVO_MARKETING_LIST_ID: i64 = 10;
/// Brevo list ID for "App Users - Onboarding" (all app signups)
const BREVO_ONBOARDING_LIST_ID: i64 = 12;

/// Service for syncing data to Brevo CRM
pub struct BrevoService {
    pub client: Arc<BrevoClient>,
    network_service: Arc<NetworkService>,
    host_service: Arc<HostService>,
    user_service: Arc<UserService>,
    organization_service: Arc<OrganizationService>,
    daemon_service: Arc<DaemonService>,
    tag_service: Arc<TagService>,
    user_api_key_service: Arc<UserApiKeyService>,
    snmp_credential_service: Arc<SnmpCredentialService>,
}

impl BrevoService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        api_key: String,
        network_service: Arc<NetworkService>,
        host_service: Arc<HostService>,
        user_service: Arc<UserService>,
        organization_service: Arc<OrganizationService>,
        daemon_service: Arc<DaemonService>,
        tag_service: Arc<TagService>,
        user_api_key_service: Arc<UserApiKeyService>,
        snmp_credential_service: Arc<SnmpCredentialService>,
    ) -> Self {
        Self {
            client: Arc::new(BrevoClient::new(api_key)),
            network_service,
            host_service,
            user_service,
            organization_service,
            daemon_service,
            tag_service,
            user_api_key_service,
            snmp_credential_service,
        }
    }

    /// Handle events and sync to Brevo
    pub async fn handle_event(&self, event: &Event) -> Result<()> {
        match event {
            Event::Telemetry(telemetry) => self.handle_telemetry_event(telemetry).await,
            Event::Auth(auth) => {
                if auth.operation == AuthOperation::LoginSuccess
                    && let AuthenticatedEntity::User { email, user_id, .. } = &auth.authentication
                {
                    self.update_contact_last_login(email.to_string(), *user_id)
                        .await?;
                }
                Ok(())
            }
            Event::Discovery(discovery) => {
                if discovery.phase
                    == crate::daemon::discovery::types::base::DiscoveryPhase::Scanning
                    && let Some(org_id) = self.get_org_id_from_network(&discovery.network_id).await
                {
                    self.update_company_last_discovery(org_id).await?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    async fn handle_telemetry_event(&self, event: &TelemetryEvent) -> Result<()> {
        match &event.operation {
            TelemetryOperation::OrgCreated => {
                self.handle_org_created(event).await?;
            }
            TelemetryOperation::CheckoutStarted => {
                self.handle_checkout_started(event).await?;
            }
            TelemetryOperation::CheckoutCompleted => {
                self.handle_checkout_completed(event).await?;
            }
            TelemetryOperation::TrialStarted => {
                self.handle_trial_started(event).await?;
            }
            TelemetryOperation::TrialEnded => {
                self.handle_trial_ended(event).await?;
            }
            TelemetryOperation::SubscriptionCancelled => {
                self.handle_subscription_cancelled(event).await?;
            }
            TelemetryOperation::TrialWillEnd => {
                self.handle_trial_will_end(event).await?;
            }
            TelemetryOperation::PlanChanged => {
                self.handle_plan_changed(event).await?;
            }
            TelemetryOperation::PaymentFailed => {
                self.update_company_by_org(
                    event.organization_id,
                    CompanyAttributes::new().with_plan_status("payment_failed"),
                )
                .await?;
            }
            TelemetryOperation::PaymentActionRequired => {
                self.update_company_by_org(
                    event.organization_id,
                    CompanyAttributes::new().with_plan_status("payment_action_required"),
                )
                .await?;
            }
            TelemetryOperation::PaymentRecovered => {
                self.update_company_by_org(
                    event.organization_id,
                    CompanyAttributes::new().with_plan_status("active"),
                )
                .await?;
            }
            TelemetryOperation::FirstDaemonRegistered => {
                self.handle_first_daemon_registered(event).await?;
            }
            TelemetryOperation::FirstTopologyRebuild => {
                self.handle_first_topology_rebuild(event).await?;
            }
            TelemetryOperation::FirstDiscoveryCompleted => {
                self.handle_first_discovery_completed(event).await?;
            }
            TelemetryOperation::SecondNetworkCreated
            | TelemetryOperation::FirstHostDiscovered
            | TelemetryOperation::FirstTagCreated
            | TelemetryOperation::FirstGroupCreated
            | TelemetryOperation::FirstUserApiKeyCreated
            | TelemetryOperation::FirstSnmpCredentialCreated
            | TelemetryOperation::InviteSent
            | TelemetryOperation::InviteAccepted => {
                self.handle_engagement_event(event).await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_engagement_event(&self, event: &TelemetryEvent) -> Result<()> {
        let mut company_attrs = CompanyAttributes::new();

        match &event.operation {
            TelemetryOperation::SecondNetworkCreated => {
                company_attrs = company_attrs.with_second_network_date(event.timestamp);
            }
            TelemetryOperation::FirstTagCreated => {
                company_attrs = company_attrs.with_first_tag_date(event.timestamp);
            }
            TelemetryOperation::FirstGroupCreated => {
                company_attrs = company_attrs.with_first_group_date(event.timestamp);
            }
            TelemetryOperation::FirstUserApiKeyCreated => {
                company_attrs = company_attrs.with_first_api_key_date(event.timestamp);
            }
            TelemetryOperation::FirstSnmpCredentialCreated => {
                company_attrs = company_attrs.with_first_snmp_credential_date(event.timestamp);
            }
            TelemetryOperation::InviteSent => {
                company_attrs = company_attrs.with_first_invite_sent_date(event.timestamp);
            }
            TelemetryOperation::InviteAccepted => {
                company_attrs = company_attrs.with_first_invite_accepted_date(event.timestamp);
            }
            TelemetryOperation::FirstHostDiscovered => {
                company_attrs = company_attrs.with_first_host_discovered_date(event.timestamp)
            }
            _ => return Ok(()),
        }

        self.update_company_by_org(event.organization_id, company_attrs)
            .await?;

        tracing::debug!(
            organization_id = %event.organization_id,
            operation = %event.operation,
            "Updated Brevo company: engagement milestone"
        );
        Ok(())
    }

    /// Handle org created - create contact and company, store company ID on org.
    async fn handle_org_created(&self, event: &TelemetryEvent) -> Result<()> {
        let (email, user_id) = match &event.authentication {
            AuthenticatedEntity::User { email, user_id, .. } => (email.clone(), *user_id),
            _ => return Ok(()),
        };

        let org_name = event
            .metadata
            .get("org_name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");
        let use_case = event
            .metadata
            .get("use_case")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let company_size = event
            .metadata
            .get("company_size")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let job_title = event
            .metadata
            .get("job_title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let marketing_opt_in = event
            .metadata
            .get("marketing_opt_in")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let referral_source = event
            .metadata
            .get("referral_source")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let referral_source_other = event
            .metadata
            .get("referral_source_other")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Format referral source for HubSpot (combine "other" with free text)
        let formatted_referral_source = referral_source.map(|source| {
            if source == "other" {
                if let Some(ref other_text) = referral_source_other {
                    format!("other: {}", other_text)
                } else {
                    source
                }
            } else {
                source
            }
        });

        let mut contact_attrs = ContactAttributes::new()
            .with_email(email.to_string())
            .with_user_id(user_id)
            .with_role("owner")
            .with_signup_date(event.timestamp)
            .with_last_login_date(event.timestamp)
            .with_email_blacklisted(false)
            .with_marketing_opt_in(marketing_opt_in)
            .with_marketing_opt_in_date(event.timestamp);

        if let Some(use_case) = &use_case {
            contact_attrs = contact_attrs.with_use_case(use_case);
        }
        if let Some(title) = job_title {
            contact_attrs = contact_attrs.with_job_title(title);
        }
        if let Some(ref source) = formatted_referral_source {
            contact_attrs = contact_attrs.with_referral_source(source);
        }

        let org_filter = StorableFilter::<Network>::new_from_org_id(&event.organization_id);
        let network_count = self.network_service.get_all(org_filter).await?.len();

        let mut company_attrs = CompanyAttributes::new()
            .with_name(org_name)
            .with_org_id(event.organization_id)
            .with_created_date(event.timestamp)
            .with_network_count(network_count as i64)
            .with_host_count(0)
            .with_user_count(1);

        if let Some(use_case) = use_case {
            company_attrs = company_attrs.with_org_type(use_case);
        }
        if let Some(size) = company_size {
            company_attrs = company_attrs.with_company_size(size);
        }

        let (_contact_id, company_id) = self
            .client
            .sync_contact_and_company(email.as_ref(), contact_attrs, org_name, company_attrs)
            .await?;

        // Add to "Product Updates" and "Onboarding" lists (all signups)
        if let Err(e) = self
            .client
            .add_contacts_to_list(BREVO_PRODUCT_UPDATES_LIST_ID, vec![email.to_string()])
            .await
        {
            tracing::warn!(error = %e, "Failed to add contact to Product Updates list");
        }
        if let Err(e) = self
            .client
            .add_contacts_to_list(BREVO_ONBOARDING_LIST_ID, vec![email.to_string()])
            .await
        {
            tracing::warn!(error = %e, "Failed to add contact to Onboarding list");
        }

        // Add to "Marketing" list only if opted in
        if marketing_opt_in
            && let Err(e) = self
                .client
                .add_contacts_to_list(BREVO_MARKETING_LIST_ID, vec![email.to_string()])
                .await
        {
            tracing::warn!(error = %e, "Failed to add contact to Marketing list");
        }

        // Store the company ID on the organization
        if let Some(mut org) = self
            .organization_service
            .get_by_id(&event.organization_id)
            .await?
        {
            org.base.brevo_company_id = Some(company_id.clone());
            self.organization_service
                .update(&mut org, event.authentication.clone())
                .await?;
        }

        // Track event for automation
        if let Err(e) = self
            .client
            .track_event("org_created", email.as_ref(), None)
            .await
        {
            tracing::warn!(error = %e, "Failed to track org_created event in Brevo");
        }

        tracing::info!(
            organization_id = %event.organization_id,
            brevo_company_id = %company_id,
            email = %email,
            "Synced new organization to Brevo"
        );

        Ok(())
    }

    /// Get stored Brevo company ID for an org, if it exists
    async fn get_brevo_company_id(&self, org_id: Uuid) -> Result<Option<String>> {
        let org = self.organization_service.get_by_id(&org_id).await?;
        Ok(org.and_then(|o| o.base.brevo_company_id))
    }

    /// Update Brevo company using stored ID. Skips if no ID stored.
    async fn update_company_by_org(&self, org_id: Uuid, attrs: CompanyAttributes) -> Result<()> {
        match self.get_brevo_company_id(org_id).await? {
            Some(id) => {
                self.client.update_company(&id, attrs).await?;
                Ok(())
            }
            None => {
                tracing::debug!(
                    organization_id = %org_id,
                    "No Brevo company ID stored - skipping update"
                );
                Ok(())
            }
        }
    }

    /// Get owner email for an org (for event tracking)
    async fn get_owner_email(&self, org_id: Uuid) -> Option<String> {
        let filter = StorableFilter::<User>::new_from_org_id(&org_id)
            .user_permissions(&UserOrgPermissions::Owner);
        if let Ok(owners) = self.user_service.get_all(filter).await {
            owners.first().map(|o| o.base.email.to_string())
        } else {
            None
        }
    }

    async fn handle_checkout_started(&self, event: &TelemetryEvent) -> Result<()> {
        let plan_name = event
            .metadata
            .get("plan_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let company_attrs = CompanyAttributes::new()
            .with_plan_type(plan_name)
            .with_plan_status("checkout_started");
        self.update_company_by_org(event.organization_id, company_attrs)
            .await?;

        tracing::debug!(
            organization_id = %event.organization_id,
            plan = %plan_name,
            "Updated Brevo company: checkout started"
        );
        Ok(())
    }

    async fn handle_checkout_completed(&self, event: &TelemetryEvent) -> Result<()> {
        let plan_name = event
            .metadata
            .get("plan_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let has_trial = event
            .metadata
            .get("has_trial")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let company_attrs = CompanyAttributes::new()
            .with_plan_type(plan_name)
            .with_plan_status(if has_trial { "trialing" } else { "active" })
            .with_checkout_completed_date(event.timestamp);

        self.update_company_by_org(event.organization_id, company_attrs)
            .await?;

        let network_limit = event
            .metadata
            .get("included_networks")
            .and_then(|v| v.as_u64())
            .map(|n| n as i64);
        let seat_limit = event
            .metadata
            .get("included_seats")
            .and_then(|v| v.as_u64())
            .map(|n| n as i64);

        if network_limit.is_some() || seat_limit.is_some() {
            self.sync_plan_limits(event.organization_id, network_limit, seat_limit)
                .await?;
        }

        // Track event for automation
        if let Some(email) = self.get_owner_email(event.organization_id).await
            && let Err(e) = self
                .client
                .track_event("checkout_completed", &email, Some(event.metadata.clone()))
                .await
        {
            tracing::warn!(error = %e, "Failed to track checkout_completed event in Brevo");
        }

        tracing::info!(
            organization_id = %event.organization_id,
            plan = %plan_name,
            "Updated Brevo: checkout completed"
        );
        Ok(())
    }

    async fn handle_trial_started(&self, event: &TelemetryEvent) -> Result<()> {
        let company_attrs = CompanyAttributes::new()
            .with_plan_status("trialing")
            .with_trial_started_date(event.timestamp);

        self.update_company_by_org(event.organization_id, company_attrs)
            .await?;

        if let Some(email) = self.get_owner_email(event.organization_id).await
            && let Err(e) = self.client.track_event("trial_started", &email, None).await
        {
            tracing::warn!(error = %e, "Failed to track trial_started event in Brevo");
        }

        tracing::debug!(
            organization_id = %event.organization_id,
            "Updated Brevo: trial started"
        );
        Ok(())
    }

    async fn handle_trial_ended(&self, event: &TelemetryEvent) -> Result<()> {
        let converted = event
            .metadata
            .get("converted")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let company_attrs = CompanyAttributes::new().with_plan_status(if converted {
            "active"
        } else {
            "trial_ended"
        });

        self.update_company_by_org(event.organization_id, company_attrs)
            .await?;

        if let Some(email) = self.get_owner_email(event.organization_id).await
            && let Err(e) = self
                .client
                .track_event("trial_ended", &email, Some(event.metadata.clone()))
                .await
        {
            tracing::warn!(error = %e, "Failed to track trial_ended event in Brevo");
        }

        tracing::debug!(
            organization_id = %event.organization_id,
            converted = %converted,
            "Updated Brevo: trial ended"
        );
        Ok(())
    }

    async fn handle_subscription_cancelled(&self, event: &TelemetryEvent) -> Result<()> {
        let company_attrs = CompanyAttributes::new().with_plan_status("cancelled");
        self.update_company_by_org(event.organization_id, company_attrs)
            .await?;

        if let Some(email) = self.get_owner_email(event.organization_id).await
            && let Err(e) = self
                .client
                .track_event("subscription_cancelled", &email, None)
                .await
        {
            tracing::warn!(error = %e, "Failed to track subscription_cancelled event in Brevo");
        }

        tracing::debug!(
            organization_id = %event.organization_id,
            "Updated Brevo: subscription cancelled"
        );
        Ok(())
    }

    async fn handle_trial_will_end(&self, event: &TelemetryEvent) -> Result<()> {
        let company_attrs = CompanyAttributes::new().with_plan_status("trial_ending_soon");
        self.update_company_by_org(event.organization_id, company_attrs)
            .await?;

        if let Some(email) = self.get_owner_email(event.organization_id).await
            && let Err(e) = self
                .client
                .track_event("trial_will_end", &email, None)
                .await
        {
            tracing::warn!(error = %e, "Failed to track trial_will_end event in Brevo");
        }

        tracing::debug!(
            organization_id = %event.organization_id,
            "Updated Brevo: trial ending soon"
        );
        Ok(())
    }

    async fn handle_plan_changed(&self, event: &TelemetryEvent) -> Result<()> {
        let new_plan = event
            .metadata
            .get("new_plan")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let plan_status = event.metadata.get("plan_status").and_then(|v| v.as_str());

        let mut company_attrs = CompanyAttributes::new().with_plan_type(new_plan);
        if let Some(status) = plan_status {
            company_attrs = company_attrs.with_plan_status(status);
        }
        self.update_company_by_org(event.organization_id, company_attrs)
            .await?;

        if let Some(email) = self.get_owner_email(event.organization_id).await
            && let Err(e) = self
                .client
                .track_event("plan_changed", &email, Some(event.metadata.clone()))
                .await
        {
            tracing::warn!(error = %e, "Failed to track plan_changed event in Brevo");
        }

        tracing::debug!(
            organization_id = %event.organization_id,
            new_plan = %new_plan,
            "Updated Brevo: plan changed"
        );
        Ok(())
    }

    async fn handle_first_daemon_registered(&self, event: &TelemetryEvent) -> Result<()> {
        let company_attrs = CompanyAttributes::new().with_first_daemon_date(event.timestamp);
        self.update_company_by_org(event.organization_id, company_attrs)
            .await?;

        if let Some(email) = self.get_owner_email(event.organization_id).await
            && let Err(e) = self
                .client
                .track_event("first_daemon_registered", &email, None)
                .await
        {
            tracing::warn!(error = %e, "Failed to track first_daemon_registered event in Brevo");
        }

        tracing::debug!(
            organization_id = %event.organization_id,
            "Updated Brevo: first daemon registered"
        );
        Ok(())
    }

    async fn handle_first_discovery_completed(&self, event: &TelemetryEvent) -> Result<()> {
        let company_attrs =
            CompanyAttributes::new().with_first_discovery_completed_date(event.timestamp);
        self.update_company_by_org(event.organization_id, company_attrs)
            .await?;

        if let Some(email) = self.get_owner_email(event.organization_id).await
            && let Err(e) = self
                .client
                .track_event("first_discovery_completed", &email, None)
                .await
        {
            tracing::warn!(error = %e, "Failed to track first_discovery_completed event in Brevo");
        }

        tracing::debug!(
            organization_id = %event.organization_id,
            "Updated Brevo: first discovery completed"
        );
        Ok(())
    }

    async fn handle_first_topology_rebuild(&self, event: &TelemetryEvent) -> Result<()> {
        let company_attrs =
            CompanyAttributes::new().with_first_topology_rebuild_date(event.timestamp);
        self.update_company_by_org(event.organization_id, company_attrs)
            .await?;

        tracing::debug!(
            organization_id = %event.organization_id,
            "Updated Brevo: first topology rebuild"
        );
        Ok(())
    }

    async fn update_contact_last_login(&self, email: String, user_id: Uuid) -> Result<()> {
        let contact_attrs = ContactAttributes::new()
            .with_email(&email)
            .with_user_id(user_id)
            .with_last_login_date(Utc::now());

        self.client.upsert_contact(&email, contact_attrs).await?;

        tracing::debug!(email = %email, "Updated Brevo contact: last login");
        Ok(())
    }

    async fn update_company_last_discovery(&self, org_id: Uuid) -> Result<()> {
        let company_attrs = CompanyAttributes::new().with_last_discovery_date(Utc::now());
        self.update_company_by_org(org_id, company_attrs).await?;

        tracing::debug!(organization_id = %org_id, "Updated Brevo company: last discovery");
        Ok(())
    }

    pub async fn sync_organization_metrics(
        &self,
        org_id: Uuid,
        network_count: i64,
        host_count: i64,
        user_count: i64,
    ) -> Result<()> {
        let company_attrs = CompanyAttributes::new()
            .with_network_count(network_count)
            .with_host_count(host_count)
            .with_user_count(user_count);

        self.update_company_by_org(org_id, company_attrs).await?;

        tracing::debug!(
            organization_id = %org_id,
            networks = %network_count,
            hosts = %host_count,
            users = %user_count,
            "Synced organization metrics to Brevo"
        );
        Ok(())
    }

    pub async fn sync_plan_limits(
        &self,
        org_id: Uuid,
        network_limit: Option<i64>,
        seat_limit: Option<i64>,
    ) -> Result<()> {
        let mut company_attrs = CompanyAttributes::new();

        if let Some(limit) = network_limit {
            company_attrs = company_attrs.with_network_limit(limit);
        }
        if let Some(limit) = seat_limit {
            company_attrs = company_attrs.with_seat_limit(limit);
        }

        self.update_company_by_org(org_id, company_attrs).await?;

        tracing::debug!(
            organization_id = %org_id,
            network_limit = ?network_limit,
            seat_limit = ?seat_limit,
            "Synced plan limits to Brevo"
        );
        Ok(())
    }

    pub async fn sync_org_entity_metrics(&self, org_id: Uuid) -> Result<()> {
        if self.get_brevo_company_id(org_id).await?.is_none() {
            tracing::debug!(
                organization_id = %org_id,
                "Skipping Brevo metrics sync - no company ID stored"
            );
            return Ok(());
        }

        let network_filter = StorableFilter::<Network>::new_from_org_id(&org_id);
        let networks = self.network_service.get_all(network_filter).await?;
        let network_ids: Vec<Uuid> = networks.iter().map(|n| n.id).collect();
        let network_count = networks.len() as i64;

        let host_filter = StorableFilter::<Host>::new_from_network_ids(&network_ids);
        let hosts = self.host_service.get_all(host_filter).await?;
        let host_count = hosts.len() as i64;

        let user_filter = StorableFilter::new_from_org_id(&org_id);
        let users = self.user_service.get_all(user_filter).await?;
        let user_count = users.len() as i64;

        self.sync_organization_metrics(org_id, network_count, host_count, user_count)
            .await?;
        Ok(())
    }

    pub async fn get_org_id_from_network(&self, network_id: &Uuid) -> Option<Uuid> {
        if let Ok(Some(network)) = self.network_service.get_by_id(network_id).await {
            Some(network.base.organization_id)
        } else {
            None
        }
    }

    /// Sync all organizations to Brevo on server startup.
    /// Syncs ALL orgs that don't have Brevo IDs yet (with backfilled telemetry).
    pub async fn sync_existing_organizations(&self) -> Result<()> {
        tracing::info!("Starting Brevo organization sync");

        let filter = StorableFilter::<Organization>::new_without_brevo_company_id();
        let orgs = self.organization_service.get_all(filter).await?;

        if orgs.is_empty() {
            tracing::info!("All organizations have Brevo company IDs");
            return Ok(());
        }

        let total = orgs.len();
        let mut synced_count = 0;

        for (i, org) in orgs.into_iter().enumerate() {
            let filter = StorableFilter::<User>::new_from_org_id(&org.id)
                .user_permissions(&UserOrgPermissions::Owner);
            let owners = self.user_service.get_all(filter).await?;

            let owner = match owners.first() {
                Some(owner) => owner,
                None => {
                    tracing::warn!(
                        organization_id = %org.id,
                        "No owner found for organization"
                    );
                    continue;
                }
            };

            tracing::info!(
                organization_id = %org.id,
                org_name = %org.base.name,
                "Backfilling org ({}/{})",
                i + 1,
                total
            );

            if let Err(e) = self.sync_organization_with_backfill(org, owner).await {
                tracing::error!(error = %e, "Failed to sync organization to Brevo");
            } else {
                synced_count += 1;
            }
        }

        tracing::info!(
            synced = synced_count,
            total = total,
            "Brevo organization sync complete"
        );
        Ok(())
    }

    async fn sync_organization_with_backfill(
        &self,
        mut org: Organization,
        owner: &User,
    ) -> Result<()> {
        let contact_attrs = ContactAttributes::new()
            .with_email(owner.base.email.to_string())
            .with_user_id(owner.id)
            .with_role("owner")
            .with_signup_date(owner.created_at)
            .with_last_login_date(owner.created_at);

        let mut company_attrs = CompanyAttributes::new()
            .with_name(&org.base.name)
            .with_org_id(org.id)
            .with_created_date(org.created_at);

        company_attrs = self
            .backfill_company_telemetry(org.id, company_attrs)
            .await?;

        let (_contact_id, company_id) = self
            .client
            .sync_contact_and_company(
                owner.base.email.as_ref(),
                contact_attrs,
                &org.base.name,
                company_attrs,
            )
            .await?;

        org.base.brevo_company_id = Some(company_id.clone());
        self.organization_service
            .update(&mut org, AuthenticatedEntity::System)
            .await?;

        tracing::info!(
            organization_id = %org.id,
            brevo_company_id = %company_id,
            "Synced organization to Brevo with backfilled telemetry"
        );
        Ok(())
    }

    async fn backfill_company_telemetry(
        &self,
        org_id: Uuid,
        mut attrs: CompanyAttributes,
    ) -> Result<CompanyAttributes> {
        let network_filter = StorableFilter::<Network>::new_from_org_id(&org_id);
        let networks = self.network_service.get_all(network_filter).await?;
        let network_ids: Vec<Uuid> = networks.iter().map(|n| n.id).collect();
        let network_count = networks.len() as i64;

        // Track second network date (first is created during onboarding, so not meaningful)
        let mut sorted_networks: Vec<_> = networks.iter().collect();
        sorted_networks.sort_by_key(|n| n.created_at);
        if let Some(second_network) = sorted_networks.get(1) {
            attrs = attrs.with_second_network_date(second_network.created_at);
        }

        let host_filter = StorableFilter::<Host>::new_from_network_ids(&network_ids);
        let hosts = self.host_service.get_all(host_filter).await?;
        let host_count = hosts.len() as i64;

        let user_filter = StorableFilter::<User>::new_from_org_id(&org_id);
        let users = self.user_service.get_all(user_filter).await?;
        let user_count = users.len() as i64;

        attrs = attrs
            .with_network_count(network_count)
            .with_host_count(host_count)
            .with_user_count(user_count);

        let daemon_filter = StorableFilter::<Daemon>::new_from_network_ids(&network_ids);
        let daemons = self.daemon_service.get_all(daemon_filter).await?;
        if let Some(first_daemon) = daemons.iter().min_by_key(|d| d.created_at) {
            attrs = attrs.with_first_daemon_date(first_daemon.created_at);
        }

        let tag_filter = StorableFilter::<Tag>::new_from_org_id(&org_id);
        let tags = self.tag_service.get_all(tag_filter).await?;
        if let Some(first_tag) = tags.iter().min_by_key(|t| t.created_at) {
            attrs = attrs.with_first_tag_date(first_tag.created_at);
        }

        let api_key_filter = StorableFilter::<UserApiKey>::new_from_org_id(&org_id);
        let api_keys = self.user_api_key_service.get_all(api_key_filter).await?;
        if let Some(first_api_key) = api_keys.iter().min_by_key(|k| k.created_at) {
            attrs = attrs.with_first_api_key_date(first_api_key.created_at);
        }

        let snmp_filter = StorableFilter::<SnmpCredential>::new_from_org_id(&org_id);
        let snmp_creds = self.snmp_credential_service.get_all(snmp_filter).await?;
        if let Some(first_snmp) = snmp_creds.iter().min_by_key(|s| s.created_at) {
            attrs = attrs.with_first_snmp_credential_date(first_snmp.created_at);
        }

        Ok(attrs)
    }
}
