//! Lists paged through `list_page`, against a database: each group's count covers every page,
//! filters narrow on the server, and no row outside the caller's sites or organization appears.

use chrono::Utc;
use uuid::Uuid;

use crate::server::shared::attribution::AttributeSource;
use crate::server::shared::handlers::list::list_page;
use crate::server::shared::storage::traits::Storage;
use crate::server::shared::types::api::GroupCount;
use crate::server::vlans::handlers::{VlanFilterQuery, VlanOrderField};
use crate::server::vlans::r#impl::base::{Vlan, VlanBase, VlanName, VlanNameValue};

use super::{list_scope, organization, site, subnet, test_state, user};
use crate::server::shared::storage::traits::Storable;

fn counts(groups: Option<&[GroupCount]>) -> Vec<(String, u64)> {
    groups
        .expect("a grouped list counts its groups")
        .iter()
        .map(|g| (g.value.clone().unwrap_or_default(), g.count))
        .collect()
}

fn vlan(site_id: Uuid, organization_id: Uuid, number: u16) -> Vlan {
    let now = Utc::now();
    Vlan {
        id: Uuid::new_v4(),
        created_at: now,
        updated_at: now,
        valid_from: now,
        last_seen_at: now,
        base: VlanBase {
            vlan_number: number,
            name: VlanName::new(
                VlanNameValue(format!("vlan-{number}")),
                AttributeSource::Manual,
            ),
            site_id,
            organization_id,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[tokio::test]
async fn a_grouped_vlan_page_counts_every_group_and_stays_in_the_callers_sites() {
    let (storage, state, _container) = test_state().await;

    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let (office, lab) = (site(&org.id), site(&org.id));
    storage.sites.create(&office).await.unwrap();
    storage.sites.create(&lab).await.unwrap();

    let other_org = organization();
    storage.organizations.create(&other_org).await.unwrap();
    let foreign = site(&other_org.id);
    storage.sites.create(&foreign).await.unwrap();

    for number in [10, 20, 30] {
        storage
            .vlans
            .create(&vlan(office.id, org.id, number))
            .await
            .unwrap();
    }
    let lab_vlan = vlan(lab.id, org.id, 40);
    storage.vlans.create(&lab_vlan).await.unwrap();
    storage
        .vlans
        .create(&vlan(foreign.id, other_org.id, 50))
        .await
        .unwrap();

    let sites = [office.id, lab.id];
    let grouped = VlanFilterQuery {
        group_by: Some(VlanOrderField::SiteId),
        order_by: Some(VlanOrderField::VlanNumber),
        limit: Some(2),
        offset: Some(1),
        ..Default::default()
    };
    let page = list_page::<Vlan>(&state, &list_scope(&sites, org.id), &grouped)
        .await
        .unwrap();

    let pagination = &page.meta.pagination;
    assert_eq!(
        pagination.total_count, 4,
        "the other org's VLAN is not counted"
    );
    let groups = counts(pagination.group_counts.as_deref());
    assert_eq!(
        groups.iter().map(|(_, count)| count).sum::<u64>(),
        pagination.total_count,
        "the groups cover every matching row, not the page"
    );
    let office_count = groups
        .iter()
        .find(|(key, _)| *key == office.id.to_string())
        .map(|(_, count)| *count);
    assert_eq!(
        office_count,
        Some(3),
        "a group cut by the page keeps its full size"
    );
    assert_eq!(page.data.len(), 2);

    // A site the caller can't see narrows to nothing rather than widening the list.
    let foreign_site = VlanFilterQuery {
        site_ids: Some(vec![foreign.id]),
        ..Default::default()
    };
    let page = list_page::<Vlan>(&state, &list_scope(&sites, org.id), &foreign_site)
        .await
        .unwrap();
    assert_eq!(page.meta.pagination.total_count, 0);

    // A subnet filter narrows through the link table.
    let lab_subnet = subnet(&lab.id);
    storage.subnets.create(&lab_subnet).await.unwrap();
    state
        .services
        .vlan_service
        .set_subnet_vlans(&lab_subnet.id, &[lab_vlan.id])
        .await
        .unwrap();
    let on_subnet = VlanFilterQuery {
        subnet_ids: Some(vec![lab_subnet.id]),
        ..Default::default()
    };
    let page = list_page::<Vlan>(&state, &list_scope(&sites, org.id), &on_subnet)
        .await
        .unwrap();
    let ids: Vec<Uuid> = page.data.iter().map(|v| v.id).collect();
    assert_eq!(ids, vec![lab_vlan.id]);
}

/// A user's API keys list, its CSV export and its filter options hold that user's keys only. The
/// export once read through the organization-wide default and returned every member's keys.
#[tokio::test]
async fn user_api_keys_list_only_the_callers_own() {
    use crate::server::shared::handlers::traits::{CrudHandlers, ListScope, field_values_filter};
    use crate::server::shared::services::traits::CrudService;
    use crate::server::user_api_keys::handlers::{UserApiKeyFilterQuery, UserApiKeyOrderField};
    use crate::server::user_api_keys::r#impl::base::{UserApiKey, UserApiKeyBase};
    use crate::server::users::r#impl::permissions::UserOrgPermissions;

    let (storage, state, _container) = test_state().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let net = site(&org.id);
    storage.sites.create(&net).await.unwrap();
    let (mut me, mut colleague) = (user(&org.id), user(&org.id));
    me.base.email = email_address::EmailAddress::new_unchecked("me@porto.example");
    colleague.base.email = email_address::EmailAddress::new_unchecked("colleague@porto.example");
    storage.users.create(&me).await.unwrap();
    storage.users.create(&colleague).await.unwrap();

    let key = |owner: Uuid, name: &str| {
        UserApiKey::new(UserApiKeyBase {
            name: name.to_string(),
            user_id: owner,
            organization_id: org.id,
            key: Uuid::new_v4().to_string(),
            is_enabled: true,
            ..Default::default()
        })
    };
    let mine = key(me.id, "ci");
    storage.user_api_keys.create(&mine).await.unwrap();
    storage
        .user_api_keys
        .create(&key(colleague.id, "backup"))
        .await
        .unwrap();

    let scope = ListScope {
        user_id: Some(me.id),
        organization_grants: Some(UserOrgPermissions::Admin.grants()),
        ..list_scope(&[net.id], org.id)
    };
    let query = UserApiKeyFilterQuery::default();

    let page = list_page::<UserApiKey>(&state, &scope, &query)
        .await
        .unwrap();
    let ids: Vec<Uuid> = page.data.iter().map(|k| k.id).collect();
    assert_eq!(ids, vec![mine.id]);

    let exported = state
        .services
        .user_api_key_service
        .get_all(
            UserApiKey::list_filter(&state, &query, &scope)
                .await
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        exported.iter().map(|k| k.id).collect::<Vec<_>>(),
        vec![mine.id]
    );

    let options = state
        .services
        .user_api_key_service
        .count_by_group(
            field_values_filter::<UserApiKey>(&scope, &query, UserApiKeyOrderField::Name),
            "user_api_keys.name",
        )
        .await
        .unwrap();
    assert_eq!(options.iter().map(|g| g.count).sum::<u64>(), 1);
}

/// Credentials sort, group and filter on what their type's metadata says, through the SQL built
/// from that metadata: integration, beta, and the targets a type can be applied to. Another
/// organization's credentials never appear.
#[tokio::test]
async fn credentials_group_and_filter_on_their_types_metadata() {
    use crate::server::credentials::handlers::{CredentialFilterQuery, CredentialOrderField};
    use crate::server::credentials::r#impl::base::{Credential, CredentialBase};
    use crate::server::credentials::r#impl::types::Target;
    use crate::server::credentials::r#impl::types::{
        CredentialIntegration, CredentialTypeDiscriminants,
    };

    let (storage, state, _container) = test_state().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let other_org = organization();
    storage.organizations.create(&other_org).await.unwrap();

    let credential = |organization_id: Uuid, t: CredentialTypeDiscriminants| {
        Credential::new(CredentialBase {
            name: format!("{t}"),
            organization_id,
            credential_type: t.to_credential_type(),
            ..Default::default()
        })
    };
    let snmp = credential(org.id, CredentialTypeDiscriminants::SnmpCommunity);
    let gnmi = credential(org.id, CredentialTypeDiscriminants::Gnmi);
    let docker = credential(org.id, CredentialTypeDiscriminants::DockerSocket);
    for c in [&snmp, &gnmi, &docker] {
        storage.credentials.create(c).await.unwrap();
    }
    storage
        .credentials
        .create(&credential(
            other_org.id,
            CredentialTypeDiscriminants::SnmpV3,
        ))
        .await
        .unwrap();
    let scope = list_scope(&[], org.id);

    let grouped = CredentialFilterQuery {
        group_by: Some(CredentialOrderField::Integration),
        ..Default::default()
    };
    let page = list_page::<Credential>(&state, &scope, &grouped)
        .await
        .unwrap();
    let mut groups = counts(page.meta.pagination.group_counts.as_deref());
    groups.sort();
    assert_eq!(
        groups,
        vec![
            ("Docker".to_string(), 1),
            ("Gnmi".to_string(), 1),
            ("Snmp".to_string(), 1)
        ],
        "grouped by the integration id the UI groups on, and only this organization's"
    );

    let ids = |query: CredentialFilterQuery| {
        let state = &state;
        let scope = &scope;
        async move {
            let mut ids: Vec<Uuid> = list_page::<Credential>(state, scope, &query)
                .await
                .unwrap()
                .data
                .iter()
                .map(|c| c.id)
                .collect();
            ids.sort();
            ids
        }
    };
    let mut snmp_and_gnmi = vec![snmp.id, gnmi.id];
    snmp_and_gnmi.sort();

    assert_eq!(
        ids(CredentialFilterQuery {
            integrations: Some(vec![CredentialIntegration::Docker]),
            ..Default::default()
        })
        .await,
        vec![docker.id]
    );
    assert_eq!(
        ids(CredentialFilterQuery {
            beta: Some(vec![true]),
            ..Default::default()
        })
        .await,
        vec![gnmi.id]
    );
    // A socket reaches only the daemon's own host; SNMP and gNMI can broadcast to a site.
    assert_eq!(
        ids(CredentialFilterQuery {
            targets: Some(vec![Target::Site]),
            ..Default::default()
        })
        .await,
        snmp_and_gnmi
    );
}

/// Every filter the credentials list documents narrows the rows.
#[tokio::test]
async fn every_documented_credential_filter_narrows_the_rows() {
    use crate::server::credentials::handlers::CredentialFilterQuery;
    use crate::server::shared::handlers::query::narrowing_check::assert_every_param_narrows;

    let scope = list_scope(&[Uuid::new_v4()], Uuid::new_v4());
    let id = Uuid::new_v4().to_string();
    assert_every_param_narrows(
        |query: &CredentialFilterQuery| query.narrow(&scope).expect("filter builds"),
        &[
            ("type", "SnmpCommunity"),
            ("integrations", "Docker"),
            ("beta", "true"),
            ("unofficial_api", "true"),
            ("targets", "Site"),
            ("daemon_os", "Windows"),
            ("site_ids", id.as_str()),
            ("tag_ids", id.as_str()),
            ("tag_sets", id.as_str()),
            ("search", "lab"),
        ],
        &[
            "limit",
            "offset",
            "group_by",
            "order_by",
            "order_direction",
            "group_by_tag_group",
            "order_by_tag_group",
        ],
    )
    .await;
}

/// Daemons sort by version in version order and group and filter by the status the server
/// derives, in memory, with totals and group counts that cover every row rather than the page.
#[tokio::test]
async fn daemons_order_and_filter_on_what_code_derives() {
    use crate::server::daemons::handlers::{DaemonFilterQuery, DaemonOrderField};
    use crate::server::daemons::r#impl::api::DaemonHealth;
    use crate::server::daemons::r#impl::base::Daemon;
    use crate::server::shared::handlers::query::OrderDirection;

    let (storage, state, _container) = test_state().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let net = site(&org.id);
    storage.sites.create(&net).await.unwrap();

    let maintainer = user(&org.id);
    storage.users.create(&maintainer).await.unwrap();
    let current = env!("CARGO_PKG_VERSION");
    for (name, version, unreachable) in [
        ("ten", "0.17.10", false),
        ("nine", "0.17.9", false),
        ("current", current, false),
        ("lost", current, true),
    ] {
        let daemon_host = super::host(&net.id);
        storage.hosts.create(&daemon_host).await.unwrap();
        let mut d = super::daemon(&net.id, &daemon_host.id);
        d.base.user_id = maintainer.id;
        d.base.name = name.to_string();
        d.base.version = Some(version.parse().unwrap());
        d.base.is_unreachable = unreachable;
        storage.daemons.create(&d).await.unwrap();
    }
    let scope = list_scope(&[net.id], org.id);
    let names = |page: &crate::server::shared::types::api::PaginatedApiResponse<
        crate::server::daemons::r#impl::api::DaemonResponse,
    >| {
        page.data
            .iter()
            .map(|d| d.base.name.clone())
            .collect::<Vec<_>>()
    };

    // Whole order: nine, ten, current|lost by name. The page skips one, takes two.
    let by_version = DaemonFilterQuery {
        order_by: Some(DaemonOrderField::Version),
        order_direction: Some(OrderDirection::Asc),
        limit: Some(2),
        offset: Some(1),
        ..Default::default()
    };
    let page = list_page::<Daemon>(&state, &scope, &by_version)
        .await
        .unwrap();
    assert_eq!(page.meta.pagination.total_count, 4);
    assert_eq!(names(&page)[0], "ten", "0.17.10 follows 0.17.9");

    let unreachable = DaemonFilterQuery {
        statuses: Some(vec![DaemonHealth::Unreachable]),
        group_by: Some(DaemonOrderField::Status),
        ..Default::default()
    };
    let page = list_page::<Daemon>(&state, &scope, &unreachable)
        .await
        .unwrap();
    assert_eq!(names(&page), vec!["lost"]);
    assert_eq!(page.meta.pagination.total_count, 1);
    assert_eq!(
        counts(page.meta.pagination.group_counts.as_deref()),
        vec![("unreachable".to_string(), 1)]
    );
}

/// Every filter the daemons list documents narrows the rows. `statuses` is derived in code, so
/// it narrows in the list rather than in this filter.
#[tokio::test]
async fn every_documented_daemon_filter_narrows_the_rows() {
    use crate::server::daemons::handlers::DaemonFilterQuery;
    use crate::server::shared::handlers::query::narrowing_check::assert_every_param_narrows;

    let site = Uuid::new_v4();
    let scope = list_scope(&[site, Uuid::new_v4()], Uuid::new_v4());
    let id = Uuid::new_v4().to_string();
    let site_param = site.to_string();
    assert_every_param_narrows(
        |query: &DaemonFilterQuery| query.narrow(&scope).expect("filter builds"),
        &[
            ("site_ids", site_param.as_str()),
            ("os", "linux"),
            ("modes", "server_poll"),
            ("maintainer_ids", id.as_str()),
            ("versions", "0.17.9"),
            ("interfaced_subnet_ids", id.as_str()),
            ("tag_ids", id.as_str()),
            ("tag_sets", id.as_str()),
            ("search", "lab"),
        ],
        &[
            "statuses",
            "limit",
            "offset",
            "group_by",
            "order_by",
            "order_direction",
            "group_by_tag_group",
            "order_by_tag_group",
        ],
    )
    .await;
}

/// Every filter the discovery lists document narrows the rows.
#[tokio::test]
async fn every_documented_discovery_filter_narrows_the_rows() {
    use crate::server::discovery::handlers::DiscoveryFilterQuery;
    use crate::server::shared::handlers::query::narrowing_check::assert_every_param_narrows;

    let site = Uuid::new_v4();
    let scope = list_scope(&[site, Uuid::new_v4()], Uuid::new_v4());
    let id = Uuid::new_v4().to_string();
    let site_param = site.to_string();
    assert_every_param_narrows(
        |query: &DiscoveryFilterQuery| query.narrow(&scope).expect("filter builds"),
        &[
            ("site_ids", site_param.as_str()),
            ("daemon_ids", id.as_str()),
            ("ids", id.as_str()),
            ("discovery_types", "Unified"),
            ("phases", "Complete"),
            ("historical", "false"),
            ("search", "nightly"),
            ("run_types", "Scheduled"),
            ("legacy", "true"),
            ("force_full_scan", "true"),
            ("tag_ids", id.as_str()),
            ("tag_sets", id.as_str()),
        ],
        &[
            "limit",
            "offset",
            "group_by",
            "order_by",
            "order_direction",
            "group_by_tag_group",
            "order_by_tag_group",
        ],
    )
    .await;
}

/// The sites list holds only the caller's sites, and its search reaches the names of what a
/// site holds, so a site is found by one of its subnets.
#[tokio::test]
async fn sites_list_only_the_callers_and_search_reaches_their_subnets() {
    use crate::server::sites::handlers::SiteListQuery;
    use crate::server::sites::r#impl::Site;
    use crate::server::subnets::r#impl::base::SubnetName;

    let (storage, state, _container) = test_state().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let (mine, hidden) = (site(&org.id), site(&org.id));
    storage.sites.create(&mine).await.unwrap();
    storage.sites.create(&hidden).await.unwrap();
    let mut lab = subnet(&mine.id);
    lab.base.name = SubnetName::unattributed("porto-lab".to_string());
    storage.subnets.create(&lab).await.unwrap();

    let scope = list_scope(&[mine.id], org.id);
    let all = list_page::<Site>(&state, &scope, &SiteListQuery::default())
        .await
        .unwrap();
    assert_eq!(
        all.data.iter().map(|s| s.id).collect::<Vec<_>>(),
        vec![mine.id],
        "a site the caller can't reach is not listed"
    );

    let found = list_page::<Site>(
        &state,
        &scope,
        &SiteListQuery {
            search: Some("porto".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(
        found.data.iter().map(|s| s.id).collect::<Vec<_>>(),
        vec![mine.id]
    );
}

/// Every filter the sites list documents narrows the rows.
#[tokio::test]
async fn every_documented_site_filter_narrows_the_rows() {
    use crate::server::shared::handlers::query::narrowing_check::assert_every_param_narrows;
    use crate::server::sites::handlers::SiteListQuery;

    let scope = list_scope(&[Uuid::new_v4()], Uuid::new_v4());
    let id = Uuid::new_v4().to_string();
    assert_every_param_narrows(
        |query: &SiteListQuery| query.narrow(&scope).expect("filter builds"),
        &[
            ("credential_ids", id.as_str()),
            ("effective_stale_after_hours", "24"),
            ("tag_ids", id.as_str()),
            ("tag_sets", id.as_str()),
            ("search", "porto"),
        ],
        &[
            "limit",
            "offset",
            "group_by",
            "order_by",
            "order_direction",
            "group_by_tag_group",
            "order_by_tag_group",
        ],
    )
    .await;
}

/// Tag groups list system groups first, and the "applies to" filter, which a computed scope keeps
/// out of SQL, narrows in memory with the total it reports.
#[tokio::test]
async fn tag_groups_list_system_first_and_filter_scope_in_memory() {
    use crate::server::shared::entities::EntityDiscriminants;
    use crate::server::tags::groups::base::{TagGroup, TagGroupBase, TagGroupType};
    use crate::server::tags::groups::handlers::TagGroupFilterQuery;

    let (storage, state, _container) = test_state().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();

    let custom = TagGroup::new(TagGroupBase {
        entity_types: Some(vec![EntityDiscriminants::Host]),
        ..TagGroupBase::new(org.id, "Backup schedule".to_string(), TagGroupType::Custom)
    });
    let everywhere = TagGroup::new(TagGroupBase::new(
        org.id,
        "Owner".to_string(),
        TagGroupType::Custom,
    ));
    let status = TagGroup::new(TagGroupBase::new(
        org.id,
        "Status".to_string(),
        TagGroupType::Status,
    ));
    for group in [&custom, &everywhere, &status] {
        storage.tag_groups.create(group).await.unwrap();
    }
    let scope = list_scope(&[], org.id);

    let all = list_page::<TagGroup>(&state, &scope, &TagGroupFilterQuery::default())
        .await
        .unwrap();
    assert_eq!(
        all.data.first().map(|g| g.id),
        Some(status.id),
        "system groups first"
    );

    let hosts = list_page::<TagGroup>(
        &state,
        &scope,
        &TagGroupFilterQuery {
            entity_types: Some(vec![EntityDiscriminants::Host]),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(hosts.data.iter().any(|g| g.id == custom.id));
    assert!(hosts.data.iter().all(|g| g.id != everywhere.id));
    assert_eq!(hosts.meta.pagination.total_count, hosts.data.len() as u64);
}

/// Every filter the tag groups list documents narrows the rows. The entity types a group applies
/// to can be computed in code, so those narrow in the list rather than in this filter.
#[tokio::test]
async fn every_documented_tag_group_filter_narrows_the_rows() {
    use crate::server::shared::handlers::query::narrowing_check::assert_every_param_narrows;
    use crate::server::tags::groups::handlers::TagGroupFilterQuery;

    let scope = list_scope(&[], Uuid::new_v4());
    let id = Uuid::new_v4().to_string();
    assert_every_param_narrows(
        |query: &TagGroupFilterQuery| query.narrow(&scope).expect("filter builds"),
        &[
            ("system", "true"),
            ("ordered", "true"),
            ("tag_ids", id.as_str()),
            ("search", "backup"),
        ],
        &[
            "entity_types",
            "include_every_entity_type",
            "limit",
            "offset",
            "group_by",
            "order_by",
            "order_direction",
        ],
    )
    .await;
}

/// Tags grouped by tag group come in the order the tag groups list uses, system groups first,
/// each group keyed as the UI builds it, with tags in no group last.
#[tokio::test]
async fn tags_group_by_tag_group_in_the_groups_list_order() {
    use crate::server::tags::groups::base::{TagGroup, TagGroupBase, TagGroupType};
    use crate::server::tags::handlers::{TagFilterQuery, TagOrderField};
    use crate::server::tags::r#impl::base::{Tag, TagBase};

    let (storage, state, _container) = test_state().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let custom = TagGroup::new(TagGroupBase::new(
        org.id,
        "Backup".to_string(),
        TagGroupType::Custom,
    ));
    let status = TagGroup::new(TagGroupBase::new(
        org.id,
        "Status".to_string(),
        TagGroupType::Status,
    ));
    storage.tag_groups.create(&custom).await.unwrap();
    storage.tag_groups.create(&status).await.unwrap();
    for (name, group) in [
        ("nightly", Some(custom.id)),
        ("active", Some(status.id)),
        ("loose", None),
    ] {
        let tag = Tag::new(TagBase {
            name: name.to_string(),
            organization_id: org.id,
            tag_group_id: group,
            ..Default::default()
        });
        storage.tags.create(&tag).await.unwrap();
    }

    let page = list_page::<Tag>(
        &state,
        &list_scope(&[], org.id),
        &TagFilterQuery {
            group_by: Some(TagOrderField::TagGroup),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let names: Vec<&str> = page.data.iter().map(|t| t.base.name.as_str()).collect();
    assert_eq!(names, vec!["active", "nightly", "loose"]);
    assert_eq!(
        counts(page.meta.pagination.group_counts.as_deref()),
        vec![
            (format!("0:Status:status:{}", status.id), 1),
            (format!("1:Custom:backup:{}", custom.id), 1),
            (String::new(), 1)
        ]
    );
}

/// Every filter the tags list documents narrows the rows.
#[tokio::test]
async fn every_documented_tag_filter_narrows_the_rows() {
    use crate::server::shared::handlers::query::narrowing_check::assert_every_param_narrows;
    use crate::server::tags::handlers::TagFilterQuery;

    let scope = list_scope(&[], Uuid::new_v4());
    let id = Uuid::new_v4().to_string();
    assert_every_param_narrows(
        |query: &TagFilterQuery| query.narrow(&scope).expect("filter builds"),
        &[
            ("colors", "blue"),
            ("tag_group_ids", id.as_str()),
            ("icons", "server"),
            ("search", "web"),
            ("at", "2026-01-01T00:00:00Z"),
        ],
        &["limit", "offset", "group_by", "order_by", "order_direction"],
    )
    .await;
}

/// The site filter reads each user's stored assignments, and the page carries them for the Sites
/// column.
#[tokio::test]
async fn members_filter_by_the_sites_assigned_to_them() {
    use crate::server::auth::access::Assignment;
    use crate::server::auth::middleware::auth::AuthenticatedEntity;
    use crate::server::site_assignments::r#impl::base::Principal;
    use crate::server::users::r#impl::permissions::UserOrgPermissions;
    use crate::server::users::members::{Member, MemberFilterQuery, members_page};

    let (storage, state, _container) = test_state().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let (office, lab) = (site(&org.id), site(&org.id));
    storage.sites.create(&office).await.unwrap();
    storage.sites.create(&lab).await.unwrap();

    let role = UserOrgPermissions::Member;
    let mut assigned = Vec::new();
    for (email, site_id) in [
        ("office@porto.example", office.id),
        ("lab@porto.example", lab.id),
    ] {
        let mut u = user(&org.id);
        u.base.email = email_address::EmailAddress::new_unchecked(email);
        u.base.permissions = role;
        storage.users.create(&u).await.unwrap();
        state
            .services
            .access_service
            .set_access(
                Principal::User(u.id),
                role,
                &Assignment::for_person(role, &[site_id]),
                AuthenticatedEntity::System,
            )
            .await
            .unwrap();
        assigned.push(u);
    }

    let page = members_page(
        &state,
        org.id,
        Uuid::new_v4(),
        &UserOrgPermissions::Owner.grants(),
        &MemberFilterQuery {
            site_ids: Some(vec![office.id]),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let [Member::User { id, data, .. }] = page.data.as_slice() else {
        panic!("one member is on the office site, got {}", page.data.len());
    };
    assert_eq!(*id, assigned[0].id);
    assert_eq!(data.base.site_ids, vec![office.id]);
}

/// The members list holds the users and invites the caller may see, filters and groups on what
/// the server derives, and the user CSV export holds no more of the organization than it does.
#[tokio::test]
async fn members_list_users_and_invites_the_caller_may_see() {
    use crate::server::invites::r#impl::base::Invite;
    use crate::server::shared::handlers::traits::{CrudHandlers, ListScope};
    use crate::server::shared::services::traits::CrudService;
    use crate::server::users::r#impl::base::User;
    use crate::server::users::r#impl::permissions::UserOrgPermissions;
    use crate::server::users::members::{
        Member, MemberFilterQuery, MemberOrderField, MemberStatus, members_page,
    };

    let (storage, state, _container) = test_state().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let person = |email: &str, role: UserOrgPermissions| {
        let mut u = user(&org.id);
        u.base.email = email_address::EmailAddress::new_unchecked(email);
        u.base.permissions = role;
        u
    };
    let admin = person("admin@porto.example", UserOrgPermissions::Admin);
    let peer = person("peer@porto.example", UserOrgPermissions::Admin);
    let member = person("member@porto.example", UserOrgPermissions::Member);
    for u in [&admin, &peer, &member] {
        storage.users.create(u).await.unwrap();
    }
    let invite = Invite::with_expiration(
        org.id,
        "https://example.test/invite".to_string(),
        admin.id,
        24,
        UserOrgPermissions::Member,
        vec![],
        None,
    );
    storage.invites.create(&invite).await.unwrap();

    let ids = |page: &[Member]| {
        let mut ids: Vec<Uuid> = page
            .iter()
            .map(|m| match m {
                Member::User { id, .. } | Member::Invite { id, .. } => *id,
            })
            .collect();
        ids.sort();
        ids
    };
    let all = members_page(
        &state,
        org.id,
        admin.id,
        &UserOrgPermissions::Admin.grants(),
        &MemberFilterQuery::default(),
    )
    .await
    .unwrap();
    let mut expected = vec![admin.id, member.id, invite.id];
    expected.sort();
    assert_eq!(ids(&all.data), expected, "a peer admin is not listed");

    let pending = members_page(
        &state,
        org.id,
        admin.id,
        &UserOrgPermissions::Admin.grants(),
        &MemberFilterQuery {
            statuses: Some(vec![MemberStatus::Pending]),
            group_by: Some(MemberOrderField::InviteStatus),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(ids(&pending.data), vec![invite.id]);
    assert_eq!(
        counts(pending.meta.pagination.group_counts.as_deref()),
        vec![("pending".to_string(), 1)]
    );

    // The export reads through `User::access_filter`, the same rule.
    let scope = ListScope {
        user_id: Some(admin.id),
        organization_grants: Some(UserOrgPermissions::Admin.grants()),
        ..list_scope(&[], org.id)
    };
    let mut exported: Vec<Uuid> = state
        .services
        .user_service
        .get_all(
            User::list_filter(&state, &Default::default(), &scope)
                .await
                .unwrap(),
        )
        .await
        .unwrap()
        .into_iter()
        .map(|u| u.id)
        .collect();
    exported.sort();
    let mut expected = vec![admin.id, member.id];
    expected.sort();
    assert_eq!(exported, expected);
}

/// Subnets grouped by range nest each range under the ranges containing it, through the generic
/// tree path: a page cut through a tree keeps every group's full count, each row carries its depth,
/// and ranges nested in nothing fall in the ungrouped bucket after every tree.
#[tokio::test]
async fn subnets_group_as_a_nesting_tree_across_pages() {
    use crate::server::subnets::handlers::{SubnetFilterQuery, SubnetOrderField};
    use crate::server::subnets::r#impl::base::{Subnet, SubnetCidr, SubnetCidrValue, SubnetName};

    let (storage, state, _container) = test_state().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let net = site(&org.id);
    storage.sites.create(&net).await.unwrap();

    let range = |cidr: &str| {
        let mut s = subnet(&net.id);
        s.base.name = SubnetName::unattributed(cidr.to_string());
        s.base.cidr = SubnetCidr::new(
            SubnetCidrValue(cidr.parse().unwrap()),
            AttributeSource::Manual,
        );
        s
    };
    let wide = range("10.0.0.0/16");
    let mid = range("10.0.16.0/20");
    let leaf = range("10.0.16.0/24");
    let lone = range("192.168.9.0/24");
    for s in [&leaf, &lone, &mid, &wide] {
        storage.subnets.create(s).await.unwrap();
    }
    let scope = list_scope(&[net.id], org.id);
    let query = |offset: u32| SubnetFilterQuery {
        group_by: Some(SubnetOrderField::Cidr),
        limit: Some(2),
        offset: Some(offset),
        ..Default::default()
    };

    let first = list_page::<Subnet>(&state, &scope, &query(0))
        .await
        .unwrap();
    let second = list_page::<Subnet>(&state, &scope, &query(2))
        .await
        .unwrap();
    let rows: Vec<(String, u32)> = first
        .data
        .iter()
        .chain(&second.data)
        .map(|s| (s.subnet.base.name().to_string(), s.nesting_depth))
        .collect();
    assert_eq!(
        rows,
        vec![
            ("10.0.0.0/16".to_string(), 0),
            ("10.0.16.0/20".to_string(), 1),
            ("10.0.16.0/24".to_string(), 2),
            ("192.168.9.0/24".to_string(), 0),
        ],
        "each parent directly before the ranges inside it, then a range in no tree"
    );
    // Each row's root is the key the UI matches it to its group by.
    assert_eq!(first.data[0].nesting_root_id, Some(wide.id));
    assert_eq!(second.data[1].nesting_root_id, None);
    let expected = vec![(wide.id.to_string(), 3), (String::new(), 1)];
    assert_eq!(
        counts(first.meta.pagination.group_counts.as_deref()),
        expected
    );
    assert_eq!(
        counts(second.meta.pagination.group_counts.as_deref()),
        expected
    );
}

/// Sorting by a computed field keeps the tree grouping: the trees, not one group per range, and
/// each tree where it falls in the sort rather than in its root's id order.
#[tokio::test]
async fn subnets_stay_grouped_as_trees_under_a_computed_sort() {
    use crate::server::shared::attribution::AttributeSource;
    use crate::server::shared::handlers::query::OrderDirection;
    use crate::server::subnets::handlers::{SubnetFilterQuery, SubnetOrderField};
    use crate::server::subnets::r#impl::base::{Subnet, SubnetCidr, SubnetCidrValue, SubnetName};

    let (storage, state, _container) = test_state().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let net = site(&org.id);
    storage.sites.create(&net).await.unwrap();

    let range = |cidr: &str, id: Option<Uuid>| {
        let mut s = subnet(&net.id);
        if let Some(id) = id {
            s.id = id;
        }
        s.base.name = SubnetName::unattributed(cidr.to_string());
        s.base.cidr = SubnetCidr::new(
            SubnetCidrValue(cidr.parse().unwrap()),
            AttributeSource::Manual,
        );
        s
    };
    // Root ids whose text order is the reverse of the address order.
    let first_root = range("10.0.0.0/16", Some(Uuid::from_u128(u128::MAX - 1)));
    let first_leaf = range("10.0.1.0/24", None);
    let second_root = range("10.5.0.0/16", Some(Uuid::from_u128(1)));
    let second_leaf = range("10.5.1.0/24", None);
    let lone_a = range("172.16.9.0/24", None);
    let lone_b = range("192.168.9.0/24", None);
    for s in [
        &first_root,
        &first_leaf,
        &second_root,
        &second_leaf,
        &lone_a,
        &lone_b,
    ] {
        storage.subnets.create(s).await.unwrap();
    }
    let scope = list_scope(&[net.id], org.id);
    let query = |offset: u32| SubnetFilterQuery {
        group_by: Some(SubnetOrderField::Cidr),
        order_by: Some(SubnetOrderField::Utilization),
        order_direction: Some(OrderDirection::Desc),
        limit: Some(2),
        offset: Some(offset),
        ..Default::default()
    };

    let mut rows = Vec::new();
    for offset in [0, 2, 4] {
        let page = list_page::<Subnet>(&state, &scope, &query(offset))
            .await
            .unwrap();
        assert_eq!(
            counts(page.meta.pagination.group_counts.as_deref()),
            vec![
                (first_root.id.to_string(), 2),
                (second_root.id.to_string(), 2),
                (String::new(), 2)
            ],
            "one group per tree, in address order, then one group of ranges in no tree"
        );
        rows.extend(
            page.data
                .into_iter()
                .map(|s| (s.subnet.base.name().to_string(), s.nesting_root_id)),
        );
    }
    assert_eq!(
        rows,
        vec![
            ("10.0.0.0/16".to_string(), Some(first_root.id)),
            ("10.0.1.0/24".to_string(), Some(first_root.id)),
            ("10.5.0.0/16".to_string(), Some(second_root.id)),
            ("10.5.1.0/24".to_string(), Some(second_root.id)),
            ("172.16.9.0/24".to_string(), None),
            ("192.168.9.0/24".to_string(), None),
        ],
        "every row sits in the group its root names, parents before the ranges inside them"
    );
}

/// Every filter the subnets list documents narrows the rows.
#[tokio::test]
async fn every_documented_subnet_filter_narrows_the_rows() {
    use crate::server::shared::handlers::query::narrowing_check::assert_every_param_narrows;
    use crate::server::subnets::handlers::SubnetFilterQuery;

    let site = Uuid::new_v4();
    let scope = list_scope(&[site, Uuid::new_v4()], Uuid::new_v4());
    let id = Uuid::new_v4().to_string();
    let site_param = site.to_string();
    assert_every_param_narrows(
        |query: &SubnetFilterQuery| query.narrow(&scope).expect("filter builds"),
        &[
            ("ids", id.as_str()),
            ("site_ids", site_param.as_str()),
            ("subnet_types", "Lan"),
            ("sources", "Discovery"),
            ("managed_by", id.as_str()),
            ("user_managed", "true"),
            ("tag_ids", id.as_str()),
            ("tag_sets", id.as_str()),
            ("search", "lab"),
            ("at", "2026-01-01T00:00:00Z"),
        ],
        &[
            "limit",
            "offset",
            "group_by",
            "order_by",
            "order_direction",
            "group_by_tag_group",
            "order_by_tag_group",
            // Widens `user_managed` by the seeded Internet and Remote ranges, never narrows.
            "include_shared_ranges",
            // Applied after `narrow`, from each site's cutoff.
            "stale",
        ],
    )
    .await;
}

/// The list shows one Internet and one Remote row, from the lowest-id site among those listed,
/// whatever number of sites hold a copy. Loopback rows stay out, and the type filter's options
/// count the rows the list shows.
#[tokio::test]
async fn subnets_list_each_seeded_shared_range_once() {
    use crate::server::shared::handlers::ordering::OrderField;
    use crate::server::shared::handlers::traits::field_values_filter;
    use crate::server::shared::services::traits::CrudService;
    use crate::server::shared::storage::seed_data::{create_remote_subnet, create_wan_subnet};
    use crate::server::subnets::handlers::{SubnetFilterQuery, SubnetOrderField};
    use crate::server::subnets::r#impl::base::{Subnet, SubnetTypeAttributed};
    use crate::server::subnets::r#impl::types::SubnetType;

    let (storage, state, _container) = test_state().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let mut sites = Vec::new();
    for _ in 0..3 {
        let s = site(&org.id);
        storage.sites.create(&s).await.unwrap();
        storage
            .subnets
            .create(&create_wan_subnet(s.id))
            .await
            .unwrap();
        storage
            .subnets
            .create(&create_remote_subnet(s.id))
            .await
            .unwrap();
        let mut loopback = subnet(&s.id);
        loopback.base.subnet_type = SubnetTypeAttributed::unattributed(SubnetType::Loopback);
        storage.subnets.create(&loopback).await.unwrap();
        storage.subnets.create(&subnet(&s.id)).await.unwrap();
        sites.push(s.id);
    }
    let site_ids: Vec<Uuid> = sites.clone();
    let lowest = *site_ids.iter().min().unwrap();
    let highest = *site_ids.iter().max().unwrap();
    let scope = list_scope(&site_ids, org.id);

    let listed = |query: SubnetFilterQuery| {
        let state = &state;
        let scope = &scope;
        async move {
            list_page::<Subnet>(state, scope, &query)
                .await
                .unwrap()
                .data
                .into_iter()
                .map(|s| (s.subnet.base.subnet_type(), s.subnet.base.site_id))
                .collect::<Vec<_>>()
        }
    };
    let shared = |query: SubnetFilterQuery| SubnetFilterQuery {
        user_managed: Some(true),
        include_shared_ranges: Some(true),
        ..query
    };
    let of_type = |rows: &[(SubnetType, Uuid)], t: SubnetType| {
        rows.iter()
            .filter(|(row_type, _)| *row_type == t)
            .map(|(_, site_id)| *site_id)
            .collect::<Vec<_>>()
    };

    let rows = listed(shared(SubnetFilterQuery::default())).await;
    assert_eq!(of_type(&rows, SubnetType::Internet), vec![lowest]);
    assert_eq!(of_type(&rows, SubnetType::Remote), vec![lowest]);
    assert!(of_type(&rows, SubnetType::Loopback).is_empty());
    assert_eq!(of_type(&rows, SubnetType::Lan).len(), 3);

    let on_highest = listed(shared(SubnetFilterQuery {
        site_ids: Some(vec![highest]),
        ..Default::default()
    }))
    .await;
    assert_eq!(
        of_type(&on_highest, SubnetType::Internet),
        vec![highest],
        "a site filter shows that site's copy"
    );
    assert_eq!(of_type(&on_highest, SubnetType::Lan), vec![highest]);

    let user_managed_only = listed(SubnetFilterQuery {
        user_managed: Some(true),
        ..Default::default()
    })
    .await;
    assert!(of_type(&user_managed_only, SubnetType::Internet).is_empty());
    assert!(of_type(&user_managed_only, SubnetType::Remote).is_empty());

    let field = SubnetOrderField::SubnetType;
    let type_counts = state
        .services
        .subnet_service
        .count_by_group(
            field_values_filter::<Subnet>(&scope, &shared(SubnetFilterQuery::default()), field),
            field.to_sql(),
        )
        .await
        .unwrap();
    let count_of = |t: &str| {
        type_counts
            .iter()
            .find(|g| g.value.as_deref() == Some(t))
            .map(|g| g.count)
    };
    assert_eq!(count_of("Internet"), Some(1));
    assert_eq!(count_of("Remote"), Some(1));
    assert_eq!(count_of("Loopback"), None);
}

/// Sorting by site lists each site's rows together, in site-name order, not site-id order, with the
/// shared Internet and Remote rows first.
#[tokio::test]
async fn subnets_sorted_by_site_follow_the_site_name() {
    use crate::server::shared::handlers::query::OrderDirection;
    use crate::server::shared::storage::seed_data::{create_remote_subnet, create_wan_subnet};
    use crate::server::subnets::handlers::{SubnetFilterQuery, SubnetOrderField};
    use crate::server::subnets::r#impl::base::{Subnet, SubnetCidr, SubnetCidrValue};

    let (storage, state, _container) = test_state().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    // Ids in the reverse of the name order.
    let mut alpha = site(&org.id);
    alpha.id = Uuid::from_u128(u128::MAX - 1);
    alpha.base.name = "Alpha".to_string();
    let mut zulu = site(&org.id);
    zulu.id = Uuid::from_u128(2);
    zulu.base.name = "Zulu".to_string();
    for s in [&alpha, &zulu] {
        storage.sites.create(s).await.unwrap();
    }
    for (site_id, cidr) in [
        (zulu.id, "10.1.0.0/24"),
        (alpha.id, "10.2.0.0/24"),
        (zulu.id, "10.3.0.0/24"),
        (alpha.id, "10.4.0.0/24"),
    ] {
        let mut s = subnet(&site_id);
        s.base.cidr = SubnetCidr::new(
            SubnetCidrValue(cidr.parse().unwrap()),
            AttributeSource::Manual,
        );
        storage.subnets.create(&s).await.unwrap();
    }
    for s in [&alpha, &zulu] {
        storage
            .subnets
            .create(&create_wan_subnet(s.id))
            .await
            .unwrap();
        storage
            .subnets
            .create(&create_remote_subnet(s.id))
            .await
            .unwrap();
    }
    let scope = list_scope(&[alpha.id, zulu.id], org.id);

    // The shared rows stand for every site, so they lead in both directions.
    let (shared, a, z) = ("all sites", "Alpha", "Zulu");
    for (direction, expected) in [
        (OrderDirection::Asc, [shared, shared, a, a, z, z]),
        (OrderDirection::Desc, [shared, shared, z, z, a, a]),
    ] {
        for group_by in [None, Some(SubnetOrderField::Cidr)] {
            let query = SubnetFilterQuery {
                user_managed: Some(true),
                include_shared_ranges: Some(true),
                group_by,
                order_by: Some(SubnetOrderField::SiteId),
                order_direction: Some(direction),
                ..Default::default()
            };
            let rows: Vec<&str> = list_page::<Subnet>(&state, &scope, &query)
                .await
                .unwrap()
                .data
                .into_iter()
                .map(|s| match s.subnet.base.site_id {
                    _ if !s.subnet.is_user_managed() => shared,
                    id if id == alpha.id => a,
                    _ => z,
                })
                .collect();
            assert_eq!(rows, expected, "{direction:?}, grouped by {group_by:?}");
        }
    }
}
