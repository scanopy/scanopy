//! Resetting an organization gives it back the setup data it started with: the same tag groups,
//! starter tags, site, system subnets, live topology and default credentials as when it was new,
//! for a self-hosted org, a cloud signup with a site and a cloud license buyer without one.

use std::sync::Arc;

use secrecy::SecretString;
use strum::IntoDiscriminant;
use uuid::Uuid;

use super::self_hosted_licensing::{cloud_test_state, test_state_from};
use crate::server::{
    auth::{
        r#impl::base::{PendingSetup, PendingSiteSetup, ProvisionOrg, ProvisionUserParams},
        middleware::auth::AuthenticatedEntity,
    },
    billing::types::base::BillingPlan,
    config::{AppState, ServerConfig, get_deployment_type},
    credentials::r#impl::{
        base::{Credential, CredentialBase},
        types::{CredentialType, SecretValue},
    },
    organizations::{handlers::reset_organization, r#impl::base::UseCase},
    shared::{
        events::types::UpdateReason,
        services::traits::CrudService,
        storage::{filter::StorableFilter, traits::Storable},
    },
    sites::r#impl::{Site, SiteBase},
    subnets::r#impl::base::Subnet,
    tags::r#impl::base::{Tag, TagBase},
    topology::types::base::Topology,
};

/// The setup data an org holds, in an order that doesn't depend on ids or creation order.
#[derive(Debug, PartialEq)]
struct SetupData {
    /// Name and type of each tag group.
    tag_groups: Vec<(String, String)>,
    /// Name of each live tag and of its group.
    tags: Vec<(String, Option<String>)>,
    /// Per site: its subnet types, topology count and the names of its credentials.
    sites: Vec<(Vec<String>, usize, Vec<String>)>,
    /// Name and type of each default credential.
    default_credentials: Vec<(String, String)>,
}

async fn setup_data(state: &AppState, organization_id: Uuid) -> SetupData {
    let services = &state.services;

    let groups = services
        .tag_group_service
        .get_for_org(&organization_id)
        .await
        .unwrap();
    let mut tag_groups: Vec<(String, String)> = groups
        .iter()
        .map(|g| (g.base.name.clone(), g.base.group_type.to_string()))
        .collect();
    tag_groups.sort();

    let mut tags: Vec<(String, Option<String>)> = services
        .tag_service
        .get_all(StorableFilter::<Tag>::new_from_org_id(&organization_id).live())
        .await
        .unwrap()
        .into_iter()
        .map(|t| {
            let group = groups
                .iter()
                .find(|g| Some(g.id) == t.base.tag_group_id)
                .map(|g| g.base.name.clone());
            (t.base.name, group)
        })
        .collect();
    tags.sort();

    let credentials = services
        .credential_service
        .get_all(StorableFilter::<Credential>::new_from_org_id(
            &organization_id,
        ))
        .await
        .unwrap();
    let credential_name = |id: &Uuid| {
        credentials
            .iter()
            .find(|c| c.id == *id)
            .map(|c| c.base.name.clone())
            .unwrap()
    };

    let mut sites = Vec::new();
    for site in services
        .site_service
        .get_all(StorableFilter::<Site>::new_from_org_id(&organization_id))
        .await
        .unwrap()
    {
        let mut subnet_types: Vec<String> = services
            .subnet_service
            .get_all(StorableFilter::<Subnet>::new_from_site_ids(&[site.id]))
            .await
            .unwrap()
            .iter()
            .map(|s| format!("{:?}", s.base.subnet_type()))
            .collect();
        subnet_types.sort();
        let topologies = services
            .topology_service
            .get_all(StorableFilter::<Topology>::new_from_site_ids(&[site.id]))
            .await
            .unwrap()
            .len();
        let mut site_credentials: Vec<String> = services
            .credential_service
            .get_credential_ids_for_site(&site.id)
            .await
            .unwrap()
            .iter()
            .map(credential_name)
            .collect();
        site_credentials.sort();
        sites.push((subnet_types, topologies, site_credentials));
    }
    sites.sort();

    let mut default_credentials: Vec<(String, String)> = credentials
        .iter()
        .filter(|c| c.is_default)
        .map(|c| {
            (
                c.base.name.clone(),
                c.base.credential_type.discriminant().to_string(),
            )
        })
        .collect();
    default_credentials.sort();

    SetupData {
        tag_groups,
        tags,
        sites,
        default_credentials,
    }
}

/// Register an owner with a new org, as the signup flow does, and wait for its setup.
async fn sign_up(state: &AppState, site: Option<PendingSiteSetup>) -> Uuid {
    let organization_id = state
        .services
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
            site_assignments: Vec::new(),
            terms_accepted_at: None,
            email_verified: true,
            deployment_type: get_deployment_type(&state.config),
        })
        .await
        .unwrap()
        .base
        .organization_id;
    state.services.event_bus.flush().await;
    organization_id
}

fn head_office() -> Option<PendingSiteSetup> {
    Some(PendingSiteSetup {
        name: "Head Office".to_string(),
        site_id: Uuid::new_v4(),
    })
}

/// Work the org has done since signup: a second site, a tag of its own and a credential.
async fn use_org(state: &AppState, organization_id: Uuid) {
    let services = &state.services;
    services
        .site_service
        .create(
            Site::new(SiteBase::new(organization_id)),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    services
        .tag_service
        .create(
            Tag::new(TagBase {
                name: "Printers".to_string(),
                organization_id,
                ..Default::default()
            }),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    services
        .credential_service
        .create(
            Credential::new(CredentialBase {
                organization_id,
                name: "Core switches".to_string(),
                credential_type: CredentialType::SnmpCommunity {
                    community: SecretValue::Inline {
                        value: SecretString::from("private".to_string()),
                    },
                },
                ..Default::default()
            }),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    services.event_bus.flush().await;
}

/// Use the org, reset it, and check it holds what it held when new.
async fn assert_reset_restores(state: &Arc<AppState>, organization_id: Uuid, fresh: &SetupData) {
    use_org(state, organization_id).await;
    assert_ne!(&setup_data(state, organization_id).await, fresh);

    reset_organization(state, organization_id, AuthenticatedEntity::System)
        .await
        .unwrap();
    state.services.event_bus.flush().await;

    assert_eq!(&setup_data(state, organization_id).await, fresh);
}

#[tokio::test]
async fn a_reset_self_hosted_org_matches_a_new_one() {
    let (state, _container) = test_state_from(ServerConfig::default()).await;
    let org = sign_up(&state, head_office()).await;
    let fresh = setup_data(&state, org).await;
    assert_eq!(fresh.sites.len(), 1);
    assert!(!fresh.default_credentials.is_empty());

    assert_reset_restores(&state, org, &fresh).await;
}

#[tokio::test]
async fn a_reset_cloud_signup_matches_a_new_one() {
    let (state, _container) = cloud_test_state().await;
    let org = sign_up(&state, head_office()).await;
    let fresh = setup_data(&state, org).await;
    assert_eq!(fresh.sites.len(), 1);

    assert_reset_restores(&state, org, &fresh).await;
}

/// A cloud license buyer never had the cloud setup, so a reset doesn't give it one, even after it
/// made a site of its own.
#[tokio::test]
async fn a_reset_license_buyer_gets_no_site() {
    let (state, _container) = cloud_test_state().await;
    let org = sign_up(&state, None).await;
    let license_plan = crate::server::billing::plans::get_purchasable_plans()
        .into_iter()
        .find(|p| matches!(p, BillingPlan::SelfHostedStandard(_)))
        .unwrap();
    state
        .services
        .organization_service
        .modify(
            org,
            AuthenticatedEntity::System,
            UpdateReason::OrganizationBillingSynced,
            |o| o.base.plan = license_plan,
        )
        .await
        .unwrap();
    let fresh = setup_data(&state, org).await;
    assert!(fresh.sites.is_empty());
    assert!(!fresh.default_credentials.is_empty());

    assert_reset_restores(&state, org, &fresh).await;
}
