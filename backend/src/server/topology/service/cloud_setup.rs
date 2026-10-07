//! Cloud setup: the site, its system subnets, the live topology row and the default Status tags a
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
            seed_data::{create_remote_subnet, create_status_tags, create_wan_subnet},
            traits::Storable,
        },
    },
    sites::r#impl::{Site, SiteBase},
    subnets::{r#impl::base::Subnet, r#impl::types::SubnetType},
    tags::r#impl::base::Tag,
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

/// The default Status tags an org lacks, by name. Names compare case-insensitively, so an org
/// that already made its own "active" tag doesn't get a second one.
pub(crate) fn missing_status_tags(organization_id: Uuid, existing_names: &[String]) -> Vec<Tag> {
    create_status_tags(organization_id)
        .into_iter()
        .filter(|tag| {
            !existing_names
                .iter()
                .any(|name| name.eq_ignore_ascii_case(&tag.base.name))
        })
        .collect()
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

        self.ensure_status_tags(organization_id, authentication)
            .await?;

        Ok(())
    }

    /// Create whichever of the default Status tags the org lacks. A tag the org already has by
    /// that name is kept as it is, so a person's edits to it survive a repeat run.
    async fn ensure_status_tags(
        &self,
        organization_id: Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<(), Error> {
        let existing: Vec<String> = self
            .tag_service
            .get_all(StorableFilter::<Tag>::new_from_org_id(&organization_id).live())
            .await?
            .into_iter()
            .map(|tag| tag.base.name)
            .collect();

        for tag in missing_status_tags(organization_id, &existing) {
            self.tag_service.create(tag, authentication.clone()).await?;
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

    #[test]
    fn a_new_org_gets_every_status_tag_in_the_status_group() {
        let org = Uuid::new_v4();
        let tags = missing_status_tags(org, &[]);
        assert!(!tags.is_empty());
        assert!(tags.iter().all(|t| t.base.organization_id == org
            && t.base.tag_group
                == Some(crate::server::tags::r#impl::base::TagGroup::Named {
                    name: crate::server::shared::storage::seed_data::STATUS_TAG_GROUP.to_string()
                })
            && t.base.icon.is_some()));
    }

    #[test]
    fn a_second_run_creates_nothing() {
        let org = Uuid::new_v4();
        let existing: Vec<String> = missing_status_tags(org, &[])
            .into_iter()
            .map(|t| t.base.name)
            .collect();
        assert!(missing_status_tags(org, &existing).is_empty());
    }

    /// A tag the org made itself under one of these names is kept, never duplicated.
    #[test]
    fn an_existing_tag_of_the_same_name_is_kept() {
        let org = Uuid::new_v4();
        let all = missing_status_tags(org, &[]);
        let taken = all[0].base.name.to_lowercase();
        let missing = missing_status_tags(org, &[taken.clone(), "Critical".to_string()]);
        assert_eq!(missing.len(), all.len() - 1);
        assert!(
            missing
                .iter()
                .all(|t| !t.base.name.eq_ignore_ascii_case(&taken))
        );
    }
}
