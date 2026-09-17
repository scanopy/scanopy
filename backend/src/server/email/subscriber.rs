//! Email subscriber for entity (Host/Network/User Created/Deleted) and
//! onboarding (FirstDaemonRegistered, FirstDiscoveryCompleted) events.
//!
//! Triggers transactional emails: plan-limit notifications on entity
//! create/delete, and post-onboarding guides (discovery walkthrough,
//! topology-ready notification).

use anyhow::Error;
use async_trait::async_trait;
use std::collections::HashMap;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    billing::types::base::BillingReason,
    digest::payload::{DiscoveryDigestOperation, DiscoveryDigestOperationDiscriminants},
    email::service::{EmailService, format_cents},
    license::mint::{GRACE_PERIOD_DAYS, PAID_THROUGH_BUFFER_DAYS},
    license::types::LicenseKeyType,
    shared::{
        entities::{Entity, EntityDiscriminants},
        events::{
            registry::SubscriberRegistration,
            traits::{EntityEventFilter, Event, EventFilter, Subscriber},
            types::{
                AuthOperation, AuthOperationDiscriminants, BillingOperation,
                BillingOperationDiscriminants, EntityOperation, EntityOperationDiscriminants,
                OnboardingOperation, OnboardingOperationDiscriminants,
            },
        },
        services::traits::CrudService,
        types::metadata::TypeMetadataProvider,
    },
};

#[async_trait]
impl Subscriber<BillingOperation> for EmailService {
    fn filter(&self) -> EventFilter<BillingOperation> {
        EventFilter::ops(vec![
            BillingOperationDiscriminants::TrialStarted,
            BillingOperationDiscriminants::TrialEnded,
            BillingOperationDiscriminants::TrialWillEnd,
            BillingOperationDiscriminants::PlanChanged,
            BillingOperationDiscriminants::SubscriptionCancelled,
            BillingOperationDiscriminants::PaymentFailed,
            BillingOperationDiscriminants::PaymentActionRequired,
            BillingOperationDiscriminants::PaymentRecovered,
            BillingOperationDiscriminants::PaymentSucceeded,
            BillingOperationDiscriminants::PaymentMethodAdded,
            BillingOperationDiscriminants::PaymentMethodRemoved,
            BillingOperationDiscriminants::CancellationInitiated,
            BillingOperationDiscriminants::CheckoutCompleted,
            BillingOperationDiscriminants::Reactivated,
            BillingOperationDiscriminants::Paused,
            BillingOperationDiscriminants::Resumed,
        ])
    }

    async fn handle(&self, events: Vec<Event<BillingOperation>>) -> Result<(), Error> {
        for event in events {
            // The filter narrows to discriminants that produce email; safe to
            // load the owner email up front.
            let org_owner = match self.get_owner_email(&event.scope.organization_id).await {
                Ok(email) => email,
                Err(e) => {
                    tracing::warn!(
                        organization_id = %event.scope.organization_id,
                        error = %e,
                        "Failed to resolve org owner email; skipping email for this event",
                    );
                    continue;
                }
            };

            match event.operation {
                BillingOperation::TrialStarted {
                    plan, trial_days, ..
                } => {
                    // A self-hosted trial fires CheckoutCompleted alongside
                    // this, and that arm sends the self-hosted welcome. The
                    // cloud trial email would be a second, wrong-footed
                    // message about scanning your network.
                    if plan.license_plan().is_none() {
                        self.send_trial_started_email(
                            org_owner,
                            plan.name(),
                            trial_days,
                            plan.billing_period(),
                        )
                        .await?;
                    }
                }
                BillingOperation::TrialEnded {
                    plan,
                    converted,
                    next_renewal_at: _,
                } => {
                    if converted {
                        self.send_trial_converted_email(
                            org_owner,
                            plan.name(),
                            plan.billing_period(),
                        )
                        .await?;
                    } else {
                        self.send_trial_expired_email(
                            org_owner,
                            plan.name(),
                            plan.billing_period(),
                        )
                        .await?;
                    }
                }
                BillingOperation::PlanChanged { from, to, .. } => {
                    // Moving from a cloud plan onto a self-hosted one is the
                    // third way an org ends up needing a license key, and the
                    // only one that raises no checkout.
                    if to.license_plan().is_some() && from.license_plan().is_none() {
                        self.send_self_hosted_welcome_email(
                            org_owner,
                            to.name(),
                            None,
                            to.features().deployment_assistance,
                        )
                        .await?;
                    } else {
                        self.send_plan_changed_email(org_owner, to.name()).await?;
                    }
                }
                BillingOperation::TrialWillEnd {
                    plan,
                    has_payment_method,
                } => {
                    self.send_trial_ending_email(
                        org_owner,
                        event.scope.organization_id,
                        plan.name(),
                        has_payment_method,
                        plan.billing_period(),
                    )
                    .await?;
                }
                BillingOperation::SubscriptionCancelled { period_end, .. } => {
                    let period_end_str = period_end.format("%B %-d, %Y").to_string();
                    self.send_subscription_cancelled_email(org_owner, &period_end_str)
                        .await?;
                }
                BillingOperation::PaymentFailed {
                    plan,
                    attempt_count,
                    ..
                } => {
                    // Stripe retries a failed invoice several times over about
                    // three weeks and raises this each time; one email is
                    // enough, and the past-due banner carries the rest.
                    if plan.license_plan().is_some() {
                        if attempt_count <= 1 {
                            let organization = self
                                .organization_service
                                .get_by_id(&event.scope.organization_id)
                                .await?;
                            let key_expires = organization
                                .as_ref()
                                .and_then(|org| org.base.license_paid_through)
                                .map(|at| at.format("%B %-d, %Y").to_string())
                                .unwrap_or_else(|| "your renewal date".to_string());
                            let air_gapped = organization.is_some_and(|org| {
                                org.base.license_key_type == Some(LicenseKeyType::Offline)
                            });
                            self.send_self_hosted_payment_failed_email(
                                org_owner,
                                &key_expires,
                                air_gapped,
                            )
                            .await?;
                        }
                    } else {
                        self.send_payment_failed_email(org_owner).await?;
                    }
                }
                BillingOperation::PaymentActionRequired {
                    hosted_invoice_url, ..
                } => {
                    self.send_payment_action_required_email(org_owner, hosted_invoice_url)
                        .await?;
                }
                BillingOperation::PaymentSucceeded { invoice } => {
                    // Send usage summary for recurring billing cycles only
                    // (skip the initial subscription invoice and one-off
                    // charges). Self-hosted license renewals have no cloud
                    // usage to summarize.
                    if invoice.billing_reason == BillingReason::SubscriptionCycle {
                        match invoice.license_paid_through() {
                            None => self.send_usage_summary_email(org_owner, &invoice).await?,
                            // A renewed self-hosted licence. An online key
                            // picks this up at its next check-in; an
                            // air-gapped key carries its expiry inside it, so
                            // its owner has to copy the new key by hand.
                            Some(renewed_through) => {
                                let Some(organization) = self
                                    .organization_service
                                    .get_by_id(&event.scope.organization_id)
                                    .await?
                                else {
                                    continue;
                                };
                                if organization.base.license_key_type
                                    == Some(LicenseKeyType::Offline)
                                {
                                    // The key on their server predates this
                                    // renewal, so it still runs to the old
                                    // paid-through plus the buffer and grace.
                                    let current_key_expires = organization
                                        .base
                                        .license_paid_through
                                        .map(|at| {
                                            (at + chrono::Duration::days(
                                                PAID_THROUGH_BUFFER_DAYS + GRACE_PERIOD_DAYS,
                                            ))
                                            .format("%B %-d, %Y")
                                            .to_string()
                                        })
                                        .unwrap_or_else(|| "its current expiry".to_string());
                                    let plan_name = organization
                                        .base
                                        .plan
                                        .map(|plan| plan.name())
                                        .unwrap_or("Self-Hosted");
                                    self.send_airgap_renewal_email(
                                        org_owner,
                                        plan_name,
                                        &current_key_expires,
                                        &renewed_through.format("%B %-d, %Y").to_string(),
                                    )
                                    .await?;
                                }
                            }
                        }
                    }
                }
                BillingOperation::PaymentMethodAdded => {
                    self.send_payment_method_added_email(org_owner).await?;
                }
                BillingOperation::PaymentMethodRemoved => {
                    self.send_payment_method_removed_email(org_owner).await?;
                }
                BillingOperation::PaymentRecovered { amount_cents, .. } => {
                    let amount = format_cents(amount_cents, "usd");
                    self.send_payment_recovered_email(org_owner, &amount)
                        .await?;
                }
                BillingOperation::CancellationInitiated {
                    planned_period_end, ..
                } => {
                    let period_end_str = planned_period_end.format("%B %-d, %Y").to_string();
                    self.send_cancellation_initiated_email(org_owner, &period_end_str)
                        .await?;
                }
                BillingOperation::CheckoutCompleted {
                    plan, is_trialing, ..
                } => {
                    // Covers both self-hosted signup paths: a trial start and
                    // an outright purchase.
                    if plan.license_plan().is_some() {
                        self.send_self_hosted_welcome_email(
                            org_owner,
                            plan.name(),
                            is_trialing.then(|| plan.config().trial_days),
                            plan.features().deployment_assistance,
                        )
                        .await?;
                    } else {
                        self.send_checkout_completed_email(org_owner, plan.name())
                            .await?;
                    }
                }
                BillingOperation::Reactivated { .. } => {
                    self.send_subscription_reactivated_email(org_owner).await?;
                }
                BillingOperation::Paused { resumes_at, .. } => {
                    let resumes_at_str = resumes_at.format("%B %-d, %Y").to_string();
                    self.send_subscription_paused_email(org_owner, &resumes_at_str)
                        .await?;
                }
                BillingOperation::Resumed { .. } => {
                    self.send_subscription_resumed_email(org_owner).await?;
                }
                _ => {}
            }
        }

        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<EmailService, BillingOperation>());

#[async_trait]
impl Subscriber<AuthOperation> for EmailService {
    fn filter(&self) -> EventFilter<AuthOperation> {
        EventFilter::ops(vec![
            AuthOperationDiscriminants::Register,
            AuthOperationDiscriminants::EmailChangeRequested,
            AuthOperationDiscriminants::EmailVerificationRequested,
            AuthOperationDiscriminants::EmailChanged,
            AuthOperationDiscriminants::PasswordChanged,
            AuthOperationDiscriminants::PasswordResetRequested,
            AuthOperationDiscriminants::OidcLinked,
            AuthOperationDiscriminants::OidcUnlinked,
        ])
    }

    async fn handle(&self, events: Vec<Event<AuthOperation>>) -> Result<(), Error> {
        for event in events {
            match event.operation {
                AuthOperation::Register {
                    email_and_token: Some(params),
                    ..
                } => {
                    self.send_verification_email(
                        params.email,
                        self.public_url.clone(),
                        params.token,
                    )
                    .await?
                }
                AuthOperation::EmailChangeRequested { email_and_token } => {
                    self.send_verification_email(
                        email_and_token.email,
                        self.public_url.clone(),
                        email_and_token.token,
                    )
                    .await?
                }
                AuthOperation::EmailVerificationRequested { email_and_token } => {
                    self.send_verification_email(
                        email_and_token.email,
                        self.public_url.clone(),
                        email_and_token.token,
                    )
                    .await?
                }
                AuthOperation::EmailChanged {
                    old_email,
                    new_email,
                } => {
                    self.send_email_changed_old_email(old_email, new_email)
                        .await?;
                }
                AuthOperation::PasswordChanged {
                    email, timestamp, ..
                } => {
                    self.send_password_changed_email(
                        email,
                        &timestamp.format("%Y-%m-%d %H:%M UTC").to_string(),
                    )
                    .await?;
                }
                AuthOperation::PasswordResetRequested { email_and_token } => {
                    self.send_password_reset(
                        email_and_token.email,
                        self.public_url.clone(),
                        email_and_token.token,
                    )
                    .await?;
                }
                AuthOperation::OidcLinked { provider, email } => {
                    self.send_oidc_linked_email(email, &provider.name).await?;
                }
                AuthOperation::OidcUnlinked { provider, email } => {
                    self.send_oidc_unlinked_email(email, &provider.name).await?;
                }
                _ => {}
            }
        }

        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<EmailService, AuthOperation>());

#[async_trait]
impl Subscriber<EntityOperation> for EmailService {
    fn filter(&self) -> EntityEventFilter {
        let create_or_delete = Some(vec![
            EntityOperationDiscriminants::Created,
            EntityOperationDiscriminants::Deleted,
        ]);
        EntityEventFilter::by_entity(HashMap::from([
            (EntityDiscriminants::Host, create_or_delete.clone()),
            (EntityDiscriminants::Network, create_or_delete.clone()),
            (EntityDiscriminants::User, create_or_delete.clone()),
            // Organization deletion sends a confirmation email to the
            // initiating user.
            (
                EntityDiscriminants::Organization,
                Some(vec![EntityOperationDiscriminants::Deleted]),
            ),
        ]))
    }

    async fn handle(&self, events: Vec<Event<EntityOperation>>) -> Result<(), Error> {
        for event in events {
            let org_id = if let Some(org_id) = event.scope.organization_id() {
                Some(org_id)
            } else if let Some(network_id) = event.scope.network_id() {
                self.network_service
                    .get_by_id(&network_id)
                    .await?
                    .map(|n| n.base.organization_id)
            } else {
                None
            };

            if let Some(org_id) = org_id
                && let Err(e) = self
                    .check_plan_limits(org_id, event.operation == EntityOperation::Deleted)
                    .await
            {
                tracing::warn!(
                    organization_id = %org_id,
                    error = %e,
                    "Failed to check plan limits"
                );
            }

            // Org-deleted confirmation email to the initiator. Skipped for
            // non-User auth (System / Anonymous have no recipient).
            if matches!(event.scope.entity_type(), Entity::Organization(_))
                && event.operation == EntityOperation::Deleted
                && let AuthenticatedEntity::User { email, .. } = &event.authentication
                && let Err(e) = self.send_organization_deleted_email(email.clone()).await
            {
                tracing::warn!(
                    error = %e,
                    "Failed to send organization-deleted email",
                );
            }
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<EmailService, EntityOperation>());

#[async_trait]
impl Subscriber<OnboardingOperation> for EmailService {
    fn filter(&self) -> EventFilter<OnboardingOperation> {
        EventFilter::ops(vec![
            OnboardingOperationDiscriminants::FirstDaemonRegistered,
        ])
    }

    async fn handle(&self, events: Vec<Event<OnboardingOperation>>) -> Result<(), Error> {
        for event in events {
            let org_id = event.scope.organization_id;
            if let OnboardingOperation::FirstDaemonRegistered {
                daemon_name,
                network_name,
            } = &event.operation
                && let Err(e) = self
                    .send_discovery_guide_for_org(org_id, daemon_name, network_name)
                    .await
            {
                tracing::warn!(
                    organization_id = %org_id,
                    error = %e,
                    "Failed to send discovery guide email"
                );
            }
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    EmailService,
    OnboardingOperation,
>());

#[async_trait]
impl Subscriber<DiscoveryDigestOperation> for EmailService {
    fn filter(&self) -> EventFilter<DiscoveryDigestOperation> {
        EventFilter::ops(vec![DiscoveryDigestOperationDiscriminants::Computed])
    }

    async fn handle(&self, events: Vec<Event<DiscoveryDigestOperation>>) -> Result<(), Error> {
        for event in events {
            let DiscoveryDigestOperation::Computed { payload } = event.operation;
            if !payload.has_changes() {
                continue;
            }
            for recipient in &payload.recipients {
                if !recipient.discovery_digest_enabled {
                    continue;
                }
                if let Err(e) = self
                    .send_discovery_digest_email(recipient.email.clone(), &payload)
                    .await
                {
                    tracing::warn!(
                        user_id = %recipient.user_id,
                        session_id = %payload.session_id,
                        error = %e,
                        "Failed to send discovery digest email",
                    );
                }
            }
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    EmailService,
    DiscoveryDigestOperation,
>());
