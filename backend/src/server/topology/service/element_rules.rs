use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::server::{
    hosts::r#impl::base::Host,
    interfaces::r#impl::base::IfOperStatus,
    services::r#impl::{
        base::Service,
        categories::ServiceCategory,
        definitions::{ServiceDefinitionExt, VirtualizationRole},
    },
    shared::entities::EntityDiscriminants,
    shared::types::Color,
    subnets::r#impl::base::Subnet,
    topology::types::{
        grouping::{
            ElementRule, GraphRule, IdentifiedRule, InlineGroup, InlineGroupRole,
            PlacementDecision, RulePlacement,
        },
        nodes::{ContainerType, Node, NodeType},
    },
};

/// Lookup maps for taggable entities. Pass whatever is relevant at the call site;
/// a missing lookup for an ancestor type just yields an empty tag set.
pub struct TaggableLookups<'a> {
    pub hosts: Option<&'a HashMap<Uuid, &'a Host>>,
    pub services: Option<&'a HashMap<Uuid, &'a Service>>,
    pub subnets: Option<&'a HashMap<Uuid, &'a Subnet>>,
}

impl<'a> TaggableLookups<'a> {
    pub fn empty() -> Self {
        Self {
            hosts: None,
            services: None,
            subnets: None,
        }
    }
}

/// Resolve the tag_ids that a ByTag element rule should match against for the
/// given element. For taggable elements (Host, Service, Subnet) the ancestor is
/// the element itself, so `taggable_ancestor_id` equals the element's id. For
/// non-taggable elements (IPAddress, Interface, Port) the caller passes the
/// structural owner id (e.g. a node's `host_id`), and the function walks
/// `parent_taggable_entity` once to reach the taggable ancestor. Returns an
/// empty set if the required lookup was not provided or the entity isn't found.
pub fn resolve_element_tag_ids(
    element_entity: EntityDiscriminants,
    taggable_ancestor_id: Uuid,
    lookups: &TaggableLookups<'_>,
) -> HashSet<Uuid> {
    let ancestor = if element_entity.is_taggable() {
        element_entity
    } else {
        match element_entity.parent_taggable_entity() {
            Some(t) => t,
            None => return HashSet::new(),
        }
    };
    match ancestor {
        EntityDiscriminants::Host => lookups
            .hosts
            .and_then(|m| m.get(&taggable_ancestor_id))
            .map(|h| h.base.tags.iter().copied().collect())
            .unwrap_or_default(),
        EntityDiscriminants::Service => lookups
            .services
            .and_then(|m| m.get(&taggable_ancestor_id))
            .map(|s| s.base.tags.iter().copied().collect())
            .unwrap_or_default(),
        EntityDiscriminants::Subnet => lookups
            .subnets
            .and_then(|m| m.get(&taggable_ancestor_id))
            .map(|s| s.base.tags.iter().copied().collect())
            .unwrap_or_default(),
        _ => HashSet::new(),
    }
}

/// Data resolved from an element node for matching against element rules.
#[derive(Clone)]
pub struct ElementMatchData {
    pub categories: HashSet<ServiceCategory>,
    pub tag_ids: HashSet<Uuid>,
    /// The entity type of this element (Host, Service, etc.) for rule filtering.
    pub element_entity: EntityDiscriminants,
    /// The service ID of the virtualizer managing this element (for ByHypervisor/ByContainerRuntime grouping).
    pub virtualizer_service_id: Option<Uuid>,
    /// The role of that virtualizer: which of ByHypervisor and ByContainerRuntime takes the element.
    pub virtualizer_role: Option<VirtualizationRole>,
    /// The deployment-unit identity this element belongs to (for ByStack grouping) —
    /// e.g. a Docker/Podman compose project. Runtime-agnostic: any orchestrator's
    /// deployment-group key feeds this field, so the rule stays vendor-neutral.
    pub deployment_group: Option<String>,
    /// Native VLAN entity UUID on this port (for ByVLAN grouping).
    pub native_vlan_id: Option<Uuid>,
    /// Native VLAN number (for grouping key and display).
    pub vlan_number: Option<u16>,
    /// Native VLAN name (for display in container header).
    pub vlan_name: Option<String>,
    /// Whether this port has tagged VLANs (for ByTrunkPort grouping).
    pub is_trunk_port: bool,
    /// Port operational status (for ByPortOpStatus grouping).
    pub oper_status: Option<IfOperStatus>,
}

/// Optional context for computing InlineOn placement decisions.
/// Only needed by builders that have virtualizer/host data (Workloads builder).
pub struct InlineContext<'a> {
    pub hosts: &'a HashMap<Uuid, &'a Host>,
    pub service_lookup: &'a HashMap<Uuid, &'a Service>,
    /// virtualizer_service_id → managed container service IDs
    pub virt_to_container_svcs: &'a HashMap<Uuid, Vec<Uuid>>,
    /// virtualizer_service_id → managed guest host IDs (VMs, LXC containers, container hosts)
    pub virt_to_guest_hosts: &'a HashMap<Uuid, Vec<Uuid>>,
}

/// Context passed to `compute_placements` for each rule invocation.
struct PlacementContext<'a> {
    rule_id: Uuid,
    elements_by_container: &'a HashMap<Uuid, Vec<Uuid>>,
    match_data: &'a HashMap<Uuid, ElementMatchData>,
    claimed: &'a HashSet<Uuid>,
    virtualizer_titles: Option<&'a HashMap<Uuid, String>>,
    inline_ctx: Option<&'a InlineContext<'a>>,
}

/// Compute all placement decisions for a single element rule.
///
/// The match on `ElementRule` is **exhaustive** — adding a new variant forces
/// the developer to decide what placement decisions it produces.
fn compute_placements(rule: &ElementRule, ctx: &PlacementContext) -> RulePlacement {
    match rule {
        ElementRule::ByHypervisor | ElementRule::ByContainerRuntime => {
            compute_virtualizer_placements(rule, ctx)
        }
        ElementRule::ByStack => compute_stack_placements(ctx),
        ElementRule::ByServiceCategory {
            categories, title, ..
        } => compute_service_category_placements(categories, title.as_deref(), ctx),
        ElementRule::ByTag { tag_ids, title } => {
            compute_tag_placements(tag_ids, title.as_deref(), ctx)
        }
        ElementRule::ByTrunkPort => compute_trunk_port_placements(ctx),
        ElementRule::ByVLAN => compute_vlan_placements(ctx),
        ElementRule::ByPortOpStatus => compute_port_op_status_placements(ctx),
    }
}

/// ByHypervisor / ByContainerRuntime: group elements by virtualizer service,
/// plus InlineOn decisions for services on guests and BecomeSubcontainer for virtualizer services.
///
/// Elements are taken by their virtualizer's role, not by entity: a container runtime manages
/// both container services (bridge networking) and container hosts (macvlan/ipvlan), and both
/// belong in the runtime's group.
fn compute_virtualizer_placements(rule: &ElementRule, ctx: &PlacementContext) -> RulePlacement {
    let mut result = RulePlacement {
        containers: Vec::new(),
        placements: HashMap::new(),
        claimed: HashSet::new(),
    };

    let is_hypervisor_rule = matches!(rule, ElementRule::ByHypervisor);
    let container_type = if is_hypervisor_rule {
        ContainerType::Hypervisor
    } else {
        ContainerType::ContainerRuntime
    };
    // ByHypervisor takes every manager whose children are hosts and that is not a runtime: a
    // hypervisor's guests, and a host's network identities.
    let takes_role = |role: Option<VirtualizationRole>| match role {
        Some(VirtualizationRole::Hypervisor { .. } | VirtualizationRole::IdentityHost { .. }) => {
            is_hypervisor_rule
        }
        Some(VirtualizationRole::ContainerRuntime { .. }) => !is_hypervisor_rule,
        None => false,
    };

    // Phase 1: Group existing elements into subcontainers (PlaceInContainer)
    for (parent_id, element_ids) in ctx.elements_by_container {
        let unclaimed: Vec<Uuid> = element_ids
            .iter()
            .filter(|id| !ctx.claimed.contains(id))
            .filter(|id| {
                ctx.match_data
                    .get(id)
                    .is_some_and(|d| takes_role(d.virtualizer_role))
            })
            .copied()
            .collect();
        if unclaimed.is_empty() {
            continue;
        }

        let mut by_virtualizer: HashMap<Option<Uuid>, Vec<Uuid>> = HashMap::new();
        for id in &unclaimed {
            let virt_id = ctx
                .match_data
                .get(id)
                .and_then(|d| d.virtualizer_service_id);
            by_virtualizer.entry(virt_id).or_default().push(*id);
        }

        for (virt_id_opt, ids) in by_virtualizer {
            let Some(vid) = virt_id_opt else {
                // No virtualizer — this rule doesn't act on these elements.
                // Don't produce a decision; a later rule or the builder may handle them.
                continue;
            };

            // Skip creating a subcontainer if the virtualizer runs on a VM —
            // Phase 2 will handle these services with InlineOn instead.
            if let Some(inline_ctx) = ctx.inline_ctx
                && let Some(virt_svc) = inline_ctx.service_lookup.get(&vid)
                && inline_ctx
                    .hosts
                    .get(&virt_svc.base.host_id)
                    .is_some_and(|h| h.base.virtualization_service_id.is_some())
            {
                continue;
            }

            let group_key = format!("{parent_id}:{}:{vid}", ctx.rule_id);
            let group_id = Uuid::new_v5(&Uuid::NAMESPACE_OID, group_key.as_bytes());

            result.containers.push(Node {
                id: group_id,
                node_type: NodeType::Container {
                    container_type,
                    parent_container_id: Some(*parent_id),
                    entity_id: None,
                    icon: None,
                    color: None,
                    associated_service_definition: ctx
                        .virtualizer_titles
                        .and_then(|t| t.get(&vid).cloned()),
                    element_rule_id: Some(ctx.rule_id),
                    will_accept_edges: rule.will_accept_edges(),
                },
                position: Default::default(),
                size: Default::default(),
                header: ctx.virtualizer_titles.and_then(|t| t.get(&vid).cloned()),
            });

            // Virtualizer service → BecomeSubcontainer
            result.placements.insert(
                vid,
                PlacementDecision::BecomeSubcontainer {
                    container_id: group_id,
                },
            );

            // Elements → PlaceInContainer
            for id in &ids {
                result.placements.insert(
                    *id,
                    PlacementDecision::PlaceInContainer {
                        container_id: group_id,
                    },
                );
            }
            result.claimed.extend(ids);
        }
    }

    // Phase 2: InlineOn decisions for elements of guest hosts (only if inline context provided)
    if let Some(inline_ctx) = ctx.inline_ctx {
        if is_hypervisor_rule {
            // Services on guest hosts (VMs, LXC containers, container hosts) → InlineOn
            // (no visual group)
            for (host_id, host) in inline_ctx.hosts.iter() {
                if host.base.virtualization_service_id.is_none() {
                    continue;
                }
                for svc in inline_ctx.service_lookup.values() {
                    if svc.base.host_id == *host_id
                        && svc.base.virtualization_service_id.is_none()
                        && !result.placements.contains_key(&svc.id)
                    {
                        result.placements.insert(
                            svc.id,
                            PlacementDecision::InlineOn {
                                node_id: *host_id,
                                inline_group: None,
                            },
                        );
                    }
                }
            }
        }

        // A manager this rule takes that runs on a guest → its children, services and hosts
        // alike, InlineOn that guest with a visual group: a runtime on a VM with its containers,
        // a guest with its network identities. Phase 1 skipped these managers.
        let manager_ids: HashSet<Uuid> = inline_ctx
            .virt_to_container_svcs
            .keys()
            .chain(inline_ctx.virt_to_guest_hosts.keys())
            .copied()
            .collect();
        let no_ids = Vec::new();
        for virt_svc_id in manager_ids {
            let Some(virt_svc) = inline_ctx.service_lookup.get(&virt_svc_id) else {
                continue;
            };
            if !takes_role(virt_svc.base.service_definition.virtualization_role()) {
                continue;
            }
            let member_svc_ids = inline_ctx
                .virt_to_container_svcs
                .get(&virt_svc_id)
                .unwrap_or(&no_ids);
            let member_host_ids = inline_ctx
                .virt_to_guest_hosts
                .get(&virt_svc_id)
                .unwrap_or(&no_ids);
            let Some(host) = inline_ctx.hosts.get(&virt_svc.base.host_id) else {
                continue;
            };
            if host.base.virtualization_service_id.is_none() {
                continue;
            }
            let guest_host_id = virt_svc.base.host_id;

            // Manager → Header
            result.placements.insert(
                virt_svc_id,
                PlacementDecision::InlineOn {
                    node_id: guest_host_id,
                    inline_group: Some(InlineGroup {
                        entity_id: virt_svc_id,
                        entity_type: EntityDiscriminants::Service,
                        group_id: virt_svc_id,
                        role: InlineGroupRole::Header,
                    }),
                },
            );

            // Managed services and hosts → Member
            let members = member_svc_ids
                .iter()
                .map(|id| (*id, EntityDiscriminants::Service))
                .chain(
                    member_host_ids
                        .iter()
                        .map(|id| (*id, EntityDiscriminants::Host)),
                );
            for (entity_id, entity_type) in members {
                result.placements.insert(
                    entity_id,
                    PlacementDecision::InlineOn {
                        node_id: guest_host_id,
                        inline_group: Some(InlineGroup {
                            entity_id,
                            entity_type,
                            group_id: virt_svc_id,
                            role: InlineGroupRole::Member,
                        }),
                    },
                );
            }

            // A managed host's own services follow it onto the guest, so they keep a node
            // (dependency edges attach to it) instead of pointing at the removed element.
            for svc in inline_ctx.service_lookup.values() {
                if member_host_ids.contains(&svc.base.host_id)
                    && svc.base.virtualization_service_id.is_none()
                {
                    result.placements.insert(
                        svc.id,
                        PlacementDecision::InlineOn {
                            node_id: guest_host_id,
                            inline_group: None,
                        },
                    );
                }
            }
        }
    }

    result
}

/// ByStack: group elements by their deployment-unit identity (`deployment_group`).
fn compute_stack_placements(ctx: &PlacementContext) -> RulePlacement {
    let mut result = RulePlacement {
        containers: Vec::new(),
        placements: HashMap::new(),
        claimed: HashSet::new(),
    };

    for (parent_id, element_ids) in ctx.elements_by_container {
        let unclaimed: Vec<Uuid> = element_ids
            .iter()
            .filter(|id| !ctx.claimed.contains(id))
            .copied()
            .collect();
        if unclaimed.is_empty() {
            continue;
        }

        let mut by_stack: HashMap<String, Vec<Uuid>> = HashMap::new();
        for id in &unclaimed {
            if let Some(group) = ctx
                .match_data
                .get(id)
                .and_then(|d| d.deployment_group.clone())
            {
                by_stack.entry(group).or_default().push(*id);
            }
        }

        for (project, ids) in by_stack {
            let group_id = Uuid::new_v5(
                &Uuid::NAMESPACE_OID,
                format!("stack:{project}:{parent_id}").as_bytes(),
            );

            result.containers.push(Node {
                id: group_id,
                node_type: NodeType::Container {
                    container_type: ContainerType::Stack,
                    parent_container_id: Some(*parent_id),
                    entity_id: None,
                    icon: None,
                    color: None,
                    associated_service_definition: None,
                    element_rule_id: Some(ctx.rule_id),
                    will_accept_edges: true,
                },
                position: Default::default(),
                size: Default::default(),
                header: Some(project),
            });

            for id in &ids {
                result.placements.insert(
                    *id,
                    PlacementDecision::PlaceInContainer {
                        container_id: group_id,
                    },
                );
            }
            result.claimed.extend(ids);
        }
    }

    result
}

/// Map each deployment-unit identity (compose project) → the container-runtime
/// service-definition id to use as that Stack subcontainer's logo.
///
/// A deployment unit is runtime-homogeneous in practice (a compose project runs
/// on one runtime), so a project with conflicting runtimes maps to `None` (no logo).
/// Builders call this and then stamp `associated_service_definition` on each
/// `ContainerType::Stack` node by its header (the project name).
pub(crate) fn stack_runtime_logos(services: &[Service]) -> HashMap<String, Option<&'static str>> {
    let mut logos: HashMap<String, Option<&'static str>> = HashMap::new();
    for service in services {
        let Some(virt) = &service.base.virtualization_metadata else {
            continue;
        };
        let Some(project) = virt.compose_project() else {
            continue;
        };
        let runtime = virt.runtime_service_definition_id();
        logos
            .entry(project.to_string())
            .and_modify(|existing| {
                if *existing != Some(runtime) {
                    *existing = None;
                }
            })
            .or_insert(Some(runtime));
    }
    logos
}

/// Stamp the runtime logo onto every `ContainerType::Stack` node, keyed by its
/// header (deployment-unit / compose-project name). Shared by the graph builders.
pub(crate) fn apply_stack_runtime_logos(nodes: &mut [Node], services: &[Service]) {
    let logos = stack_runtime_logos(services);
    for node in nodes.iter_mut() {
        let header = node.header.clone();
        if let NodeType::Container {
            container_type: ContainerType::Stack,
            associated_service_definition,
            ..
        } = &mut node.node_type
        {
            *associated_service_definition = header
                .as_deref()
                .and_then(|h| logos.get(h).copied().flatten())
                .map(str::to_string);
        }
    }
}

/// ByServiceCategory: group elements matching specified categories.
fn compute_service_category_placements(
    categories: &[ServiceCategory],
    title: Option<&str>,
    ctx: &PlacementContext,
) -> RulePlacement {
    let mut result = RulePlacement {
        containers: Vec::new(),
        placements: HashMap::new(),
        claimed: HashSet::new(),
    };

    for (parent_id, element_ids) in ctx.elements_by_container {
        let matched_ids: HashSet<Uuid> = element_ids
            .iter()
            .filter(|id| !ctx.claimed.contains(id))
            .filter(|id| {
                ctx.match_data
                    .get(id)
                    .is_some_and(|d| categories.iter().any(|c| d.categories.contains(c)))
            })
            .copied()
            .collect();

        if matched_ids.is_empty() {
            continue;
        }

        let group_id = Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            format!("{parent_id}:{}", ctx.rule_id).as_bytes(),
        );

        result.containers.push(Node {
            id: group_id,
            node_type: NodeType::Container {
                container_type: ContainerType::NestedServiceCategory,
                parent_container_id: Some(*parent_id),
                entity_id: None,
                icon: None,
                color: None,
                associated_service_definition: None,
                element_rule_id: Some(ctx.rule_id),
                will_accept_edges: false,
            },
            position: Default::default(),
            size: Default::default(),
            header: title.map(String::from),
        });

        for id in &matched_ids {
            result.placements.insert(
                *id,
                PlacementDecision::PlaceInContainer {
                    container_id: group_id,
                },
            );
        }
        result.claimed.extend(matched_ids);
    }

    result
}

/// ByTag: group elements matching specified tag IDs.
fn compute_tag_placements(
    tag_ids: &[Uuid],
    title: Option<&str>,
    ctx: &PlacementContext,
) -> RulePlacement {
    let mut result = RulePlacement {
        containers: Vec::new(),
        placements: HashMap::new(),
        claimed: HashSet::new(),
    };

    for (parent_id, element_ids) in ctx.elements_by_container {
        let matched_ids: HashSet<Uuid> = element_ids
            .iter()
            .filter(|id| !ctx.claimed.contains(id))
            .filter(|id| {
                ctx.match_data
                    .get(id)
                    .is_some_and(|d| tag_ids.iter().any(|t| d.tag_ids.contains(t)))
            })
            .copied()
            .collect();

        if matched_ids.is_empty() {
            continue;
        }

        let group_id = Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            format!("{parent_id}:{}", ctx.rule_id).as_bytes(),
        );

        result.containers.push(Node {
            id: group_id,
            node_type: NodeType::Container {
                container_type: ContainerType::NestedTag,
                parent_container_id: Some(*parent_id),
                entity_id: None,
                icon: None,
                color: None,
                associated_service_definition: None,
                element_rule_id: Some(ctx.rule_id),
                will_accept_edges: false,
            },
            position: Default::default(),
            size: Default::default(),
            header: title.map(String::from),
        });

        for id in &matched_ids {
            result.placements.insert(
                *id,
                PlacementDecision::PlaceInContainer {
                    container_id: group_id,
                },
            );
        }
        result.claimed.extend(matched_ids);
    }

    result
}

/// ByTrunkPort: group trunk ports (ports with tagged VLANs).
fn compute_trunk_port_placements(ctx: &PlacementContext) -> RulePlacement {
    let mut result = RulePlacement {
        containers: Vec::new(),
        placements: HashMap::new(),
        claimed: HashSet::new(),
    };

    for (parent_id, element_ids) in ctx.elements_by_container {
        let matched_ids: Vec<Uuid> = element_ids
            .iter()
            .filter(|id| !ctx.claimed.contains(id))
            .filter(|id| ctx.match_data.get(id).is_some_and(|d| d.is_trunk_port))
            .copied()
            .collect();

        if matched_ids.is_empty() {
            continue;
        }

        let group_key = format!("{parent_id}:{}:trunk", ctx.rule_id);
        let group_id = Uuid::new_v5(&Uuid::NAMESPACE_OID, group_key.as_bytes());

        result.containers.push(Node {
            id: group_id,
            node_type: NodeType::Container {
                container_type: ContainerType::TrunkPort,
                parent_container_id: Some(*parent_id),
                entity_id: None,
                icon: None,
                color: None,
                associated_service_definition: None,
                element_rule_id: Some(ctx.rule_id),
                will_accept_edges: false,
            },
            position: Default::default(),
            size: Default::default(),
            header: Some("Trunk Ports".to_string()),
        });

        for id in &matched_ids {
            result.placements.insert(
                *id,
                PlacementDecision::PlaceInContainer {
                    container_id: group_id,
                },
            );
        }
        result.claimed.extend(matched_ids);
    }

    result
}

/// ByVLAN: group access ports by native VLAN number.
fn compute_vlan_placements(ctx: &PlacementContext) -> RulePlacement {
    let mut result = RulePlacement {
        containers: Vec::new(),
        placements: HashMap::new(),
        claimed: HashSet::new(),
    };

    for (parent_id, element_ids) in ctx.elements_by_container {
        let unclaimed: Vec<Uuid> = element_ids
            .iter()
            .filter(|id| !ctx.claimed.contains(id))
            .copied()
            .collect();
        if unclaimed.is_empty() {
            continue;
        }

        let mut by_vlan: HashMap<u16, Vec<Uuid>> = HashMap::new();
        for id in &unclaimed {
            if let Some(vlan_number) = ctx.match_data.get(id).and_then(|d| d.vlan_number) {
                by_vlan.entry(vlan_number).or_default().push(*id);
            }
        }

        for (vlan_number, ids) in by_vlan {
            let group_key = format!("{parent_id}:{}:vlan:{vlan_number}", ctx.rule_id);
            let group_id = Uuid::new_v5(&Uuid::NAMESPACE_OID, group_key.as_bytes());

            let vlan_name = ids
                .first()
                .and_then(|id| ctx.match_data.get(id))
                .and_then(|d| d.vlan_name.as_deref());

            let header = match vlan_name {
                Some(name) => format!("VLAN {} ({})", vlan_number, name),
                None => format!("VLAN {}", vlan_number),
            };

            result.containers.push(Node {
                id: group_id,
                node_type: NodeType::Container {
                    container_type: ContainerType::VLAN,
                    parent_container_id: Some(*parent_id),
                    entity_id: None,
                    icon: None,
                    color: None,
                    associated_service_definition: None,
                    element_rule_id: Some(ctx.rule_id),
                    will_accept_edges: false,
                },
                position: Default::default(),
                size: Default::default(),
                header: Some(header),
            });

            for id in &ids {
                result.placements.insert(
                    *id,
                    PlacementDecision::PlaceInContainer {
                        container_id: group_id,
                    },
                );
            }
            result.claimed.extend(ids);
        }
    }

    result
}

/// ByPortOpStatus: group ports by operational status.
fn compute_port_op_status_placements(ctx: &PlacementContext) -> RulePlacement {
    let mut result = RulePlacement {
        containers: Vec::new(),
        placements: HashMap::new(),
        claimed: HashSet::new(),
    };

    for (parent_id, element_ids) in ctx.elements_by_container {
        let unclaimed: Vec<Uuid> = element_ids
            .iter()
            .filter(|id| !ctx.claimed.contains(id))
            .copied()
            .collect();
        if unclaimed.is_empty() {
            continue;
        }

        // Keyed on `Option`, so a port whose status was never read gets a group of its own
        // rather than falling out of the rule. Dropping it would remove the port from the view
        // entirely — every element this rule does not place is left unclaimed and unshown — and a
        // port learned from a neighbour's advertisement has no status by construction.
        //
        // Distinct from `IfOperStatus::Unknown`, which is the MIB's own value 4: that is the
        // device saying it does not know, and merging the two would claim a reading nothing made.
        let mut by_status: HashMap<Option<IfOperStatus>, Vec<Uuid>> = HashMap::new();
        for id in &unclaimed {
            let status = ctx.match_data.get(id).and_then(|d| d.oper_status);
            by_status.entry(status).or_default().push(*id);
        }

        for (status, ids) in by_status {
            let status_name = match status {
                Some(status) => format!("{status:?}"),
                None => "Not reported".to_string(),
            };
            let group_key = format!(
                "{parent_id}:{}:status:{}",
                ctx.rule_id,
                status.map_or(-1, |s| s as i32)
            );
            let group_id = Uuid::new_v5(&Uuid::NAMESPACE_OID, group_key.as_bytes());

            let color = match status {
                Some(IfOperStatus::Up) => Color::Green,
                Some(IfOperStatus::Down | IfOperStatus::LowerLayerDown) => Color::Red,
                Some(IfOperStatus::Testing) => Color::Amber,
                Some(IfOperStatus::Dormant) => Color::Blue,
                Some(IfOperStatus::Unknown | IfOperStatus::NotPresent) | None => Color::Gray,
            };

            result.containers.push(Node {
                id: group_id,
                node_type: NodeType::Container {
                    container_type: ContainerType::PortOpStatus,
                    parent_container_id: Some(*parent_id),
                    entity_id: None,
                    icon: None,
                    color: Some(color),
                    associated_service_definition: None,
                    element_rule_id: Some(ctx.rule_id),
                    will_accept_edges: false,
                },
                position: Default::default(),
                size: Default::default(),
                header: Some(status_name),
            });

            for id in &ids {
                result.placements.insert(
                    *id,
                    PlacementDecision::PlaceInContainer {
                        container_id: group_id,
                    },
                );
            }
            result.claimed.extend(ids);
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Apply element rules to nodes, returning unified placement decisions.
///
/// Creates subcontainers, reassigns elements, and computes inline placements.
/// Returns a map of entity_id → PlacementDecision for ALL entities acted on.
///
/// `resolve_element` maps an Element node to its matchable data.
/// `virtualizer_titles` maps virtualizer service IDs to display names.
/// `inline_ctx` provides host/service data needed for InlineOn decisions (Workloads builder only).
pub fn apply_element_rules(
    nodes: &mut Vec<Node>,
    element_rules: &[IdentifiedRule<ElementRule>],
    resolve_element: impl Fn(&Node) -> Option<ElementMatchData>,
    virtualizer_titles: Option<&HashMap<Uuid, String>>,
    inline_ctx: Option<&InlineContext>,
) -> HashMap<Uuid, PlacementDecision> {
    if element_rules.is_empty() {
        return HashMap::new();
    }

    // Collect element nodes grouped by their current parent container
    let mut elements_by_container: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for node in nodes.iter() {
        if let NodeType::Element { container_id, .. } = &node.node_type {
            elements_by_container
                .entry(*container_id)
                .or_default()
                .push(node.id);
        }
    }

    // Resolve match data for all element nodes upfront
    let match_data: HashMap<Uuid, ElementMatchData> = nodes
        .iter()
        .filter(|n| matches!(n.node_type, NodeType::Element { .. }))
        .filter_map(|n| resolve_element(n).map(|data| (n.id, data)))
        .collect();

    let mut claimed: HashSet<Uuid> = HashSet::new();
    let mut all_placements: HashMap<Uuid, PlacementDecision> = HashMap::new();
    let mut new_containers: Vec<Node> = Vec::new();
    let mut reassignments: HashMap<Uuid, Uuid> = HashMap::new();

    for IdentifiedRule { id: rule_id, rule } in element_rules {
        let ctx = PlacementContext {
            rule_id: *rule_id,
            elements_by_container: &elements_by_container,
            match_data: &match_data,
            claimed: &claimed,
            virtualizer_titles,
            inline_ctx,
        };

        let placement = compute_placements(rule, &ctx);

        // Merge results
        new_containers.extend(placement.containers);
        claimed.extend(placement.claimed);

        for (entity_id, decision) in placement.placements {
            if let PlacementDecision::PlaceInContainer { container_id } = &decision {
                reassignments.insert(entity_id, *container_id);
            }
            all_placements.insert(entity_id, decision);
        }
    }

    // Apply reassignments to node container_ids
    for node in nodes.iter_mut() {
        if let NodeType::Element {
            ref mut container_id,
            ..
        } = node.node_type
            && let Some(new_id) = reassignments.get(&node.id)
        {
            *container_id = *new_id;
        }
    }

    nodes.extend(new_containers);

    all_placements
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::services::r#impl::{
        base::ServiceBase,
        virtualization::{DockerVirtualization, PodmanVirtualization, ServiceVirtualization},
    };
    use crate::server::topology::types::{grouping::IdentifiedRule, nodes::ElementEntityType};

    fn make_element(id: Uuid, container_id: Uuid) -> Node {
        Node::element(
            id,
            container_id,
            Uuid::new_v4(),
            ElementEntityType::Service {},
        )
    }

    fn make_match_data(deployment_group: Option<&str>) -> ElementMatchData {
        ElementMatchData {
            categories: HashSet::new(),
            tag_ids: HashSet::new(),
            element_entity: EntityDiscriminants::Service,
            virtualizer_service_id: None,
            virtualizer_role: None,
            deployment_group: deployment_group.map(String::from),
            native_vlan_id: None,
            vlan_number: None,
            vlan_name: None,
            is_trunk_port: false,
            oper_status: None,
        }
    }

    #[test]
    fn by_stack_groups_same_compose_project() {
        let container_id = Uuid::new_v4();
        let svc1 = Uuid::new_v4();
        let svc2 = Uuid::new_v4();
        let mut nodes = vec![
            make_element(svc1, container_id),
            make_element(svc2, container_id),
        ];

        let match_map: HashMap<Uuid, ElementMatchData> = [
            (svc1, make_match_data(Some("media-stack"))),
            (svc2, make_match_data(Some("media-stack"))),
        ]
        .into();

        let rules = vec![IdentifiedRule::new(ElementRule::ByStack)];
        apply_element_rules(
            &mut nodes,
            &rules,
            |node| match_map.get(&node.id).cloned(),
            None,
            None,
        );

        // Both services should be in the same new container
        let svc1_container = nodes
            .iter()
            .find(|n| n.id == svc1)
            .map(|n| match &n.node_type {
                NodeType::Element { container_id, .. } => *container_id,
                _ => panic!("expected element"),
            })
            .unwrap();
        let svc2_container = nodes
            .iter()
            .find(|n| n.id == svc2)
            .map(|n| match &n.node_type {
                NodeType::Element { container_id, .. } => *container_id,
                _ => panic!("expected element"),
            })
            .unwrap();
        assert_eq!(svc1_container, svc2_container);
        assert_ne!(svc1_container, container_id); // moved out of original container

        // Verify the Stack subcontainer was created
        let stack_container = nodes.iter().find(|n| n.id == svc1_container).unwrap();
        assert!(matches!(
            stack_container.node_type,
            NodeType::Container {
                container_type: ContainerType::Stack,
                ..
            }
        ));
        assert_eq!(stack_container.header.as_deref(), Some("media-stack"));
    }

    #[test]
    fn by_stack_no_compose_project_stays_ungrouped() {
        let container_id = Uuid::new_v4();
        let svc1 = Uuid::new_v4();
        let mut nodes = vec![make_element(svc1, container_id)];

        let match_map: HashMap<Uuid, ElementMatchData> = [(svc1, make_match_data(None))].into();

        let rules = vec![IdentifiedRule::new(ElementRule::ByStack)];
        apply_element_rules(
            &mut nodes,
            &rules,
            |node| match_map.get(&node.id).cloned(),
            None,
            None,
        );

        // Should stay in original container
        let svc1_container = match &nodes[0].node_type {
            NodeType::Element { container_id, .. } => *container_id,
            _ => panic!("expected element"),
        };
        assert_eq!(svc1_container, container_id);

        // No new containers created
        assert_eq!(nodes.len(), 1);
    }

    #[test]
    fn by_stack_different_projects_get_different_containers() {
        let container_id = Uuid::new_v4();
        let svc1 = Uuid::new_v4();
        let svc2 = Uuid::new_v4();
        let svc3 = Uuid::new_v4();
        let mut nodes = vec![
            make_element(svc1, container_id),
            make_element(svc2, container_id),
            make_element(svc3, container_id),
        ];

        let match_map: HashMap<Uuid, ElementMatchData> = [
            (svc1, make_match_data(Some("media-stack"))),
            (svc2, make_match_data(Some("monitoring"))),
            (svc3, make_match_data(Some("media-stack"))),
        ]
        .into();

        let rules = vec![IdentifiedRule::new(ElementRule::ByStack)];
        apply_element_rules(
            &mut nodes,
            &rules,
            |node| match_map.get(&node.id).cloned(),
            None,
            None,
        );

        let get_container = |id: Uuid| -> Uuid {
            nodes
                .iter()
                .find(|n| n.id == id)
                .map(|n| match &n.node_type {
                    NodeType::Element { container_id, .. } => *container_id,
                    _ => panic!("expected element"),
                })
                .unwrap()
        };

        // svc1 and svc3 share media-stack
        assert_eq!(get_container(svc1), get_container(svc3));
        // svc2 is in a different container (monitoring)
        assert_ne!(get_container(svc1), get_container(svc2));
        // Both are moved out of original
        assert_ne!(get_container(svc1), container_id);
        assert_ne!(get_container(svc2), container_id);

        // Two Stack subcontainers created
        let stack_count = nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.node_type,
                    NodeType::Container {
                        container_type: ContainerType::Stack,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(stack_count, 2);
    }

    fn service_with_virt(virt: ServiceVirtualization) -> Service {
        Service {
            base: ServiceBase {
                virtualization_metadata: Some(virt),
                virtualization_service_id: Some(Uuid::new_v4()),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn docker_virt(project: Option<&str>) -> ServiceVirtualization {
        ServiceVirtualization::Docker(DockerVirtualization {
            container_name: None,
            container_id: None,
            compose_project: project.map(String::from),
        })
    }

    fn podman_virt(project: Option<&str>) -> ServiceVirtualization {
        ServiceVirtualization::Podman(PodmanVirtualization {
            container_name: None,
            container_id: None,
            compose_project: project.map(String::from),
        })
    }

    fn stack_container(header: &str) -> Node {
        Node {
            id: Uuid::new_v4(),
            node_type: NodeType::Container {
                container_type: ContainerType::Stack,
                parent_container_id: None,
                entity_id: None,
                icon: None,
                color: None,
                associated_service_definition: None,
                element_rule_id: None,
                will_accept_edges: true,
            },
            position: Default::default(),
            size: Default::default(),
            header: Some(header.to_string()),
        }
    }

    fn associated_def(node: &Node) -> Option<&str> {
        match &node.node_type {
            NodeType::Container {
                associated_service_definition,
                ..
            } => associated_service_definition.as_deref(),
            _ => panic!("expected container"),
        }
    }

    #[test]
    fn stack_runtime_logos_maps_each_runtime_to_its_service_definition() {
        let services = vec![
            service_with_virt(docker_virt(Some("media"))),
            service_with_virt(podman_virt(Some("infra"))),
        ];
        let logos = stack_runtime_logos(&services);
        assert_eq!(logos.get("media").copied().flatten(), Some("Docker"));
        assert_eq!(logos.get("infra").copied().flatten(), Some("Podman"));
    }

    #[test]
    fn stack_runtime_logos_conflicting_runtime_is_none() {
        // Same project name on two different runtimes → ambiguous logo.
        let services = vec![
            service_with_virt(docker_virt(Some("shared"))),
            service_with_virt(podman_virt(Some("shared"))),
        ];
        let logos = stack_runtime_logos(&services);
        assert_eq!(logos.get("shared").copied().flatten(), None);
    }

    #[test]
    fn apply_stack_runtime_logos_stamps_podman_and_docker_stacks() {
        let mut nodes = vec![stack_container("media"), stack_container("infra")];
        let services = vec![
            service_with_virt(docker_virt(Some("media"))),
            service_with_virt(podman_virt(Some("infra"))),
        ];
        apply_stack_runtime_logos(&mut nodes, &services);
        assert_eq!(associated_def(&nodes[0]), Some("Docker"));
        assert_eq!(associated_def(&nodes[1]), Some("Podman"));
    }

    #[test]
    fn apply_stack_runtime_logos_leaves_unknown_stack_without_logo() {
        let mut nodes = vec![stack_container("orphan")];
        let services = vec![service_with_virt(docker_virt(Some("media")))];
        apply_stack_runtime_logos(&mut nodes, &services);
        assert_eq!(associated_def(&nodes[0]), None);
    }
}
