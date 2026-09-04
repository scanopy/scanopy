use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    billing::types::base::PlanStatus,
    brevo::{
        client::BrevoClient,
        domain_classification::{
            DomainClass, classify_email_domain, classify_email_domain_probed, domain_has_website,
        },
        types::{CompanyAttributes, ContactAttributes},
    },
    credentials::{
        r#impl::{base::Credential, types::CredentialType},
        service::CredentialService,
    },
    daemons::{r#impl::base::Daemon, service::DaemonService},
    hosts::service::HostService,
    networks::{r#impl::Network, service::NetworkService},
    organizations::{r#impl::base::Organization, service::OrganizationService},
    shared::{
        events::{
            traits::Event,
            types::{AuthOperation, BillingOperation, OnboardingOperation},
        },
        services::traits::CrudService,
        storage::filter::StorableFilter,
        types::metadata::TypeMetadataProvider,
    },
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
/// Brevo DOI template ID — TODO: set to actual Brevo template ID before go-live
const BREVO_DOI_TEMPLATE_ID: i64 = 1;
/// Redirect URL after DOI confirmation
const BREVO_DOI_REDIRECTION_URL: &str = "https://scanopy.net/newsletter-confirmed";

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
    credential_service: Arc<CredentialService>,
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
        credential_service: Arc<CredentialService>,
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
            credential_service,
        }
    }

    pub(super) async fn handle_billing_event(&self, event: &Event<BillingOperation>) -> Result<()> {
        match &event.operation {
            BillingOperation::CheckoutStarted { .. } => {
                self.handle_checkout_started(event).await?;
            }
            BillingOperation::CheckoutCompleted { .. } => {
                self.handle_checkout_completed(event).await?;
            }
            BillingOperation::TrialStarted { .. } => {
                self.handle_trial_started(event).await?;
            }
            BillingOperation::TrialEnded { .. } => {
                self.handle_trial_ended(event).await?;
            }
            BillingOperation::SubscriptionCancelled { .. } => {
                self.handle_subscription_cancelled(event).await?;
            }
            BillingOperation::TrialWillEnd { .. } => {
                self.handle_trial_will_end(event).await?;
            }
            BillingOperation::PlanChanged { .. } => {
                self.handle_plan_changed(event).await?;
            }
            // Variants whose only Brevo effect is updating plan_status — drive
            // off `implied_status()` so the mapping stays canonical.
            BillingOperation::PaymentFailed { .. }
            | BillingOperation::PaymentActionRequired { .. }
            | BillingOperation::PaymentRecovered { .. } => {
                if let Some(status) = event.operation.implied_status() {
                    self.update_company_by_org(
                        event.scope.organization_id,
                        CompanyAttributes::new().with_plan_status(status),
                    )
                    .await?;
                }
            }
            BillingOperation::FeatureLimitHit { .. } => {}
            // Phase 5 additions — Brevo doesn't currently track these states explicitly.
            BillingOperation::Paused { .. }
            | BillingOperation::Resumed { .. }
            | BillingOperation::Reactivated { .. }
            | BillingOperation::DiscountApplied { .. }
            | BillingOperation::PaymentSucceeded { .. }
            | BillingOperation::TrialExtended { .. }
            | BillingOperation::CancellationInitiated { .. }
            | BillingOperation::CancellationFeedbackProvided { .. }
            | BillingOperation::PaymentMethodAdded
            | BillingOperation::PaymentMethodRemoved
            // Instance-level plan reconciliation of a self-hosted org. Brevo
            // isn't the system of record for self-hosted plan state; the org
            // subscriber owns the plan write. No CRM sync needed.
            | BillingOperation::PlanReconciled { .. }
            | BillingOperation::StripeCustomerCreated { .. } => {}
        }
        Ok(())
    }

    pub(super) async fn handle_onboarding_event(
        &self,
        event: &Event<OnboardingOperation>,
    ) -> Result<()> {
        match &event.operation {
            OnboardingOperation::OrgCreated { .. } => {
                self.handle_org_created(event).await?;
            }
            OnboardingOperation::FirstDaemonRegistered { .. } => {
                self.handle_first_daemon_registered(event).await?;
            }
            OnboardingOperation::FirstTopologyRebuild => {
                self.handle_first_topology_rebuild(event).await?;
            }
            OnboardingOperation::FirstDiscoveryCompleted { .. } => {
                self.handle_first_discovery_completed(event).await?;
            }
            OnboardingOperation::ProfileCompleted { .. } => {
                self.handle_profile_completed(event).await?;
            }
            OnboardingOperation::SecondNetworkCreated { .. }
            | OnboardingOperation::FirstHostDiscovered
            | OnboardingOperation::FirstTagCreated
            | OnboardingOperation::FirstDependencyCreated
            | OnboardingOperation::FirstApplicationTagCreated
            | OnboardingOperation::FirstUserApiKeyCreated
            | OnboardingOperation::FirstSnmpCredentialCreated
            | OnboardingOperation::FirstCredentialCreated
            | OnboardingOperation::FirstSnapshotCreated { .. }
            | OnboardingOperation::InviteSent
            | OnboardingOperation::InviteAccepted
            | OnboardingOperation::ReferralSourceCompleted { .. } => {
                self.handle_engagement_event(event).await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_engagement_event(&self, event: &Event<OnboardingOperation>) -> Result<()> {
        let mut company_attrs = CompanyAttributes::new();

        match &event.operation {
            OnboardingOperation::SecondNetworkCreated { .. } => {
                company_attrs = company_attrs.with_second_network_date(event.timestamp);
            }
            OnboardingOperation::FirstTagCreated => {
                company_attrs = company_attrs.with_first_tag_date(event.timestamp);
            }
            OnboardingOperation::FirstDependencyCreated => {
                company_attrs = company_attrs.with_first_dependency_date(event.timestamp);
            }
            OnboardingOperation::FirstApplicationTagCreated => {
                company_attrs =
                    company_attrs.with_first_application_group_tag_date(event.timestamp);
            }
            OnboardingOperation::FirstUserApiKeyCreated => {
                company_attrs = company_attrs.with_first_api_key_date(event.timestamp);
            }
            OnboardingOperation::FirstSnmpCredentialCreated => {
                company_attrs = company_attrs.with_first_snmp_credential_date(event.timestamp);
            }
            OnboardingOperation::FirstCredentialCreated => {
                company_attrs = company_attrs.with_first_credential_date(event.timestamp);
            }
            OnboardingOperation::InviteSent => {
                company_attrs = company_attrs.with_first_invite_sent_date(event.timestamp);
            }
            OnboardingOperation::InviteAccepted => {
                company_attrs = company_attrs.with_first_invite_accepted_date(event.timestamp);
            }
            OnboardingOperation::FirstHostDiscovered => {
                company_attrs = company_attrs.with_first_host_discovered_date(event.timestamp)
            }
            OnboardingOperation::FirstSnapshotCreated { .. } => {
                company_attrs = company_attrs.with_first_snapshot_date(event.timestamp);
            }
            _ => return Ok(()),
        }

        self.update_company_by_org(event.scope.organization_id, company_attrs)
            .await?;

        tracing::debug!(
            organization_id = %event.scope.organization_id,
            operation = %event.operation,
            "Updated Brevo company: engagement milestone"
        );
        Ok(())
    }

    async fn handle_profile_completed(&self, event: &Event<OnboardingOperation>) -> Result<()> {
        let email = match &event.authentication {
            AuthenticatedEntity::User { email, .. } => email.to_string(),
            _ => return Ok(()),
        };

        let OnboardingOperation::ProfileCompleted {
            job_title,
            company_size,
        } = &event.operation
        else {
            return Ok(());
        };

        if let Some(title) = job_title {
            let contact_attrs = ContactAttributes::new().with_job_title(title);
            let _ = self.client.upsert_contact(&email, contact_attrs).await;
        }
        if let Some(size) = company_size {
            let company_attrs = CompanyAttributes::new().with_company_size(size);
            let _ = self
                .update_company_by_org(event.scope.organization_id, company_attrs)
                .await;
        }

        tracing::debug!(
            organization_id = %event.scope.organization_id,
            "Updated Brevo: profile completed"
        );
        Ok(())
    }

    /// Handle Register events — sync the user's Brevo contact, list
    /// subscriptions, and DOI flow if they opted into marketing. Fires for
    /// every register flow (new org or invited user).
    pub(super) async fn handle_register(&self, event: &Event<AuthOperation>) -> Result<()> {
        let AuthOperation::Register {
            marketing_opt_in, ..
        } = &event.operation
        else {
            return Ok(());
        };
        let marketing_opt_in = *marketing_opt_in;

        let (email, user_id) = match &event.authentication {
            AuthenticatedEntity::User { email, user_id, .. } => (email.clone(), *user_id),
            _ => return Ok(()),
        };

        let (domain_class, institution_type) = classify_email_domain_probed(email.domain()).await;
        let contact_attrs = ContactAttributes::new()
            .with_email(email.to_string())
            .with_user_id(user_id)
            .with_role("owner")
            .with_signup_date(event.timestamp)
            .with_last_login_date(event.timestamp)
            .with_email_blacklisted(false)
            .with_marketing_opt_in(marketing_opt_in)
            .with_marketing_opt_in_date(event.timestamp)
            .with_domain_classification(domain_class, institution_type);

        let doi_attributes = contact_attrs.to_attributes();

        // Upsert the contact. The company is created on the OrgCreated channel;
        // once both exist we link them. Because the two events arrive on separate
        // subscriber channels with no ordering guarantee, we link idempotently
        // from both sides (Brevo's link-unlink endpoint is idempotent) — whichever
        // handler runs second finds the other party present and links.
        let contact_id = match self
            .client
            .upsert_contact(email.as_ref(), contact_attrs)
            .await
        {
            Ok(id) => Some(id),
            Err(e) => {
                tracing::warn!(error = %e, "Failed to upsert Brevo contact at register");
                None
            }
        };

        // If the org's Brevo company already exists, link the contact now.
        // Otherwise handle_org_created links it once the company is created.
        // Diagnostic logging on every branch so a missing link is traceable.
        if let Some(contact_id) = contact_id {
            match event.scope.organization_id {
                Some(org_id) => match self.get_brevo_company_id(org_id).await {
                    Ok(Some(company_id)) => match self
                        .client
                        .link_contact_to_company(&company_id, contact_id)
                        .await
                    {
                        Ok(()) => tracing::info!(
                            brevo_contact_id = %contact_id,
                            brevo_company_id = %company_id,
                            organization_id = %org_id,
                            "Linked Brevo contact to company at register"
                        ),
                        Err(e) => tracing::warn!(
                            error = %e,
                            brevo_contact_id = %contact_id,
                            brevo_company_id = %company_id,
                            organization_id = %org_id,
                            "Failed to link Brevo contact to company at register"
                        ),
                    },
                    Ok(None) => tracing::info!(
                        brevo_contact_id = %contact_id,
                        organization_id = %org_id,
                        "Brevo company not created yet at register; org-created handler will link"
                    ),
                    Err(e) => tracing::warn!(
                        error = %e,
                        organization_id = %org_id,
                        "Failed to look up Brevo company id at register"
                    ),
                },
                None => tracing::warn!(
                    brevo_contact_id = %contact_id,
                    "Register event missing organization_id; cannot link Brevo contact to company"
                ),
            }
        }

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

        // Trigger DOI confirmation for marketing list if opted in
        if marketing_opt_in
            && let Err(e) = self
                .client
                .create_doi_contact(
                    email.as_ref(),
                    vec![BREVO_MARKETING_LIST_ID],
                    BREVO_DOI_TEMPLATE_ID,
                    BREVO_DOI_REDIRECTION_URL,
                    doi_attributes,
                )
                .await
        {
            tracing::warn!(error = %e, "Failed to trigger DOI for Marketing list");
        }

        Ok(())
    }

    /// Handle org created — create the Brevo company and store its ID on the
    /// organization. Contact-side work (including marketing_opt_in / DOI) is
    /// handled by `handle_register` on the AuthOperation channel.
    async fn handle_org_created(&self, event: &Event<OnboardingOperation>) -> Result<()> {
        let OnboardingOperation::OrgCreated {
            org_name,
            plan: _,
            use_case,
        } = &event.operation
        else {
            return Ok(());
        };
        let org_name = org_name.clone();
        let use_case = *use_case;

        let owner_email = self.get_owner_email(event.scope.organization_id).await;

        let org_filter = StorableFilter::<Network>::new_from_org_id(&event.scope.organization_id);
        let network_count = self.network_service.get_all(org_filter).await?.len();

        let company_attrs = CompanyAttributes::new()
            .with_name(&org_name)
            .with_org_id(event.scope.organization_id)
            .with_created_date(event.timestamp)
            .with_network_count(network_count as i64)
            .with_host_count(0)
            .with_user_count(1)
            .with_org_type(use_case.to_string());

        let company_id = self
            .client
            .create_company(&org_name, company_attrs, None)
            .await?;

        // Store the company ID on the organization
        if let Some(mut org) = self
            .organization_service
            .get_by_id(&event.scope.organization_id)
            .await?
        {
            org.base.brevo_company_id = Some(company_id.clone());
            self.organization_service
                .update(&mut org, event.authentication.clone())
                .await?;
        }

        // Link the owner's contact to the new company. The contact is created on
        // the AuthOperation (Register) channel; if it isn't in Brevo yet,
        // handle_register links it once the company exists. Idempotent on Brevo's
        // side, so linking from both handlers is safe. Diagnostic logging on every
        // branch so a missing link is traceable.
        match &owner_email {
            Some(email) => match self.client.get_contact_id_by_email(email).await {
                Ok(contact_id) => match self
                    .client
                    .link_contact_to_company(&company_id, contact_id)
                    .await
                {
                    Ok(()) => tracing::info!(
                        brevo_contact_id = %contact_id,
                        brevo_company_id = %company_id,
                        organization_id = %event.scope.organization_id,
                        "Linked owner contact to new Brevo company"
                    ),
                    Err(e) => tracing::warn!(
                        error = %e,
                        brevo_contact_id = %contact_id,
                        brevo_company_id = %company_id,
                        organization_id = %event.scope.organization_id,
                        "Failed to link owner contact to new Brevo company"
                    ),
                },
                Err(e) => tracing::info!(
                    error = %e,
                    brevo_company_id = %company_id,
                    organization_id = %event.scope.organization_id,
                    "Owner contact not yet in Brevo at org-created; register handler will link"
                ),
            },
            None => tracing::warn!(
                brevo_company_id = %company_id,
                organization_id = %event.scope.organization_id,
                "No owner email found for org; cannot link contact to new Brevo company"
            ),
        }

        // Track event for automation (uses owner email; OK to skip if missing)
        if let Some(email) = owner_email
            && let Err(e) = self.client.track_event("org_created", &email, None).await
        {
            tracing::warn!(error = %e, "Failed to track org_created event in Brevo");
        }

        tracing::info!(
            organization_id = %event.scope.organization_id,
            brevo_company_id = %company_id,
            "Synced new organization company to Brevo"
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

    async fn handle_checkout_started(&self, event: &Event<BillingOperation>) -> Result<()> {
        let BillingOperation::CheckoutStarted { plan, .. } = &event.operation else {
            return Ok(());
        };
        let plan_name = plan.name();

        let company_attrs = CompanyAttributes::new()
            .with_plan_type(plan_name)
            .with_lifecycle_marker("checkout_started");
        self.update_company_by_org(event.scope.organization_id, company_attrs)
            .await?;

        tracing::debug!(
            organization_id = %event.scope.organization_id,
            plan = %plan_name,
            "Updated Brevo company: checkout started"
        );
        Ok(())
    }

    async fn handle_checkout_completed(&self, event: &Event<BillingOperation>) -> Result<()> {
        let BillingOperation::CheckoutCompleted {
            plan,
            included_networks,
            included_seats,
            mrr_amount_cents: _,
            is_trialing: _,
            next_renewal_at: _,
        } = &event.operation
        else {
            return Ok(());
        };
        let plan_name = plan.name();
        // CheckoutCompleted is emitted whether or not the subscription is in
        // trial — Brevo treats checkout_completed without a separate trial
        // event as "active". TrialStarted handler updates to "trialing".
        let company_attrs = CompanyAttributes::new()
            .with_plan_type(plan_name)
            .with_plan_status(PlanStatus::Active)
            .with_checkout_completed_date(event.timestamp);

        self.update_company_by_org(event.scope.organization_id, company_attrs)
            .await?;

        let network_limit = included_networks.map(|n| n as i64);
        let seat_limit = included_seats.map(|n| n as i64);

        if network_limit.is_some() || seat_limit.is_some() {
            self.sync_plan_limits(event.scope.organization_id, network_limit, seat_limit)
                .await?;
        }

        // Track event for automation
        if let Some(email) = self.get_owner_email(event.scope.organization_id).await
            && let Err(e) = self
                .client
                .track_event(
                    "checkout_completed",
                    &email,
                    serde_json::to_value(&event.operation).ok(),
                )
                .await
        {
            tracing::warn!(error = %e, "Failed to track checkout_completed event in Brevo");
        }

        tracing::info!(
            organization_id = %event.scope.organization_id,
            plan = %plan_name,
            "Updated Brevo: checkout completed"
        );
        Ok(())
    }

    async fn handle_trial_started(&self, event: &Event<BillingOperation>) -> Result<()> {
        let company_attrs = CompanyAttributes::new()
            .with_plan_status(PlanStatus::Trialing)
            .with_trial_started_date(event.timestamp);

        self.update_company_by_org(event.scope.organization_id, company_attrs)
            .await?;

        if let Some(email) = self.get_owner_email(event.scope.organization_id).await
            && let Err(e) = self.client.track_event("trial_started", &email, None).await
        {
            tracing::warn!(error = %e, "Failed to track trial_started event in Brevo");
        }

        tracing::debug!(
            organization_id = %event.scope.organization_id,
            "Updated Brevo: trial started"
        );
        Ok(())
    }

    async fn handle_trial_ended(&self, event: &Event<BillingOperation>) -> Result<()> {
        let BillingOperation::TrialEnded { converted, .. } = &event.operation else {
            return Ok(());
        };
        let converted = *converted;

        use PlanStatus;
        let company_attrs = CompanyAttributes::new().with_plan_status(if converted {
            PlanStatus::Active
        } else {
            PlanStatus::Cancelled
        });

        self.update_company_by_org(event.scope.organization_id, company_attrs)
            .await?;

        if let Some(email) = self.get_owner_email(event.scope.organization_id).await
            && let Err(e) = self
                .client
                .track_event(
                    "trial_ended",
                    &email,
                    serde_json::to_value(&event.operation).ok(),
                )
                .await
        {
            tracing::warn!(error = %e, "Failed to track trial_ended event in Brevo");
        }

        tracing::debug!(
            organization_id = %event.scope.organization_id,
            converted = %converted,
            "Updated Brevo: trial ended"
        );
        Ok(())
    }

    async fn handle_subscription_cancelled(&self, event: &Event<BillingOperation>) -> Result<()> {
        // Cancellation always downgrades to Free. Used to ride a chained
        // PlanChanged{to: Free} for the plan_type write; now folded in here
        // so the cancel-side-effects path emits exactly one event.
        let was_trialing = matches!(
            &event.operation,
            BillingOperation::SubscriptionCancelled {
                was_trialing: true,
                ..
            }
        );
        let company_attrs = CompanyAttributes::new()
            .with_plan_status(PlanStatus::Cancelled)
            .with_plan_type("Free");
        self.update_company_by_org(event.scope.organization_id, company_attrs)
            .await?;

        if let Some(email) = self.get_owner_email(event.scope.organization_id).await {
            if let Err(e) = self
                .client
                .track_event("subscription_cancelled", &email, None)
                .await
            {
                tracing::warn!(error = %e, "Failed to track subscription_cancelled event in Brevo");
            }
            // Mirror the trial-end analytics signal that used to come from
            // the chained TrialEnded{converted:false} emission.
            if was_trialing
                && let Err(e) = self
                    .client
                    .track_event(
                        "trial_ended",
                        &email,
                        serde_json::to_value(&event.operation).ok(),
                    )
                    .await
            {
                tracing::warn!(error = %e, "Failed to track trial_ended event in Brevo");
            }
        }

        tracing::debug!(
            organization_id = %event.scope.organization_id,
            "Updated Brevo: subscription cancelled"
        );
        Ok(())
    }

    async fn handle_trial_will_end(&self, event: &Event<BillingOperation>) -> Result<()> {
        let company_attrs = CompanyAttributes::new().with_lifecycle_marker("trial_ending_soon");
        self.update_company_by_org(event.scope.organization_id, company_attrs)
            .await?;

        if let Some(email) = self.get_owner_email(event.scope.organization_id).await
            && let Err(e) = self
                .client
                .track_event("trial_will_end", &email, None)
                .await
        {
            tracing::warn!(error = %e, "Failed to track trial_will_end event in Brevo");
        }

        tracing::debug!(
            organization_id = %event.scope.organization_id,
            "Updated Brevo: trial ending soon"
        );
        Ok(())
    }

    async fn handle_plan_changed(&self, event: &Event<BillingOperation>) -> Result<()> {
        let BillingOperation::PlanChanged {
            to, is_downgrade, ..
        } = &event.operation
        else {
            return Ok(());
        };
        let new_plan = to.name();
        // PlanChanged maps to PlanStatus::Active via implied_status — the
        // typed mapping handles is_downgrade implicitly.
        let _ = is_downgrade;
        let company_attrs = CompanyAttributes::new()
            .with_plan_type(new_plan)
            .with_plan_status(PlanStatus::Active);
        self.update_company_by_org(event.scope.organization_id, company_attrs)
            .await?;

        if let Some(email) = self.get_owner_email(event.scope.organization_id).await
            && let Err(e) = self
                .client
                .track_event(
                    "plan_changed",
                    &email,
                    serde_json::to_value(&event.operation).ok(),
                )
                .await
        {
            tracing::warn!(error = %e, "Failed to track plan_changed event in Brevo");
        }

        tracing::debug!(
            organization_id = %event.scope.organization_id,
            new_plan = %new_plan,
            "Updated Brevo: plan changed"
        );
        Ok(())
    }

    async fn handle_first_daemon_registered(
        &self,
        event: &Event<OnboardingOperation>,
    ) -> Result<()> {
        let company_attrs = CompanyAttributes::new().with_first_daemon_date(event.timestamp);
        self.update_company_by_org(event.scope.organization_id, company_attrs)
            .await?;

        if let Some(email) = self.get_owner_email(event.scope.organization_id).await
            && let Err(e) = self
                .client
                .track_event("first_daemon_registered", &email, None)
                .await
        {
            tracing::warn!(error = %e, "Failed to track first_daemon_registered event in Brevo");
        }

        tracing::debug!(
            organization_id = %event.scope.organization_id,
            "Updated Brevo: first daemon registered"
        );
        Ok(())
    }

    async fn handle_first_discovery_completed(
        &self,
        event: &Event<OnboardingOperation>,
    ) -> Result<()> {
        let company_attrs =
            CompanyAttributes::new().with_first_discovery_completed_date(event.timestamp);
        self.update_company_by_org(event.scope.organization_id, company_attrs)
            .await?;

        if let Some(email) = self.get_owner_email(event.scope.organization_id).await
            && let Err(e) = self
                .client
                .track_event("first_discovery_completed", &email, None)
                .await
        {
            tracing::warn!(error = %e, "Failed to track first_discovery_completed event in Brevo");
        }

        tracing::debug!(
            organization_id = %event.scope.organization_id,
            "Updated Brevo: first discovery completed"
        );
        Ok(())
    }

    async fn handle_first_topology_rebuild(
        &self,
        event: &Event<OnboardingOperation>,
    ) -> Result<()> {
        let company_attrs =
            CompanyAttributes::new().with_first_topology_rebuild_date(event.timestamp);
        self.update_company_by_org(event.scope.organization_id, company_attrs)
            .await?;

        tracing::debug!(
            organization_id = %event.scope.organization_id,
            "Updated Brevo: first topology rebuild"
        );
        Ok(())
    }

    pub(super) async fn update_contact_last_login(
        &self,
        email: String,
        user_id: Uuid,
    ) -> Result<()> {
        let contact_attrs = ContactAttributes::new()
            .with_email(&email)
            .with_user_id(user_id)
            .with_last_login_date(Utc::now());

        self.client.upsert_contact(&email, contact_attrs).await?;

        tracing::debug!(email = %email, "Updated Brevo contact: last login");
        Ok(())
    }

    pub(super) async fn update_company_last_discovery(&self, org_id: Uuid) -> Result<()> {
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

        // count_for_networks/count_for_org narrow SCD2 entities to live rows so
        // snapshot closed-copies don't inflate the synced counts.
        let host_count = self.host_service.count_for_networks(&network_ids).await? as i64;
        let user_count = self.user_service.count_for_org(&org_id).await? as i64;

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

    /// One-shot backfill of `SCANOPY_DOMAIN_CLASS` / `SCANOPY_INSTITUTION_TYPE`
    /// for every existing user, spawned once at server startup. Ephemeral
    /// release code — remove (together with
    /// `StorableFilter::new_for_brevo_backfill`) in the release after
    /// email-domain classification ships.
    ///
    /// Idempotent: recomputes the same pure classification and upserts, so
    /// re-running (every startup during this release) is safe. Only reachable
    /// when the Brevo API key is configured — an unconfigured environment
    /// never constructs `BrevoService`, so nothing syncs.
    pub async fn backfill_domain_classifications(&self) -> Result<()> {
        let users = self
            .user_service
            .get_all(StorableFilter::<User>::new_for_brevo_backfill())
            .await?;
        let total = users.len();
        tracing::info!(total, "Starting Brevo domain-classification backfill");

        let mut synced = 0usize;
        // Memoize website probes: many users can share one company domain.
        let mut has_website_cache: std::collections::HashMap<String, bool> =
            std::collections::HashMap::new();
        for user in users {
            let email = &user.base.email;
            let (mut domain_class, institution_type) = classify_email_domain(email.domain());
            if domain_class == DomainClass::Company {
                let has_website = match has_website_cache.get(email.domain()) {
                    Some(v) => *v,
                    None => {
                        let v = domain_has_website(email.domain()).await;
                        has_website_cache.insert(email.domain().to_string(), v);
                        v
                    }
                };
                if !has_website {
                    domain_class = DomainClass::Personal;
                }
            }
            let attrs = ContactAttributes::new()
                .with_email(email.to_string())
                .with_user_id(user.id)
                .with_domain_classification(domain_class, institution_type);
            match self.client.upsert_contact(email.as_ref(), attrs).await {
                Ok(_) => synced += 1,
                Err(e) => tracing::warn!(
                    error = %e,
                    domain = email.domain(),
                    "Failed to backfill domain classification"
                ),
            }
        }

        tracing::info!(
            synced,
            total,
            "Brevo domain-classification backfill complete"
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

        // count_for_networks/count_for_org narrow SCD2 entities to live rows so
        // snapshot closed-copies don't inflate the synced counts.
        let host_count = self.host_service.count_for_networks(&network_ids).await? as i64;
        let user_count = self.user_service.count_for_org(&org_id).await? as i64;

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

        let cred_filter = StorableFilter::<Credential>::new_from_org_id(&org_id);
        let creds = self.credential_service.get_all(cred_filter).await?;
        if let Some(first_cred) = creds.iter().min_by_key(|c| c.created_at) {
            attrs = attrs.with_first_credential_date(first_cred.created_at);
        }
        if let Some(first_snmp) = creds
            .iter()
            .filter(|c| matches!(c.base.credential_type, CredentialType::SnmpV2c { .. }))
            .min_by_key(|c| c.created_at)
        {
            attrs = attrs.with_first_snmp_credential_date(first_snmp.created_at);
        }

        Ok(attrs)
    }
}
