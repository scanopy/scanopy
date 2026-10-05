//! Cloud setup: the site, its system subnets and the live topology row a
//! cloud org works from.
//!
//! This is the one place that sequence is created. It runs from `OrgCreated`
//! (the site requested at signup) and from billing events that land an org
//! on a cloud plan, so a self-hosted license buyer who moves to cloud ends up
//! with what a cloud signup has. The site-create and org-reset handlers call
//! the same functions. Every step is idempotent: it fills only what's missing,
//! so a bus retry or a repeat event never duplicates anything.

use anyhow::Error;
use async_trait::async_trait;
use uuid::Uuid;

use crate::server::{
    auth::{r#impl::base::PendingSiteSetup, middleware::auth::AuthenticatedEntity},
    billing::types::base::BillingPlan,
    shared::{
        events::{
            registry::SubscriberRegistration,
            traits::{Event, EventFilter, Subscriber},
            types::{
                BillingOperation, BillingOperationDiscriminants, OnboardingOperation,
                OnboardingOperationDiscriminants,
            },
        },
        services::traits::CrudService,
        storage::{
            filter::StorableFilter,
            seed_data::{create_remote_subnet, create_wan_subnet},
            traits::Storable,
        },
    },
    sites::r#impl::{Site, SiteBase},
    subnets::{r#impl::base::Subnet, r#impl::types::SubnetType},
    topology::{
        service::main::TopologyService,
        types::base::{Topology, TopologyBase},
    },
};

/// Subnet types every site is seeded with.
const SYSTEM_SUBNET_TYPES: [SubnetType; 2] = [SubnetType::Internet, SubnetType::Remote];

/// What one site lacks compared with a freshly created one.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct SiteSetupGaps {
    pub topology: bool,
    pub subnet_types: Vec<SubnetType>,
}

impl SiteSetupGaps {
    pub(crate) fn find(has_topology: bool, existing_subnet_types: &[SubnetType]) -> Self {
        Self {
            topology: !has_topology,
            subnet_types: SYSTEM_SUBNET_TYPES
                .into_iter()
                .filter(|t| !existing_subnet_types.contains(t))
                .collect(),
        }
    }
}

/// Whether a billing event puts the org on a cloud plan it may not have been
/// set up for: a first plan pick or resubscribe, or a move off a license plan.
/// A move between cloud plans and renewals are excluded, so a cloud org that
/// removed its sites doesn't get one back on every billing change.
pub(crate) fn lands_on_cloud_plan(operation: &BillingOperation) -> bool {
    let is_cloud = |plan: &BillingPlan| plan.license_plan().is_none();
    match operation {
        BillingOperation::TrialStarted { plan, .. }
        | BillingOperation::CheckoutCompleted { plan, .. } => is_cloud(plan),
        BillingOperation::PlanChanged { from, to, .. } => !is_cloud(from) && is_cloud(to),
        _ => false,
    }
}

impl TopologyService {
    /// Give the org a site if it has none, then fill in each site's system
    /// subnets and live topology. `requested` is the site named at signup; an
    /// org without one gets the default site.
    pub async fn ensure_cloud_setup(
        &self,
        organization_id: Uuid,
        requested: Option<&PendingSiteSetup>,
        authentication: AuthenticatedEntity,
    ) -> Result<(), Error> {
        let mut sites = self
            .site_service
            .get_all(StorableFilter::<Site>::new_from_org_id(&organization_id))
            .await?;

        if sites.is_empty() {
            let mut site = Site::new(SiteBase::new(organization_id));
            if let Some(requested) = requested {
                site.id = requested.site_id;
                site.base.name = requested.name.clone();
            }
            sites.push(
                self.site_service
                    .create(site, authentication.clone())
                    .await?,
            );
        }

        for site in &sites {
            self.ensure_site_setup(site.id, authentication.clone())
                .await?;
        }

        Ok(())
    }

    /// Create whichever of the site's system subnets and live topology row
    /// are missing.
    pub async fn ensure_site_setup(
        &self,
        site_id: Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<(), Error> {
        let has_topology = !self
            .get_all(StorableFilter::<Topology>::new_from_site_ids(&[site_id]))
            .await?
            .is_empty();
        let subnet_types: Vec<SubnetType> = self
            .subnet_service
            .get_all(StorableFilter::<Subnet>::new_from_site_ids(&[site_id]))
            .await?
            .into_iter()
            .map(|s| s.base.subnet_type)
            .collect();

        let gaps = SiteSetupGaps::find(has_topology, &subnet_types);

        for subnet_type in gaps.subnet_types {
            let subnet = match subnet_type {
                SubnetType::Internet => create_wan_subnet(site_id),
                SubnetType::Remote => create_remote_subnet(site_id),
                _ => continue,
            };
            self.subnet_service
                .create(subnet, authentication.clone())
                .await?;
        }

        if gaps.topology {
            self.create(Topology::new(TopologyBase::new(site_id)), authentication)
                .await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Subscriber<OnboardingOperation> for TopologyService {
    fn filter(&self) -> EventFilter<OnboardingOperation> {
        EventFilter::ops(vec![OnboardingOperationDiscriminants::OrgCreated])
    }

    /// Inline (no debounce): registration provisions the integrated daemon on
    /// this site right after `OrgCreated` is published.
    async fn handle(&self, events: Vec<Event<OnboardingOperation>>) -> Result<(), Error> {
        for event in events {
            // No requested site means a self-hosted license buyer; the org
            // is set up if it later lands on a cloud plan.
            if let OnboardingOperation::OrgCreated {
                site: Some(site), ..
            } = &event.operation
            {
                self.ensure_cloud_setup(
                    event.scope.organization_id,
                    Some(site),
                    event.authentication.clone(),
                )
                .await?;
            }
        }
        Ok(())
    }
}

inventory::submit!(SubscriberRegistration::new::<
    TopologyService,
    OnboardingOperation,
>());

#[async_trait]
impl Subscriber<BillingOperation> for TopologyService {
    fn filter(&self) -> EventFilter<BillingOperation> {
        EventFilter::ops(vec![
            BillingOperationDiscriminants::TrialStarted,
            BillingOperationDiscriminants::CheckoutCompleted,
            BillingOperationDiscriminants::PlanChanged,
        ])
    }

    async fn handle(&self, events: Vec<Event<BillingOperation>>) -> Result<(), Error> {
        for event in events {
            if lands_on_cloud_plan(&event.operation) {
                self.ensure_cloud_setup(
                    event.scope.organization_id,
                    None,
                    event.authentication.clone(),
                )
                .await?;
            }
        }
        Ok(())
    }
}

inventory::submit!(SubscriberRegistration::new::<
    TopologyService,
    BillingOperation,
>());

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::billing::types::base::PlanConfig;

    fn plan_changed(from: BillingPlan, to: BillingPlan) -> BillingOperation {
        BillingOperation::PlanChanged {
            from,
            to,
            is_downgrade: false,
            next_renewal_at: None,
            license_key_type: None,
        }
    }

    #[test]
    fn only_moves_onto_cloud_trigger_setup() {
        let cloud = BillingPlan::Pro(PlanConfig::default());
        let other_cloud = BillingPlan::Starter(PlanConfig::default());
        let licensed = BillingPlan::SelfHostedStandard(PlanConfig::default());
        let trial = |plan: BillingPlan| BillingOperation::TrialStarted {
            plan,
            trial_end: chrono::Utc::now(),
            trial_days: 14,
        };

        assert!(lands_on_cloud_plan(&trial(cloud)));
        assert!(!lands_on_cloud_plan(&trial(licensed)));
        assert!(lands_on_cloud_plan(&plan_changed(licensed, cloud)));
        assert!(!lands_on_cloud_plan(&plan_changed(cloud, licensed)));
        assert!(!lands_on_cloud_plan(&plan_changed(other_cloud, cloud)));
    }

    #[test]
    fn gaps_cover_only_what_is_missing() {
        assert_eq!(
            SiteSetupGaps::find(false, &[]),
            SiteSetupGaps {
                topology: true,
                subnet_types: vec![SubnetType::Internet, SubnetType::Remote],
            }
        );
        assert_eq!(
            SiteSetupGaps::find(true, &[SubnetType::Lan, SubnetType::Remote]),
            SiteSetupGaps {
                topology: false,
                subnet_types: vec![SubnetType::Internet],
            }
        );
        assert_eq!(
            SiteSetupGaps::find(true, &[SubnetType::Remote, SubnetType::Internet]),
            SiteSetupGaps::default()
        );
    }
}
