//! `TopologyService::ensure_cloud_setup` and the paths that run it (registration and the
//! billing subscriber): an org ends up with one site, its two system subnets and one live
//! topology, no matter how many times setup runs or what the org already had.

use uuid::Uuid;

use super::{organization, site, subnet, test_services};
use crate::server::{
    auth::{
        r#impl::base::{PendingSetup, PendingSiteSetup, ProvisionOrg, ProvisionUserParams},
        middleware::auth::AuthenticatedEntity,
    },
    billing::types::base::BillingPlan,
    config::DeploymentType,
    organizations::r#impl::base::UseCase,
    services::r#impl::categories::ServiceCategory,
    shared::{
        events::{
            traits::{Event, OrgScope},
            types::BillingOperation,
        },
        services::{factory::ServiceFactory, traits::CrudService},
        storage::{filter::StorableFilter, traits::Storage},
    },
    sites::r#impl::Site,
    subnets::r#impl::{base::Subnet, types::SubnetType},
    tags::groups::base::TagGroupType,
    topology::types::{
        base::{Topology, TopologyRequestOptions},
        grouping::ElementRule,
    },
};

/// Sites, subnet types per site, and topology count for an org.
struct OrgSetup {
    sites: Vec<Site>,
    subnet_types: Vec<(Uuid, SubnetType)>,
    topologies: usize,
}

async fn org_setup(services: &ServiceFactory, organization_id: Uuid) -> OrgSetup {
    let sites = services
        .site_service
        .get_all(StorableFilter::<Site>::new_from_org_id(&organization_id))
        .await
        .unwrap();
    let ids: Vec<Uuid> = sites.iter().map(|n| n.id).collect();
    let subnet_types = services
        .subnet_service
        .get_all(StorableFilter::<Subnet>::new_from_site_ids(&ids))
        .await
        .unwrap()
        .into_iter()
        .map(|s| (s.base.site_id, s.base.subnet_type))
        .collect();
    let topologies = services
        .topology_service
        .get_all(StorableFilter::<Topology>::new_from_site_ids(&ids))
        .await
        .unwrap()
        .len();
    OrgSetup {
        sites,
        subnet_types,
        topologies,
    }
}

fn purchasable(matches: fn(&BillingPlan) -> bool) -> BillingPlan {
    crate::server::billing::plans::get_purchasable_plans()
        .into_iter()
        .find(matches)
        .unwrap()
}

fn cloud_plan() -> BillingPlan {
    purchasable(|p| matches!(p, BillingPlan::Pro(_)))
}

fn license_plan() -> BillingPlan {
    purchasable(|p| matches!(p, BillingPlan::SelfHostedStandard(_)))
}

fn plan_changed(from: BillingPlan, to: BillingPlan) -> BillingOperation {
    BillingOperation::PlanChanged {
        from,
        to,
        is_downgrade: false,
        next_renewal_at: None,
        license_key_type: None,
    }
}

async fn publish_billing(services: &ServiceFactory, organization_id: Uuid, op: BillingOperation) {
    services
        .event_bus
        .publish(Event::new(
            OrgScope { organization_id },
            op,
            AuthenticatedEntity::System,
        ))
        .await
        .unwrap();
    services.event_bus.flush().await;
}

/// Register an owner with a new org, as the signup flow does. Returns the org id.
async fn sign_up(services: &ServiceFactory, site: Option<PendingSiteSetup>) -> Uuid {
    services
        .auth_service
        .provision_user(ProvisionUserParams {
            email: "owner@example.com".parse().unwrap(),
            password_hash: Some("hash".to_string()),
            oidc_subject: None,
            oidc_provider: None,
            provision_org: ProvisionOrg::New(PendingSetup {
                org_name: "Acme".to_string(),
                site,
                use_case: UseCase::InternalIt,
            }),
            permissions: None,
            site_ids: Vec::new(),
            terms_accepted_at: None,
            email_verified: true,
            deployment_type: DeploymentType::Cloud,
        })
        .await
        .unwrap()
        .base
        .organization_id
}

fn assert_fresh_signup_shape(setup: &OrgSetup) {
    assert_eq!(setup.sites.len(), 1);
    let mut types: Vec<SubnetType> = setup.subnet_types.iter().map(|(_, t)| *t).collect();
    types.sort_by_key(|t| format!("{t:?}"));
    assert_eq!(types, vec![SubnetType::Internet, SubnetType::Remote]);
    assert_eq!(setup.topologies, 1);
}

#[tokio::test]
async fn ensure_twice_creates_the_requested_site_once() {
    let (storage, services, _container) = test_services().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();

    let requested = PendingSiteSetup {
        name: "Head Office".to_string(),
        site_id: Uuid::new_v4(),
    };
    for _ in 0..2 {
        services
            .topology_service
            .ensure_cloud_setup(org.id, Some(&requested), AuthenticatedEntity::System)
            .await
            .unwrap();
    }

    let setup = org_setup(&services, org.id).await;
    assert_fresh_signup_shape(&setup);
    assert_eq!(setup.sites[0].id, requested.site_id);
    assert_eq!(setup.sites[0].base.name, "Head Office");
}

#[tokio::test]
async fn ensure_fills_gaps_on_existing_sites_without_adding_one() {
    let (storage, services, _container) = test_services().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();

    // A site holding only a LAN subnet and no topology.
    let net = site(&org.id);
    storage.sites.create(&net).await.unwrap();
    storage.subnets.create(&subnet(&net.id)).await.unwrap();

    services
        .topology_service
        .ensure_cloud_setup(org.id, None, AuthenticatedEntity::System)
        .await
        .unwrap();

    let setup = org_setup(&services, org.id).await;
    assert_eq!(setup.sites.len(), 1);
    assert_eq!(setup.sites[0].id, net.id);
    assert_eq!(setup.subnet_types.len(), 3);
    assert!(setup.subnet_types.contains(&(net.id, SubnetType::Lan)));
    assert_eq!(setup.topologies, 1);
}

#[tokio::test]
async fn self_hosted_signup_gets_no_site_until_it_moves_to_cloud() {
    let (_storage, services, _container) = test_services().await;
    let org_id = sign_up(&services, None).await;

    publish_billing(
        &services,
        org_id,
        BillingOperation::TrialStarted {
            plan: license_plan(),
            trial_end: chrono::Utc::now(),
            trial_days: 14,
        },
    )
    .await;
    assert!(org_setup(&services, org_id).await.sites.is_empty());
    // Every new org gets the system tag groups, with or without a site.
    let groups = services
        .tag_group_service
        .get_for_org(&org_id)
        .await
        .unwrap();
    assert!(TagGroupType::system().all(|t| groups.iter().any(|g| g.base.group_type == t)));

    publish_billing(
        &services,
        org_id,
        plan_changed(license_plan(), cloud_plan()),
    )
    .await;
    // A repeat delivery adds nothing.
    publish_billing(
        &services,
        org_id,
        plan_changed(license_plan(), cloud_plan()),
    )
    .await;

    assert_fresh_signup_shape(&org_setup(&services, org_id).await);
}

#[tokio::test]
async fn cloud_signup_has_its_site_when_registration_returns() {
    let (_storage, services, _container) = test_services().await;

    let requested = PendingSiteSetup {
        name: "Lab".to_string(),
        site_id: Uuid::new_v4(),
    };
    // No flush: the integrated daemon is provisioned on this site right after.
    let org_id = sign_up(&services, Some(requested.clone())).await;
    let setup = org_setup(&services, org_id).await;
    assert_fresh_signup_shape(&setup);
    assert_eq!(setup.sites[0].id, requested.site_id);

    // Picking a cloud plan afterwards leaves the signup site as it is.
    publish_billing(
        &services,
        org_id,
        BillingOperation::TrialStarted {
            plan: cloud_plan(),
            trial_end: chrono::Utc::now(),
            trial_days: 14,
        },
    )
    .await;

    let setup = org_setup(&services, org_id).await;
    assert_fresh_signup_shape(&setup);
    assert_eq!(setup.sites[0].id, requested.site_id);
}

/// Registration and a billing event landing at once must not both create a site.
#[tokio::test]
async fn concurrent_setup_runs_create_one_site() {
    let (storage, services, _container) = test_services().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();

    let requested = PendingSiteSetup {
        name: "Lab".to_string(),
        site_id: Uuid::new_v4(),
    };
    let topology = &services.topology_service;
    let (a, b) = tokio::join!(
        topology.ensure_cloud_setup(org.id, Some(&requested), AuthenticatedEntity::System),
        topology.ensure_cloud_setup(org.id, None, AuthenticatedEntity::System),
    );
    a.unwrap();
    b.unwrap();

    assert_fresh_signup_shape(&org_setup(&services, org.id).await);
}

fn infra_categories(options: &TopologyRequestOptions) -> Vec<ServiceCategory> {
    options
        .element_rules
        .iter()
        .find_map(|r| match &r.rule {
            ElementRule::ByServiceCategory {
                categories,
                is_infra_rule: true,
                ..
            } => Some(categories.clone()),
            _ => None,
        })
        .expect("a new topology carries an infrastructure rule")
}

/// The topology a site is created with groups infrastructure by its own org's use case, and the
/// default options each org is offered are its own.
#[tokio::test]
async fn new_topology_and_default_options_follow_each_orgs_use_case() {
    let (storage, services, _container) = test_services().await;
    let mut msp = organization();
    msp.base.use_case = UseCase::Msp;
    let mut homelab = organization();
    homelab.base.use_case = UseCase::Homelab;
    storage.organizations.create(&msp).await.unwrap();
    storage.organizations.create(&homelab).await.unwrap();

    services
        .topology_service
        .ensure_cloud_setup(msp.id, None, AuthenticatedEntity::System)
        .await
        .unwrap();
    let site_ids: Vec<Uuid> = org_setup(&services, msp.id)
        .await
        .sites
        .iter()
        .map(|s| s.id)
        .collect();
    let topology = services
        .topology_service
        .get_all(StorableFilter::<Topology>::new_from_site_ids(&site_ids))
        .await
        .unwrap()
        .remove(0);
    let stored = infra_categories(&topology.base.options.request);
    assert!(!stored.contains(&ServiceCategory::NetworkAppliance));
    assert!(stored.contains(&ServiceCategory::IoT));

    let for_homelab = services
        .topology_service
        .default_request_options(homelab.id)
        .await
        .unwrap();
    let homelab_infra = infra_categories(&for_homelab);
    assert!(homelab_infra.contains(&ServiceCategory::NetworkAppliance));
    assert!(!homelab_infra.contains(&ServiceCategory::IoT));
}
