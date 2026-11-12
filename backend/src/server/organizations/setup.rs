//! The data an organization and its sites start with: default credentials, system tag groups,
//! the first site with its system subnets and live topology, starter tags and the integrated
//! daemon.
//!
//! `OrgSetupService` is the one place this data is created, and it runs only from events:
//! - an org created at signup (`AuthOperation::OrgCreated`) gets its credentials and system
//!   groups, and, when signup named a site, the cloud setup and the integrated daemon;
//! - a billing event that lands an org on a cloud plan gives it the cloud setup, so a self-hosted
//!   license buyer who moves to cloud ends up with what a cloud signup has;
//! - an org reset (`UpdateReason::OrganizationReset`) gets back what it started with;
//! - a new site (`EntityOperation::Created`) gets its system subnets and live topology.
//!
//! Every step fills only what's missing, so a bus retry or a repeat event never duplicates
//! anything. `LockKey::CloudSetup` keeps two runs for one org from both creating a site, and
//! `LockKey::SiteSetup` two runs for one site from both creating its subnets.

use std::{collections::HashMap, sync::Arc};

use anyhow::Error;
use async_trait::async_trait;
use uuid::Uuid;

use crate::daemon::shared::config::DEFAULT_DAEMON_NAME;
use crate::server::{
    auth::{r#impl::base::PendingSiteSetup, middleware::auth::AuthenticatedEntity},
    billing::types::base::BillingPlan,
    config::DeploymentType,
    credentials::service::CredentialService,
    daemons::{
        r#impl::{
            api::ProvisionDaemonRequest,
            base::{Daemon, DaemonMode, DaemonOs},
        },
        service::DaemonService,
    },
    organizations::service::OrganizationService,
    shared::{
        entities::{Entity, EntityDiscriminants},
        events::{
            registry::SubscriberRegistration,
            traits::{EntityEventFilter, Event, EventFilter, Subscriber},
            types::{
                AuthOperation, AuthOperationDiscriminants, BillingOperation,
                BillingOperationDiscriminants, EntityOperation, EntityOperationDiscriminants,
                UpdateReason,
            },
        },
        services::traits::CrudService,
        storage::{
            filter::StorableFilter,
            lock::{DEFAULT_LOCK_TIMEOUT, LockKey},
            seed_data::{
                create_remote_subnet, create_starter_tags, create_wan_subnet,
                starter_custom_group_names,
            },
            traits::Storable,
        },
    },
    sites::{
        r#impl::{Site, SiteBase},
        service::SiteService,
    },
    subnets::{r#impl::base::Subnet, r#impl::types::SubnetType, service::SubnetService},
    tags::{
        groups::{base::TagGroup, service::TagGroupService},
        r#impl::base::Tag,
        service::TagService,
    },
    topology::{
        service::main::TopologyService,
        types::base::{Topology, TopologyBase},
    },
};

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
            subnet_types: SubnetType::seeded_per_site()
                .iter()
                .copied()
                .filter(|t| !existing_subnet_types.contains(t))
                .collect(),
        }
    }
}

/// The starter tags an org lacks, by name, in its `groups`. Names compare case-insensitively, so
/// an org that already made its own "active" tag doesn't get a second one.
pub(crate) fn missing_starter_tags(
    organization_id: Uuid,
    groups: &[TagGroup],
    existing_names: &[String],
) -> Vec<Tag> {
    create_starter_tags(organization_id, groups)
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

/// Whether a reset org gets the cloud setup back: whether it was given one. Signup on a
/// self-hosted server always names a site, and an org on a cloud plan was set up when it chose
/// it. A cloud org on a license plan never had one. A cloud org that hasn't picked a plan had one
/// only if signup named a site, which `had_sites` stands in for once the sites are deleted.
pub(crate) fn reset_gets_cloud_setup(
    deployment_type: DeploymentType,
    plan: Option<&BillingPlan>,
    had_sites: bool,
) -> bool {
    if deployment_type.is_self_hosted() {
        return true;
    }
    match plan {
        Some(plan) => plan.license_plan().is_none(),
        None => had_sites,
    }
}

pub struct OrgSetupService {
    organization_service: Arc<OrganizationService>,
    credential_service: Arc<CredentialService>,
    tag_service: Arc<TagService>,
    tag_group_service: Arc<TagGroupService>,
    site_service: Arc<SiteService>,
    subnet_service: Arc<SubnetService>,
    topology_service: Arc<TopologyService>,
    daemon_service: Arc<DaemonService>,
    deployment_type: DeploymentType,
    /// Where the integrated daemon listens, on a server that runs one.
    integrated_daemon_url: Option<String>,
}

impl OrgSetupService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        organization_service: Arc<OrganizationService>,
        credential_service: Arc<CredentialService>,
        tag_service: Arc<TagService>,
        tag_group_service: Arc<TagGroupService>,
        site_service: Arc<SiteService>,
        subnet_service: Arc<SubnetService>,
        topology_service: Arc<TopologyService>,
        daemon_service: Arc<DaemonService>,
        deployment_type: DeploymentType,
        integrated_daemon_url: Option<String>,
    ) -> Self {
        Self {
            organization_service,
            credential_service,
            tag_service,
            tag_group_service,
            site_service,
            subnet_service,
            topology_service,
            daemon_service,
            deployment_type,
            integrated_daemon_url,
        }
    }

    /// What every org starts with: its default credentials, then its system tag groups.
    pub async fn seed_organization(
        &self,
        organization_id: Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<(), Error> {
        self.credential_service
            .seed_defaults(organization_id)
            .await?;
        self.tag_group_service
            .seed_groups(organization_id, &[], authentication)
            .await?;
        Ok(())
    }

    /// What an org with a site starts with on top of [`Self::seed_organization`], which runs
    /// first so a new site gets the SNMP default. Give the org a site if it has none, fill in
    /// each site's system subnets and live topology, then add the starter groups and tags.
    /// `requested` is the site named at signup; an org without one gets the default site.
    pub async fn seed_cloud_setup(
        &self,
        organization_id: Uuid,
        requested: Option<&PendingSiteSetup>,
        authentication: AuthenticatedEntity,
    ) -> Result<(), Error> {
        let lock = self
            .site_service
            .storage()
            .session_lock(
                LockKey::CloudSetup { organization_id },
                DEFAULT_LOCK_TIMEOUT,
            )
            .await?;

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

        // The new site's Created event seeds it too; seeding it here as well means the org is
        // complete when this returns, and fills gaps on sites the org already had.
        for site in &sites {
            self.seed_site(site.id, authentication.clone()).await?;
        }

        self.seed_starter_tags(organization_id, authentication)
            .await?;

        lock.release().await?;
        Ok(())
    }

    /// Create whichever of the system groups, starter groups and starter tags the org lacks. A
    /// group or tag the org already has by that name is kept as it is, so a person's edits to it
    /// survive a repeat run.
    async fn seed_starter_tags(
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

        let groups = self
            .tag_group_service
            .seed_groups(
                organization_id,
                &starter_custom_group_names(),
                authentication.clone(),
            )
            .await?;

        for tag in missing_starter_tags(organization_id, &groups, &existing) {
            self.tag_service.create(tag, authentication.clone()).await?;
        }
        Ok(())
    }

    /// Create whichever of the site's system subnets and live topology row are missing.
    pub async fn seed_site(
        &self,
        site_id: Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<(), Error> {
        let lock = self
            .site_service
            .storage()
            .session_lock(LockKey::SiteSetup { site_id }, DEFAULT_LOCK_TIMEOUT)
            .await?;

        let has_topology = !self
            .topology_service
            .get_all(StorableFilter::<Topology>::new_from_site_ids(&[site_id]))
            .await?
            .is_empty();
        let subnet_types: Vec<SubnetType> = self
            .subnet_service
            .get_all(StorableFilter::<Subnet>::new_from_site_ids(&[site_id]))
            .await?
            .into_iter()
            .map(|s| s.base.subnet_type())
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
            let site = self
                .site_service
                .get_by_id(&site_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Site {site_id} not found"))?;
            let mut base = TopologyBase::new(site_id);
            base.options.request = self
                .topology_service
                .default_request_options(site.base.organization_id)
                .await?;
            self.topology_service
                .create(Topology::new(base), authentication)
                .await?;
        }

        lock.release().await?;
        Ok(())
    }

    /// Provision the integrated daemon on `site_id` and hand it its key, on a server that runs
    /// one. A site that already has a daemon is left alone, so a repeat run provisions nothing.
    ///
    /// The integrated daemon is provisioned, not handed a shared site key: since 0.17.5 a daemon
    /// that reaches /register without resolving to an existing record is rejected
    /// (`daemon_not_provisioned`), so a shared key would leave it permanently unregistered.
    /// Provisioning creates its record + a 1:1 key up front; the daemon then learns its identity
    /// from that key on first contact.
    pub async fn seed_integrated_daemon(
        &self,
        site_id: Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<(), Error> {
        let Some(integrated_daemon_url) = &self.integrated_daemon_url else {
            return Ok(());
        };
        let has_daemon = !self
            .daemon_service
            .get_all(StorableFilter::<Daemon>::new_from_site_ids(&[site_id]))
            .await?
            .is_empty();
        if has_daemon {
            return Ok(());
        }

        let (_daemon, plaintext) = self
            .daemon_service
            .provision(
                &ProvisionDaemonRequest {
                    name: Some(DEFAULT_DAEMON_NAME.to_string()),
                    site_id: Some(site_id),
                    // The integrated daemon dials the server, so it needs no reachable url.
                    mode: DaemonMode::DaemonPoll,
                    url: None,
                    seed_credential_refs: Vec::new(),
                    daemon_id: None,
                    // The integrated daemon ships in the Linux server image.
                    os: Some(DaemonOs::Linux),
                },
                None,
                authentication,
            )
            .await?;

        self.daemon_service
            .initialize_local_daemon(integrated_daemon_url.clone(), site_id, plaintext)
            .await
    }
}

#[async_trait]
impl Subscriber<AuthOperation> for OrgSetupService {
    fn filter(&self) -> EventFilter<AuthOperation> {
        EventFilter::ops(vec![AuthOperationDiscriminants::OrgCreated])
    }

    /// A requested site means a cloud signup or a self-hosted server. No requested site means a
    /// self-hosted license buyer on cloud; the org gets the cloud setup if it later lands on a
    /// cloud plan.
    async fn handle(&self, events: Vec<Event<AuthOperation>>) -> Result<(), Error> {
        for event in events {
            let AuthOperation::OrgCreated { site, .. } = &event.operation else {
                continue;
            };
            let Some(organization_id) = event.scope.organization_id else {
                continue;
            };
            self.seed_organization(organization_id, event.authentication.clone())
                .await?;
            if let Some(site) = site {
                self.seed_cloud_setup(organization_id, Some(site), event.authentication.clone())
                    .await?;
                self.seed_integrated_daemon(site.site_id, event.authentication.clone())
                    .await?;
            }
        }
        Ok(())
    }
}

inventory::submit!(SubscriberRegistration::new::<OrgSetupService, AuthOperation>());

#[async_trait]
impl Subscriber<BillingOperation> for OrgSetupService {
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
                let organization_id = event.scope.organization_id;
                self.seed_organization(organization_id, event.authentication.clone())
                    .await?;
                self.seed_cloud_setup(organization_id, None, event.authentication.clone())
                    .await?;
            }
        }
        Ok(())
    }
}

inventory::submit!(SubscriberRegistration::new::<
    OrgSetupService,
    BillingOperation,
>());

#[async_trait]
impl Subscriber<EntityOperation> for OrgSetupService {
    fn filter(&self) -> EntityEventFilter {
        EntityEventFilter::by_entity(HashMap::from([
            (
                EntityDiscriminants::Organization,
                Some(vec![EntityOperationDiscriminants::Updated]),
            ),
            (
                EntityDiscriminants::Site,
                Some(vec![EntityOperationDiscriminants::Created]),
            ),
        ]))
    }

    async fn handle(&self, events: Vec<Event<EntityOperation>>) -> Result<(), Error> {
        for event in events {
            match (event.scope.entity_type(), &event.operation) {
                (
                    Entity::Organization(_),
                    EntityOperation::Updated {
                        reason: UpdateReason::OrganizationReset { had_sites },
                    },
                ) => {
                    let organization_id = event.scope.entity_id();
                    let Some(org) = self
                        .organization_service
                        .get_by_id(&organization_id)
                        .await?
                    else {
                        continue;
                    };
                    self.seed_organization(organization_id, event.authentication.clone())
                        .await?;
                    if reset_gets_cloud_setup(
                        self.deployment_type,
                        Some(&org.base.plan),
                        *had_sites,
                    ) {
                        self.seed_cloud_setup(organization_id, None, event.authentication.clone())
                            .await?;
                    }
                }
                (Entity::Site(site), EntityOperation::Created) => {
                    self.seed_site(site.id, event.authentication.clone())
                        .await?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}

inventory::submit!(SubscriberRegistration::new::<
    OrgSetupService,
    EntityOperation,
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
            trial_end: crate::server::shared::clock::now(),
            trial_days: 14,
        };

        assert!(lands_on_cloud_plan(&trial(cloud)));
        assert!(!lands_on_cloud_plan(&trial(licensed)));
        assert!(lands_on_cloud_plan(&plan_changed(licensed, cloud)));
        assert!(!lands_on_cloud_plan(&plan_changed(cloud, licensed)));
        assert!(!lands_on_cloud_plan(&plan_changed(other_cloud, cloud)));
    }

    /// A reset gives back the cloud setup only to an org that was given one.
    #[test]
    fn a_reset_restores_cloud_setup_only_where_setup_gave_it() {
        let cloud = BillingPlan::Pro(PlanConfig::default());
        let licensed = BillingPlan::SelfHostedStandard(PlanConfig::default());
        let community = BillingPlan::Community(PlanConfig::default());

        // A self-hosted server's signup always names a site.
        assert!(reset_gets_cloud_setup(
            DeploymentType::Community,
            Some(&community),
            false
        ));
        // On cloud: a cloud plan was set up when chosen; a license plan never was.
        assert!(reset_gets_cloud_setup(
            DeploymentType::Cloud,
            Some(&cloud),
            false
        ));
        assert!(!reset_gets_cloud_setup(
            DeploymentType::Cloud,
            Some(&licensed),
            true
        ));
        // No plan yet: only a signup that named a site had one.
        assert!(reset_gets_cloud_setup(DeploymentType::Cloud, None, true));
        assert!(!reset_gets_cloud_setup(DeploymentType::Cloud, None, false));
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

    /// The groups seed_groups gives a new org: the system groups and the starter custom groups.
    fn starter_groups(org: Uuid) -> Vec<TagGroup> {
        use crate::server::tags::groups::base::{TagGroupBase, TagGroupType};

        TagGroupType::system()
            .map(|t| TagGroupBase::new(org, t.to_string(), t))
            .chain(
                starter_custom_group_names()
                    .into_iter()
                    .map(|name| TagGroupBase::new(org, name.to_string(), TagGroupType::Custom)),
            )
            .map(TagGroup::new)
            .collect()
    }

    #[test]
    fn a_new_org_gets_every_starter_tag_in_its_group_in_order() {
        let org = Uuid::new_v4();
        let groups = starter_groups(org);
        let tags = missing_starter_tags(org, &groups, &[]);
        assert!(
            tags.iter()
                .all(|t| t.base.organization_id == org && t.base.icon.is_some())
        );
        let group_of = |name: &str| {
            let tag = tags.iter().find(|t| t.base.name == name).unwrap();
            let group = groups
                .iter()
                .find(|g| Some(g.id) == tag.base.tag_group_id)
                .unwrap();
            (group.base.name.clone(), tag.base.position)
        };
        assert_eq!(group_of("Planned"), ("Status".to_string(), Some(1)));
        assert_eq!(group_of("Decommissioned"), ("Status".to_string(), Some(4)));
        assert_eq!(group_of("Production").0, "Environment");
        assert_eq!(group_of("Critical").0, "Criticality");
        assert_eq!(group_of("Third-party").0, "Management");
    }

    #[test]
    fn a_second_run_creates_nothing() {
        let org = Uuid::new_v4();
        let groups = starter_groups(org);
        let existing: Vec<String> = missing_starter_tags(org, &groups, &[])
            .into_iter()
            .map(|t| t.base.name)
            .collect();
        assert!(missing_starter_tags(org, &groups, &existing).is_empty());
    }

    /// A tag the org made itself under one of these names is kept, never duplicated.
    #[test]
    fn an_existing_tag_of_the_same_name_is_kept() {
        let org = Uuid::new_v4();
        let groups = starter_groups(org);
        let all = missing_starter_tags(org, &groups, &[]);
        let taken = all[0].base.name.to_lowercase();
        let missing = missing_starter_tags(org, &groups, &[taken.clone(), "Printers".to_string()]);
        assert_eq!(missing.len(), all.len() - 1);
        assert!(
            missing
                .iter()
                .all(|t| !t.base.name.eq_ignore_ascii_case(&taken))
        );
    }
}
