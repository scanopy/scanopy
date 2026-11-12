use super::*;
use crate::server::shared::entities::EntityDiscriminants;
use std::collections::{HashMap, HashSet};

#[test]
fn subnet_vlan_records_are_derived_and_reference_valid_entities() {
    let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4(), UseCase::default());

    // The derivation should link at least the subnets whose hosts carry a
    // native VLAN (otherwise the junction is silently empty and demo
    // subnets show no VLANs).
    assert!(
        !demo.subnet_vlan_records.is_empty(),
        "expected derived subnet↔vlan links"
    );

    let subnet_ids: HashSet<Uuid> = demo.subnets.iter().map(|s| s.id).collect();
    let vlan_ids: HashSet<Uuid> = demo.vlans.iter().map(|v| v.id).collect();
    let mut pairs: HashSet<(Uuid, Uuid)> = HashSet::new();
    for r in &demo.subnet_vlan_records {
        assert!(
            subnet_ids.contains(&r.base.subnet_id),
            "subnet_vlan references unknown subnet"
        );
        assert!(
            vlan_ids.contains(&r.base.vlan_id),
            "subnet_vlan references unknown vlan"
        );
        assert!(
            pairs.insert((r.base.subnet_id, r.base.vlan_id)),
            "duplicate subnet↔vlan link"
        );
    }
}

/// DC Storage's nodes are read over SNMP by nothing, so the demo's only link from that subnet to
/// a VLAN is the switch's access ports having learned their MACs. A demo without it would show
/// no example of that link.
#[test]
fn a_subnet_is_linked_to_a_vlan_through_the_macs_its_access_ports_learned() {
    let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4(), UseCase::default());
    let storage = demo
        .subnets
        .iter()
        .find(|s| s.base.name() == "DC Storage")
        .expect("the DC Storage subnet");
    let servers: HashSet<Uuid> = demo
        .vlans
        .iter()
        .filter(|v| v.base.vlan_number == 20 && v.base.site_id == storage.base.site_id)
        .map(|v| v.id)
        .collect();

    assert!(
        demo.subnet_vlan_records
            .iter()
            .any(|r| r.base.subnet_id == storage.id && servers.contains(&r.base.vlan_id)),
        "DC Storage should be linked to the Servers VLAN"
    );
}

#[test]
fn a_subnet_carries_a_provisional_cidr_source() {
    use crate::server::shared::attribution::AttributeMethod;

    let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4(), UseCase::default());
    assert!(
        demo.subnets
            .iter()
            .any(|s| s.base.cidr.source().method() == AttributeMethod::Inferred),
        "expected at least one subnet whose cidr_source is Inferred-tier (a provisional range)"
    );
}

/// Some demo hosts read as stale from the moment the org is created, and each one reads as
/// stale as a whole: its addresses, ports, services and bindings go stale with it, and no
/// child of a current host does.
#[test]
fn stale_demo_hosts_are_stale_together_with_their_children() {
    use crate::server::shared::storage::snapshot::DiscoveryTracked;
    use crate::server::shared::types::entities::EntityFreshness;

    let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4(), UseCase::default());
    let now = crate::server::shared::clock::now();
    let cutoffs: HashMap<Uuid, DateTime<Utc>> = demo
        .sites
        .iter()
        .map(|n| (n.id, n.stale_cutoff(now)))
        .collect();

    let mut stale_hosts = 0;
    for hws in demo
        .hosts_with_services
        .iter()
        .chain(&demo.recent_hosts_with_services)
    {
        let cutoff = cutoffs[&hws.host.base.site_id];
        let host_freshness = hws.host.freshness(cutoff);
        if host_freshness == EntityFreshness::Stale {
            stale_hosts += 1;
        }
        let name = format!("{:?}", hws.host.base.name);

        let mut children: Vec<(EntityFreshness, DateTime<Utc>, DateTime<Utc>)> = Vec::new();
        children.extend(
            hws.ip_addresses
                .iter()
                .map(|e| (e.freshness(cutoff), e.created_at, e.last_seen_at)),
        );
        children.extend(
            hws.ports
                .iter()
                .map(|e| (e.freshness(cutoff), e.created_at, e.last_seen_at)),
        );
        for svc in &hws.services {
            children.push((svc.freshness(cutoff), svc.created_at, svc.last_seen_at));
            children.extend(
                svc.base
                    .bindings
                    .iter()
                    .map(|e| (e.freshness(cutoff), e.created_at, e.last_seen_at)),
            );
        }

        assert!(
            hws.host.created_at <= hws.host.last_seen_at,
            "{name}: last seen before it was created"
        );
        for (freshness, created_at, last_seen_at) in children {
            assert_eq!(
                freshness, host_freshness,
                "{name}: a child's freshness differs from its host's"
            );
            assert!(
                created_at <= last_seen_at,
                "{name}: a child was last seen before it was created"
            );
        }
    }
    assert!(stale_hosts > 0, "expected at least one stale demo host");
}

#[test]
fn a_host_is_known_only_by_inference() {
    let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4(), UseCase::default());
    assert!(
        demo.hosts_with_services
            .iter()
            .any(|hws| matches!(hws.host.base.source, EntitySource::Inferred { .. })),
        "expected at least one host with EntitySource::Inferred (never contacted directly)"
    );
}

#[test]
fn a_discovery_carries_warnings_from_more_than_one_remedy_group() {
    use crate::daemon::discovery::types::warnings::DiscoveryWarning;
    use crate::server::discovery::r#impl::types::RunType;
    use crate::server::shared::types::metadata::TypeMetadataProvider;

    let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4(), UseCase::default());
    let warnings: Vec<&DiscoveryWarning> = demo
        .discoveries
        .iter()
        .filter_map(|d| match &d.base.run_type {
            RunType::Historical { results } => Some(&results.warnings),
            _ => None,
        })
        .flatten()
        .collect();
    assert!(
        !warnings.is_empty(),
        "expected at least one seeded discovery warning"
    );

    let remedy_groups: HashSet<&'static str> =
        warnings.iter().map(|w| w.code().category()).collect();
    assert!(
        remedy_groups.len() > 1,
        "expected seeded warnings to span more than one WarningRemedy group, got {remedy_groups:?}"
    );
}

#[test]
fn a_service_is_in_the_industrial_category() {
    use crate::server::services::r#impl::categories::ServiceCategory;

    let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4(), UseCase::default());
    assert!(
        demo.hosts_with_services
            .iter()
            .flat_map(|hws| &hws.services)
            .any(|svc| svc.base.service_definition.category() == ServiceCategory::Industrial),
        "expected at least one seeded service in the Industrial category"
    );
}

/// The host editor explains each host's title by the rung that produced it, so the demo has a
/// host titled from each: a named host, and nameless ones titled by their hostname, sysName,
/// chassis ID and address.
#[test]
fn demo_hosts_are_titled_from_every_rung() {
    use crate::server::hosts::r#impl::name_ladder::HostNameRung;

    let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4(), UseCase::default());
    let rungs: HashSet<HostNameRung> = demo
        .hosts_with_services
        .iter()
        .filter_map(|hws| hws.host.resolved_name(&hws.ip_addresses))
        .map(|(_, rung)| rung)
        .collect();

    for rung in [
        HostNameRung::Name,
        HostNameRung::Hostname,
        HostNameRung::SysName,
        HostNameRung::ChassisId,
        HostNameRung::Address,
    ] {
        assert!(rungs.contains(&rung), "no demo host is titled by {rung:?}");
    }
}

#[test]
fn a_host_shows_distinct_firmware_and_software_revisions() {
    let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4(), UseCase::default());
    assert!(
        demo.hosts_with_services.iter().any(|hws| {
            match (
                &hws.host.base.firmware_revision,
                &hws.host.base.software_revision,
            ) {
                (Some(firmware), Some(software)) => firmware.value().0 != software.value().0,
                _ => false,
            }
        }),
        "expected at least one host with distinct, populated firmware and software revisions"
    );
}

#[test]
fn every_credential_type_is_in_the_demo() {
    use crate::server::credentials::r#impl::types::CredentialTypeDiscriminants;
    use strum::{IntoDiscriminant, IntoEnumIterator};

    let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4(), UseCase::default());
    let present: HashSet<CredentialTypeDiscriminants> = demo
        .credentials
        .iter()
        .map(|c| c.base.credential_type.discriminant())
        .collect();

    for credential_type in CredentialTypeDiscriminants::iter() {
        assert!(
            present.contains(&credential_type),
            "no demo credential of type {credential_type}"
        );
    }
}

/// The seed bulk-inserts credentials, skipping the API's validation, so the demo has to hold
/// itself to it.
#[test]
fn demo_credentials_are_valid_and_described() {
    use validator::Validate;

    let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4(), UseCase::default());
    for credential in &demo.credentials {
        let name = &credential.base.name;
        credential
            .base
            .validate()
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        credential
            .base
            .validate_settings()
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(
            credential
                .base
                .description
                .as_deref()
                .is_some_and(|d| !d.trim().is_empty()),
            "{name} has no description"
        );
    }
}

/// Every assignment uses a scope its credential type supports (the seed skips the API's
/// target check too), and every credential is assigned somewhere.
#[test]
fn demo_credentials_are_assigned_within_their_targets() {
    use crate::server::credentials::r#impl::types::Target;
    use crate::server::credentials::service::find_targetless;

    let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4(), UseCase::default());
    let by_id: HashMap<Uuid, &Credential> = demo.credentials.iter().map(|c| (c.id, c)).collect();

    let mut uses: Vec<(Uuid, Target)> = Vec::new();
    for hws in &demo.hosts_with_services {
        for assignment in &hws.host.base.credential_assignments {
            uses.push((assignment.credential_id, Target::Hosts));
        }
    }
    for assignment in &demo.site_credential_assignments {
        for &credential_id in &assignment.credential_ids {
            uses.push((credential_id, Target::Site));
        }
    }
    for discovery in &demo.discoveries {
        for target in &discovery.integration_targets {
            uses.push((target.credential_id(), Target::from(target)));
        }
    }

    for (credential_id, scope) in &uses {
        let credential = by_id
            .get(credential_id)
            .expect("assignment names a credential the demo does not define");
        assert!(
            credential.base.credential_type.targets().contains(scope),
            "{} is assigned with scope {scope:?}, which its type does not support",
            credential.base.name
        );
    }

    // Every credential has a target, so Home lists no TargetlessCredential issue. The Podman
    // socket keeps none (both demo daemons run Docker), and as a seeded default it is not
    // reported.
    let used: HashSet<Uuid> = uses.iter().map(|(id, _)| *id).collect();
    let targetless = find_targetless(demo.credentials.clone(), &used, &HashSet::new());
    assert!(
        targetless.is_empty(),
        "demo credentials with no target: {:?}",
        targetless.iter().map(|c| &c.base.name).collect::<Vec<_>>()
    );
}

#[test]
fn daemons_report_the_subnets_their_host_has_addresses_on() {
    let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4(), UseCase::default());
    assert!(!demo.daemons.is_empty());

    let subnets_by_id: HashMap<Uuid, &Subnet> = demo.subnets.iter().map(|s| (s.id, s)).collect();
    let reported: HashMap<Uuid, &Vec<Uuid>> = demo
        .daemon_interfaced_subnets
        .iter()
        .map(|(daemon_id, subnet_ids)| (*daemon_id, subnet_ids))
        .collect();

    for daemon in &demo.daemons {
        let host_subnets: HashSet<Uuid> = demo
            .hosts_with_services
            .iter()
            .filter(|hws| hws.host.id == daemon.base.host_id)
            .flat_map(|hws| hws.ip_addresses.iter().map(|ip| ip.base.subnet_id))
            .collect();
        let subnet_ids = reported
            .get(&daemon.id)
            .unwrap_or_else(|| panic!("{} reports no subnets", daemon.base.name));

        assert_eq!(
            subnet_ids.iter().copied().collect::<HashSet<_>>(),
            host_subnets,
            "{} should report exactly its host's subnets",
            daemon.base.name
        );
        assert_eq!(subnet_ids.len(), host_subnets.len(), "duplicate subnet ids");
        // The daemon hosts run Docker, so a real heartbeat includes the bridge.
        assert!(
            subnet_ids
                .iter()
                .any(|id| subnets_by_id[id].base.subnet_type().is_container_bridge()),
            "{} reports no container bridge subnet",
            daemon.base.name
        );
    }
}

/// The demo's tag assignments keep the rule tag groups exist for: no entity holds two tags of
/// one group. Seeding writes the junction rows directly, so nothing else would catch a demo host
/// tagged both Production and Development.
#[test]
fn no_demo_entity_holds_two_tags_of_one_tag_group() {
    let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4(), UseCase::default());
    let group_of: HashMap<Uuid, Uuid> = demo
        .tags
        .iter()
        .filter_map(|t| t.base.tag_group_id.map(|g| (t.id, g)))
        .collect();

    let tag_lists = demo_tag_lists(&demo)
        .into_iter()
        .map(|(id, _, tags)| (id, tags));

    let name_of = |tag_id: &Uuid| {
        demo.tags
            .iter()
            .find(|t| t.id == *tag_id)
            .map(|t| t.base.name.clone())
    };
    for (id, tags) in tag_lists {
        let mut seen = HashSet::new();
        for group in tags.iter().filter_map(|id| group_of.get(id)) {
            assert!(
                seen.insert(group),
                "entity {id} holds two tags of the {group} group: {:?}",
                tags.iter().filter_map(name_of).collect::<Vec<_>>()
            );
        }
    }
}

/// The Application view draws a dependency between the application groups its members sit in.
/// A member with no Application tag lands in Ungrouped, and its edge runs into that box. Members
/// that name no service (the retired Zabbix in Monitoring Stack) draw nothing and are skipped.
#[test]
fn every_dependency_member_sits_in_an_application() {
    use crate::server::tags::groups::base::TagGroupType;
    use crate::server::topology::service::application_builder::ApplicationBuilder;

    let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4(), UseCase::default());
    let application_group = demo
        .tag_groups
        .iter()
        .find(|g| g.base.group_type == TagGroupType::Application)
        .map(|g| g.id);
    let app_tags: HashMap<Uuid, &Tag> = demo
        .tags
        .iter()
        .filter(|t| t.in_group(application_group))
        .map(|t| (t.id, t))
        .collect();
    let hosts: Vec<Host> = demo
        .hosts_with_services
        .iter()
        .map(|h| h.host.clone())
        .collect();
    let services: Vec<&Service> = demo
        .hosts_with_services
        .iter()
        .flat_map(|h| &h.services)
        .collect();

    for dependency in &demo.dependencies {
        let members: Vec<&Service> = match &dependency.base.members {
            DependencyMembers::Services { service_ids } => service_ids
                .iter()
                .filter_map(|id| services.iter().copied().find(|s| s.id == *id))
                .collect(),
            DependencyMembers::Bindings { binding_ids } => binding_ids
                .iter()
                .filter_map(|id| {
                    services
                        .iter()
                        .copied()
                        .find(|s| s.base.bindings.iter().any(|b| b.id == *id))
                })
                .collect(),
        };
        for service in members {
            assert!(
                ApplicationBuilder::find_app_tag(service, &hosts, &app_tags).is_some(),
                "dependency {} has member {}, which is in no application",
                dependency.base.name,
                service.base.name
            );
        }
    }
}

#[test]
fn every_daemon_api_key_is_bound_one_to_one() {
    let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4(), UseCase::default());
    assert!(!demo.api_keys.is_empty());

    // No site-shared legacy keys: each key names a daemon, and that daemon names it back.
    for key in &demo.api_keys {
        let daemon_id = key
            .base
            .daemon_id
            .unwrap_or_else(|| panic!("{} is not bound to a daemon", key.base.name));
        let daemon = demo
            .daemons
            .iter()
            .find(|d| d.id == daemon_id)
            .unwrap_or_else(|| panic!("{} names a daemon the demo lacks", key.base.name));
        assert_eq!(daemon.base.api_key_id, Some(key.id), "{}", daemon.base.name);
        assert_eq!(daemon.base.site_id, key.base.site_id, "{}", key.base.name);
    }
    assert!(demo.daemons.iter().all(|d| d.base.api_key_id.is_some()));
}

/// Every tagged demo entity: its id, its type, and its tags.
fn demo_tag_lists(demo: &DemoData) -> Vec<(Uuid, EntityDiscriminants, Vec<Uuid>)> {
    let hosts: Vec<&HostWithServices> = demo
        .hosts_with_services
        .iter()
        .chain(&demo.recent_hosts_with_services)
        .collect();
    hosts
        .iter()
        .map(|h| {
            (
                h.host.id,
                EntityDiscriminants::Host,
                h.host.base.tags.clone(),
            )
        })
        .chain(hosts.iter().flat_map(|h| {
            h.services
                .iter()
                .map(|s| (s.id, EntityDiscriminants::Service, s.base.tags.clone()))
        }))
        .chain(
            demo.subnets
                .iter()
                .map(|s| (s.id, EntityDiscriminants::Subnet, s.base.tags.clone())),
        )
        .chain(
            demo.sites
                .iter()
                .map(|s| (s.id, EntityDiscriminants::Site, s.base.tags.clone())),
        )
        .collect()
}

/// Seeding writes the junction rows directly, past the check that keeps a limited group's tags
/// off other entity types, so the demo has to keep the rule itself.
#[test]
fn demo_tags_sit_only_on_types_their_group_allows() {
    let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4(), UseCase::default());
    assert!(
        demo.tag_groups
            .iter()
            .any(|g| g.base.entity_types.is_some()),
        "the demo shows a group limited to some entity types"
    );

    for (id, entity_type, tags) in demo_tag_lists(&demo) {
        for tag in demo.tags.iter().filter(|t| tags.contains(&t.id)) {
            let Some(group) = demo
                .tag_groups
                .iter()
                .find(|g| Some(g.id) == tag.base.tag_group_id)
            else {
                continue;
            };
            assert!(
                group.base.allows(entity_type),
                "{entity_type} {id} holds {} from the {} group",
                tag.base.name,
                group.base.name
            );
        }
    }
}

/// Positions within each ordered demo group run 1, 2, 3… with no gap or repeat, so tables and
/// pickers list them in the order written. An unordered group's tags carry none.
#[test]
fn demo_tag_positions_run_from_one_in_each_ordered_group() {
    let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4(), UseCase::default());
    for group in &demo.tag_groups {
        let in_group = demo
            .tags
            .iter()
            .filter(|t| t.base.tag_group_id == Some(group.id));
        if !group.base.ordered {
            assert!(in_group.clone().all(|t| t.base.position.is_none()));
            continue;
        }
        let mut positions: Vec<i32> = in_group
            .map(|t| {
                t.base
                    .position
                    .expect("a tag in an ordered group has a position")
            })
            .collect();
        positions.sort();
        let expected: Vec<i32> = (1..=positions.len() as i32).collect();
        assert_eq!(positions, expected, "{}", group.base.name);
    }
}

/// The L3 view draws a host once per subnet it has an address on and joins the copies, but only
/// for regular subnets: container-bridge addresses render inside their host instead. Each of the
/// host's addresses sits on one of its interfaces, so L2 and the host page show which NIC carries
/// which subnet.
#[test]
fn a_host_is_multi_homed_across_subnets() {
    let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4(), UseCase::default());
    let subnets_by_id: HashMap<Uuid, &Subnet> = demo.subnets.iter().map(|s| (s.id, s)).collect();

    let multi_homed = demo.hosts_with_services.iter().find(|hws| {
        let subnets: HashSet<Uuid> = hws
            .ip_addresses
            .iter()
            .map(|ip| ip.base.subnet_id)
            .filter(|id| !subnets_by_id[id].base.subnet_type().is_container_bridge())
            .collect();
        subnets.len() >= 2
    });
    let hws = multi_homed.expect("a demo host with addresses on two or more regular subnets");

    let attached: HashSet<Uuid> = demo
        .interfaces
        .iter()
        .filter(|i| i.base.host_id == hws.host.id)
        .filter_map(|i| i.base.ip_address_id)
        .collect();
    for ip in &hws.ip_addresses {
        assert!(
            attached.contains(&ip.id),
            "{} on {} is on none of its host's interfaces",
            ip.base.ip_address,
            hws.host.base.name
        );
    }
}

#[test]
fn no_two_demo_addresses_share_an_address_on_one_subnet() {
    let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4(), UseCase::default());
    let mut seen = HashSet::new();
    for ip in demo
        .hosts_with_services
        .iter()
        .flat_map(|hws| &hws.ip_addresses)
    {
        assert!(
            seen.insert((ip.base.subnet_id, ip.base.ip_address)),
            "{} appears twice on one subnet",
            ip.base.ip_address
        );
    }
}
