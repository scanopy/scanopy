//! Virtualization owner ordering for the demo's bulk writes: which hosts and services go first so
//! no row names a service that does not exist yet.

use super::*;

/// A demo dataset whose virtualization owners cannot be ordered.
///
/// Both variants mean `demo_data.rs` and this module have drifted apart, not that a user did
/// anything wrong — hence `internal_error`.
#[derive(Debug)]
pub(crate) enum DemoSeedError {
    /// A row names a virtualizing service the dataset never defines.
    UnresolvableOwner { owner: Uuid },
    /// Owner services wait on each other in a cycle, so no order writes each one after the
    /// service its own row (or its host's row) names.
    CyclicOwners { stuck: Uuid, names: Uuid },
}

impl std::fmt::Display for DemoSeedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnresolvableOwner { owner } => write!(
                f,
                "demo data declares virtualization_service_id {owner}, which matches no service \
                 in the dataset"
            ),
            Self::CyclicOwners { stuck, names } => write!(
                f,
                "demo data owner service {stuck} waits on {names}, which waits on it in turn; \
                 virtualization owners must form chains, not cycles"
            ),
        }
    }
}

impl From<DemoSeedError> for ApiError {
    fn from(e: DemoSeedError) -> Self {
        ApiError::internal_error(&e.to_string())
    }
}

/// One write step of the hoist: hosts first, then the owner services that run on them.
#[derive(Debug, Default)]
pub(crate) struct HoistLayer {
    /// Hosts that own a service in this layer — written first, because `services.host_id` FKs
    /// to them.
    pub hosts: Vec<Host>,
    /// The virtualizing services themselves.
    pub services: Vec<Service>,
}

/// A batch split so that no row is written before the service its `virtualization_service_id`
/// names.
#[derive(Debug)]
pub(crate) struct VirtualizationHoist {
    /// Owner rows in write order. A later layer may name a service from an earlier one: a
    /// hypervisor's VM that runs a container engine is hoisted one layer after the hypervisor.
    pub owner_layers: Vec<HoistLayer>,
    pub deferred_hosts: Vec<Host>,
    pub deferred_services: Vec<Service>,
}

/// Split a batch so every virtualizing service is written before the rows that name it.
///
/// Demo seeding writes subnets and hosts before services, but a DockerBridge subnet names the
/// runtime service that owns it, a VM host names its hypervisor service, and a container service
/// names its runtime — all now real foreign keys into `services`. Discovery hit the same problem
/// and broke the cycle by materialising the owner's row early (`hosts/service/create.rs:537`);
/// this is that idea for a bulk batch. It is not a true cycle: `services` FKs only to `hosts` and
/// `sites` — bindings live in their own table — so an owner service can be written as soon as
/// its host row exists.
///
/// Owners can nest: a VM under a hypervisor can itself run a container engine. Each owner service
/// goes in the first layer where both it and its host name only services already written, so a
/// chain of any depth orders itself; only a cycle fails.
///
/// **Where this deliberately differs from discovery.** `resolve_owner_service_id`
/// (`hosts/service/create.rs:36`) degrades an id it cannot resolve to `None`, which is right
/// there: the daemon mints service ids in memory and cannot resolve them across submissions, so
/// one unowned bridge beats a host that will not create. Demo data mints both ends of every
/// reference itself, so an id that does not resolve is an authoring bug — and nulling it would
/// quietly delete the exact relationship the demo exists to show (containers under their runtime,
/// VMs under their Proxmox host). So this errors where discovery degrades.
pub(crate) fn hoist_virtualization_owners(
    subnets: &[Subnet],
    hosts: &[Host],
    services: &[Service],
    already_written: &HashSet<Uuid>,
) -> Result<VirtualizationHoist, DemoSeedError> {
    let owner_ids: HashSet<Uuid> = subnets
        .iter()
        .filter_map(|s| s.base.virtualization_service_id)
        .chain(
            hosts
                .iter()
                .filter_map(|h| h.base.virtualization_service_id),
        )
        .chain(
            services
                .iter()
                .filter_map(|s| s.base.virtualization_service_id),
        )
        .filter(|id| !already_written.contains(id))
        .collect();

    let services_by_id: HashMap<Uuid, &Service> = services.iter().map(|s| (s.id, s)).collect();
    let hosts_by_id: HashMap<Uuid, &Host> = hosts.iter().map(|h| (h.id, h)).collect();

    // Resolve every owner up front, so an undefined one is reported as such rather than as a
    // cycle the loop below cannot break.
    let mut pending: Vec<&Service> = Vec::with_capacity(owner_ids.len());
    for owner in &owner_ids {
        pending.push(
            services_by_id
                .get(owner)
                .copied()
                .ok_or(DemoSeedError::UnresolvableOwner { owner: *owner })?,
        );
    }
    // Deterministic layers regardless of HashSet iteration order.
    pending.sort_by_key(|s| s.id);

    let mut written: HashSet<Uuid> = already_written.clone();
    let mut hoisted_host_ids: HashSet<Uuid> = HashSet::new();
    let mut owner_layers: Vec<HoistLayer> = Vec::new();

    while !pending.is_empty() {
        // The first service this owner's row, or its host's row, names that is not yet written.
        let blocker = |service: &Service| {
            let host_owner = hosts_by_id
                .get(&service.base.host_id)
                .and_then(|h| h.base.virtualization_service_id);
            [service.base.virtualization_service_id, host_owner]
                .into_iter()
                .flatten()
                .find(|id| !written.contains(id))
        };
        let (ready, waiting): (Vec<&Service>, Vec<&Service>) =
            pending.into_iter().partition(|s| blocker(s).is_none());

        if ready.is_empty() {
            let stuck = waiting[0];
            return Err(DemoSeedError::CyclicOwners {
                stuck: stuck.id,
                names: blocker(stuck).unwrap_or(stuck.id),
            });
        }

        let mut layer = HoistLayer::default();
        for service in ready {
            if hoisted_host_ids.insert(service.base.host_id)
                && let Some(host) = hosts_by_id.get(&service.base.host_id)
            {
                layer.hosts.push((*host).clone());
            }
            written.insert(service.id);
            layer.services.push(service.clone());
        }
        owner_layers.push(layer);
        pending = waiting;
    }

    let deferred_hosts = hosts
        .iter()
        .filter(|h| !hoisted_host_ids.contains(&h.id))
        .cloned()
        .collect();
    let deferred_services = services
        .iter()
        .filter(|s| !owner_ids.contains(&s.id))
        .cloned()
        .collect();

    Ok(VirtualizationHoist {
        owner_layers,
        deferred_hosts,
        deferred_services,
    })
}

/// Write a hoist's owner layers in order, returning every host and service it created.
pub(super) async fn write_owner_layers(
    services: &ServiceFactory,
    hoist: &VirtualizationHoist,
    entity: &AuthenticatedEntity,
    entity_tags: &mut Vec<EntityTag>,
) -> ApiResult<(Vec<Host>, Vec<Service>)> {
    let mut created_hosts = Vec::new();
    let mut created_services = Vec::new();
    for layer in &hoist.owner_layers {
        let hosts = services
            .host_service
            .create_many(&layer.hosts, entity.clone())
            .await?;
        collect_entity_tags(&hosts, entity_tags);
        created_hosts.extend(hosts);
        let layer_services = services
            .service_service
            .create_many(&layer.services, entity.clone())
            .await?;
        collect_entity_tags(&layer_services, entity_tags);
        created_services.extend(layer_services);
    }
    Ok((created_hosts, created_services))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{host, service, subnet};

    /// The shape the demo dataset actually has: a bare-metal runtime host, the runtime service on
    /// it, a bridge subnet owned by that service, and a guest host owned by it too.
    fn dataset() -> (Vec<Subnet>, Vec<Host>, Vec<Service>) {
        let site_id = Uuid::new_v4();

        let runtime_host = host(&site_id);
        let guest_host = host(&site_id);
        let runtime_service = service(&site_id, &runtime_host.id);
        let mut guest_service = service(&site_id, &guest_host.id);
        guest_service.base.virtualization_service_id = Some(runtime_service.id);

        let mut bridge = subnet(&site_id);
        bridge.base.virtualization_service_id = Some(runtime_service.id);

        let mut guest_host = guest_host;
        guest_host.base.virtualization_service_id = Some(runtime_service.id);

        (
            vec![bridge],
            vec![runtime_host, guest_host],
            vec![runtime_service, guest_service],
        )
    }

    /// Owner service ids per layer, in write order.
    fn layer_service_ids(hoist: &VirtualizationHoist) -> Vec<Vec<Uuid>> {
        hoist
            .owner_layers
            .iter()
            .map(|l| l.services.iter().map(|s| s.id).collect())
            .collect()
    }

    #[test]
    fn the_owner_and_its_host_are_written_before_everything_that_names_them() {
        let (subnets, hosts, services) = dataset();
        let runtime_service_id = subnets[0].base.virtualization_service_id.unwrap();
        let runtime_host_id = hosts[0].id;

        let hoist =
            hoist_virtualization_owners(&subnets, &hosts, &services, &HashSet::new()).unwrap();

        assert_eq!(layer_service_ids(&hoist), vec![vec![runtime_service_id]]);
        assert_eq!(
            hoist.owner_layers[0]
                .hosts
                .iter()
                .map(|h| h.id)
                .collect::<Vec<_>>(),
            vec![runtime_host_id]
        );
        // The guest half waits, and nothing is dropped on the floor.
        assert_eq!(hoist.deferred_hosts.len(), 1);
        assert_eq!(hoist.deferred_services.len(), 1);
        assert!(
            hoist
                .deferred_hosts
                .iter()
                .all(|h| h.base.virtualization_service_id == Some(runtime_service_id))
        );
    }

    #[test]
    fn an_owner_already_in_the_database_is_not_hoisted_again() {
        let (subnets, hosts, services) = dataset();
        let runtime_service_id = subnets[0].base.virtualization_service_id.unwrap();
        let already_written = HashSet::from([runtime_service_id]);

        let hoist =
            hoist_virtualization_owners(&subnets, &hosts, &services, &already_written).unwrap();

        assert!(hoist.owner_layers.is_empty());
        assert_eq!(hoist.deferred_hosts.len(), 2);
        assert_eq!(hoist.deferred_services.len(), 2);
    }

    #[test]
    fn an_owner_the_dataset_never_defines_is_an_error_not_a_null() {
        let (mut subnets, hosts, services) = dataset();
        subnets[0].base.virtualization_service_id = Some(Uuid::new_v4());

        let err = hoist_virtualization_owners(&subnets, &hosts, &services, &HashSet::new())
            .expect_err("an unresolvable owner must not be silently dropped");

        assert!(matches!(err, DemoSeedError::UnresolvableOwner { .. }));
    }

    /// Hypervisor → VM running a container engine → container: the engine service sits on a host
    /// the hypervisor service virtualizes, so it can only be written one layer later.
    #[test]
    fn a_nested_owner_is_written_one_layer_after_the_owner_it_names() {
        let site_id = Uuid::new_v4();
        let hypervisor_host = host(&site_id);
        let hypervisor = service(&site_id, &hypervisor_host.id);
        let mut vm_host = host(&site_id);
        vm_host.base.virtualization_service_id = Some(hypervisor.id);
        let engine = service(&site_id, &vm_host.id);
        let mut container_host = host(&site_id);
        container_host.base.virtualization_service_id = Some(engine.id);

        let hosts = vec![
            container_host.clone(),
            vm_host.clone(),
            hypervisor_host.clone(),
        ];
        let services = vec![engine.clone(), hypervisor.clone()];
        let hoist = hoist_virtualization_owners(&[], &hosts, &services, &HashSet::new()).unwrap();

        assert_eq!(
            layer_service_ids(&hoist),
            vec![vec![hypervisor.id], vec![engine.id]]
        );
        assert_eq!(hoist.owner_layers[0].hosts[0].id, hypervisor_host.id);
        assert_eq!(hoist.owner_layers[1].hosts[0].id, vm_host.id);
        assert_eq!(
            hoist
                .deferred_hosts
                .iter()
                .map(|h| h.id)
                .collect::<Vec<_>>(),
            vec![container_host.id]
        );
    }

    #[test]
    fn owners_that_wait_on_each_other_are_rejected_rather_than_mis_ordered() {
        let (subnets, hosts, mut services) = dataset();
        // The runtime service is now itself virtualized by the guest service, which the runtime
        // virtualizes: no write order satisfies both.
        let guest_service_id = services[1].id;
        services[0].base.virtualization_service_id = Some(guest_service_id);

        let err = hoist_virtualization_owners(&subnets, &hosts, &services, &HashSet::new())
            .expect_err("a cycle must fail loudly, not reach Postgres");

        assert!(matches!(err, DemoSeedError::CyclicOwners { .. }));
    }
}
