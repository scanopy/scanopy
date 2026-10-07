import type { components } from '$lib/api/schema';
import type { RenderableTopology, TopologyEdge, TopologyNode } from './types/base';
import { entities } from '$lib/shared/stores/metadata';
import { hostDisplayName } from '$lib/features/hosts/host-display-name';
import { getTopologyIndex, type ContainerContents } from './entity-index';
import type { FreshnessSubject } from '$lib/shared/utils/freshness';

/**
 * Where each entity type's instances live on a Topology.
 *
 * Entity-registry data, not view- or feature-specific: the only knowledge encoded is "Service
 * entities live in topology.services". One owner, because two partial copies of this had already
 * drifted — a hardcoded five-type switch in the filter pass, which silently matched nothing for
 * every type it omitted, and a fuller map in the layout pipeline. Adding an entity type is a
 * one-line append here.
 */
export const ENTITY_COLLECTIONS: Record<string, keyof RenderableTopology> = {
	Service: 'services',
	Port: 'ports',
	Interface: 'interfaces',
	IPAddress: 'ip_addresses',
	Host: 'hosts',
	Subnet: 'subnets',
	Binding: 'bindings',
	Dependency: 'dependencies',
	Vlan: 'vlans'
};

/** Instances of `entityType` on `topo`, or `undefined` if the type has no collection. */
export function entityCollection(
	topo: RenderableTopology,
	entityType: string
): Array<{ id: string }> | undefined {
	const key = ENTITY_COLLECTIONS[entityType];
	if (!key) return undefined;
	const collection = topo[key];
	return Array.isArray(collection) ? (collection as Array<{ id: string }>) : undefined;
}

type ElementEntityType = components['schemas']['ElementEntityType'];
type ElementEntityTypeDiscriminant = ElementEntityType['element_type'];
type InlineGroup = components['schemas']['InlineGroup'];

// Resolver return types
export interface ElementRenderContext {
	elementType: ElementEntityTypeDiscriminant;
	host: RenderableTopology['hosts'][number] | undefined;
	ipAddress: RenderableTopology['ip_addresses'][number] | undefined;
	snmpInterface: RenderableTopology['interfaces'][number] | undefined;
	services: RenderableTopology['services'][number][];
	hostId: string | undefined;
	ipAddressId: string | undefined;
	interfaceId: string | undefined;
	subnetId: string;
	isInfra: boolean;
}

export interface ContainerRenderContext {
	tags: string[];
	title: string | null;
	containerType: string;
}

// Exhaustive resolver maps — TypeScript errors if a variant is missing
const elementResolvers: Record<
	ElementEntityTypeDiscriminant,
	(nodeId: string, node: TopologyNode, topology: RenderableTopology) => ElementRenderContext
> = {
	IPAddress: (_nodeId, node, topology) => {
		const index = getTopologyIndex(topology);
		const hostId = 'host_id' in node ? (node.host_id as string) : undefined;
		const ipAddressId =
			'ip_address_id' in node ? (node.ip_address_id as string | undefined) : undefined;
		const subnetId = 'subnet_id' in node ? (node.subnet_id as string) : '';
		const isInfra = 'is_infra' in node ? (node.is_infra as boolean) : false;

		const host = hostId ? index.hostsById.get(hostId) : undefined;
		const ipAddress = ipAddressId ? index.ipAddressesById.get(ipAddressId) : undefined;
		// Narrowed to this host's services first, then the same binding predicate
		// as before — so ordering and semantics are unchanged, but the scan is
		// over one host's services rather than every service in the topology.
		const services = (hostId ? (index.servicesByHostId.get(hostId) ?? []) : []).filter((s) =>
			s.bindings.some((b) => b.ip_address_id === ipAddressId || b.ip_address_id === null)
		);

		return {
			elementType: 'IPAddress',
			host,
			ipAddress,
			snmpInterface: undefined,
			services,
			hostId,
			ipAddressId: ipAddressId,
			interfaceId: undefined,
			subnetId,
			isInfra
		};
	},
	Service: (nodeId, node, topology) => {
		const index = getTopologyIndex(topology);
		const hostId = 'host_id' in node ? (node.host_id as string) : undefined;
		const host = hostId ? index.hostsById.get(hostId) : undefined;
		const service = index.servicesById.get(nodeId);
		const services = service ? [service] : [];

		return {
			elementType: 'Service' as ElementEntityTypeDiscriminant,
			host,
			ipAddress: undefined,
			snmpInterface: undefined,
			services,
			hostId,
			ipAddressId: undefined,
			interfaceId: undefined,
			subnetId: '',
			isInfra: false
		};
	},
	Host: (_nodeId, node, topology) => {
		const index = getTopologyIndex(topology);
		const hostId = 'host_id' in node ? (node.host_id as string) : undefined;
		const host = hostId ? index.hostsById.get(hostId) : undefined;
		const services = hostId ? (index.servicesByHostId.get(hostId) ?? []) : [];

		return {
			elementType: 'Host' as ElementEntityTypeDiscriminant,
			host,
			ipAddress: undefined,
			snmpInterface: undefined,
			services,
			hostId,
			ipAddressId: undefined,
			interfaceId: undefined,
			subnetId: '',
			isInfra: false
		};
	},
	Interface: (_nodeId, node, topology) => {
		const index = getTopologyIndex(topology);
		const hostId = 'host_id' in node ? (node.host_id as string) : undefined;
		const interfaceId = 'interface_id' in node ? (node.interface_id as string) : undefined;
		const host = hostId ? index.hostsById.get(hostId) : undefined;
		const snmpInterface = interfaceId ? index.interfacesById.get(interfaceId) : undefined;
		return {
			elementType: 'Interface' as ElementEntityTypeDiscriminant,
			host,
			ipAddress: undefined,
			snmpInterface,
			services: [],
			hostId,
			ipAddressId: undefined,
			interfaceId: interfaceId,
			subnetId: '',
			isInfra: false
		};
	}
};

// Returns tags generically from whatever entity
// the container represents
function resolveContainer(
	nodeId: string,
	node: TopologyNode,
	topology: RenderableTopology
): ContainerRenderContext {
	const containerType = 'container_type' in node ? (node.container_type as string) : 'Subnet';
	const title = 'header' in node ? (node.header as string | null) : null;
	return { tags: resolveContainerTags(nodeId, node, topology), title, containerType };
}

/**
 * Resolve tags for any container entity, regardless of container type.
 * Uses entity_id (the entity this container represents) for direct lookup,
 * falling back to node ID match (for Subnet containers where ID === entity ID)
 * and then to getContainerContents for legacy/indirect resolution.
 */
function resolveContainerTags(
	nodeId: string,
	node: TopologyNode,
	topology: RenderableTopology
): string[] {
	const { entityTags, containerContents } = getTopologyIndex(topology);

	// Use entity_id for direct entity lookup (set by builders on all containers)
	const entityId = 'entity_id' in node ? (node.entity_id as string | undefined) : undefined;
	if (entityId && entityTags.has(entityId)) return entityTags.get(entityId)!;

	// Fallback: direct ID match (e.g. Subnet container ID === subnet entity ID)
	if (entityTags.has(nodeId)) return entityTags.get(nodeId)!;

	// Indirect: find entities inside this container, return first match
	const contents = containerContents(nodeId);
	for (const id of contents.hostIds) {
		if (entityTags.has(id)) return entityTags.get(id)!;
	}
	for (const id of contents.serviceIds) {
		if (entityTags.has(id)) return entityTags.get(id)!;
	}
	return [];
}

// Dependency creation targets — one per selected node that can become a dep member.
// Dependencies are a Services/Bindings concept; L2 Interface elements and non-Host
// containers (Subnet, Application) are filtered out at resolution time.
export type DependencyTarget =
	| { type: 'service'; serviceId: string; elementId: string; label: string; hostName: string }
	| {
			type: 'host';
			hostId: string;
			candidateServiceIds: string[];
			elementId: string;
			label: string;
	  }
	| {
			type: 'ipAddress';
			hostId: string;
			ipAddressId: string;
			candidateServiceIds: string[];
			elementId: string;
			label: string;
			hostName: string;
	  };

/**
 * Resolve selected topology nodes into dependency targets without fanning out.
 * A host container / host element → a single `host` target the user must disambiguate.
 * An IP address element → a single `ipAddress` target scoped to services at that IP.
 * A service element → a direct `service` target.
 * Interfaces, subnet containers, application containers, and unknowns are dropped.
 */
export function resolveDependencyTargets(
	selectedNodes: { id: string; data: unknown }[],
	topology: RenderableTopology
): DependencyTarget[] {
	const targets: DependencyTarget[] = [];
	const seen = new Set<string>();

	for (const node of selectedNodes) {
		const data = node.data as TopologyNode | undefined;
		if (!data) continue;

		if (data.node_type === 'Container') {
			const containerType =
				'container_type' in data ? (data.container_type as string | undefined) : undefined;
			if (containerType !== 'Host') continue;
			const entityId = 'entity_id' in data ? (data.entity_id as string | undefined) : undefined;
			if (!entityId) continue;
			const host = topology.hosts.find((h) => h.id === entityId);
			if (!host) continue;
			const key = `host:${entityId}`;
			if (seen.has(key)) continue;
			seen.add(key);
			const candidateServiceIds = topology.services
				.filter((s) => s.host_id === entityId)
				.map((s) => s.id);
			targets.push({
				type: 'host',
				hostId: entityId,
				candidateServiceIds,
				elementId: node.id,
				label: hostDisplayName(host)
			});
			continue;
		}

		if (data.node_type !== 'Element') continue;
		const elementType = data.element_type;

		if (elementType === 'Service') {
			const key = `service:${node.id}`;
			if (seen.has(key)) continue;
			seen.add(key);
			const service = topology.services.find((s) => s.id === node.id);
			if (!service) continue;
			const host = topology.hosts.find((h) => h.id === service.host_id);
			targets.push({
				type: 'service',
				serviceId: node.id,
				elementId: node.id,
				label: service.name,
				hostName: host ? hostDisplayName(host) : ''
			});
		} else if (elementType === 'Host') {
			const hostId = 'host_id' in data ? (data.host_id as string | undefined) : undefined;
			if (!hostId) continue;
			const host = topology.hosts.find((h) => h.id === hostId);
			if (!host) continue;
			const key = `host:${hostId}`;
			if (seen.has(key)) continue;
			seen.add(key);
			const candidateServiceIds = topology.services
				.filter((s) => s.host_id === hostId)
				.map((s) => s.id);
			targets.push({
				type: 'host',
				hostId,
				candidateServiceIds,
				elementId: node.id,
				label: hostDisplayName(host)
			});
		} else if (elementType === 'IPAddress') {
			const resolved = resolveElementNode(node.id, data, topology);
			if (!resolved.hostId || !resolved.ipAddressId) continue;
			const key = `ip:${resolved.hostId}:${resolved.ipAddressId}`;
			if (seen.has(key)) continue;
			seen.add(key);
			const candidateServiceIds = resolved.services.map((s) => s.id);
			const ipLabel = resolved.ipAddress
				? (resolved.ipAddress.name ? `${resolved.ipAddress.name}: ` : '') +
					resolved.ipAddress.ip_address
				: resolved.ipAddressId;
			targets.push({
				type: 'ipAddress',
				hostId: resolved.hostId,
				ipAddressId: resolved.ipAddressId,
				candidateServiceIds,
				elementId: node.id,
				label: ipLabel,
				hostName: resolved.host ? hostDisplayName(resolved.host) : ''
			});
		}
		// Interface elements: not valid dep targets (L2 uses PhysicalLink edges instead) — skip.
	}

	return targets;
}

/**
 * Targets of a dependency being edited: its saved members, in order and minus any the user
 * removed, then whatever else is selected, resolved like a new dependency's targets.
 *
 * Editing selects each saved member as the node that shows it: the service itself, or (L3) the
 * IP card its binding sits on. Those nodes, removed members' included, are not new targets.
 */
export function resolveEditDependencyTargets(
	dependency: components['schemas']['Dependency'],
	selectedNodes: { id: string; data: unknown }[],
	topology: RenderableTopology,
	removedServiceIds: ReadonlySet<string>
): DependencyTarget[] {
	const members = dependency.members;
	const serviceIds: string[] =
		members.type === 'Services'
			? [...members.service_ids]
			: members.binding_ids
					.map((bid) => topology.services.find((s) => s.bindings.some((b) => b.id === bid))?.id)
					.filter((id): id is string => !!id);

	const memberTargets = serviceIds
		.filter((sid) => !removedServiceIds.has(sid))
		.map((sid): DependencyTarget => {
			const svc = topology.services.find((s) => s.id === sid);
			const host = svc ? topology.hosts.find((h) => h.id === svc.host_id) : undefined;
			return {
				type: 'service',
				serviceId: sid,
				elementId: sid,
				label: svc?.name ?? '',
				hostName: host ? hostDisplayName(host) : ''
			};
		});

	const memberNodeIds = new Set<string>();
	for (const sid of serviceIds) {
		memberNodeIds.add(sid);
		const svc = topology.services.find((s) => s.id === sid);
		for (const b of svc?.bindings ?? []) {
			if (b.ip_address_id) memberNodeIds.add(b.ip_address_id);
		}
	}
	const addedTargets = resolveDependencyTargets(
		selectedNodes.filter((n) => !memberNodeIds.has(n.id)),
		topology
	);

	return [...memberTargets, ...addedTargets];
}

// Resolve the taggable entity behind an element node. Walks the element_type's
// parent_taggable_entity chain (per entity metadata) until a taggable entity is reached.
// Returns null for containers, unknown elements, or chains with no taggable ancestor.
export interface TagTarget {
	entityType: 'Host' | 'Service';
	entityId: string;
}

export function resolveTagTarget(nodeId: string, node: TopologyNode): TagTarget | null {
	if (node.node_type !== 'Element') return null;
	const elementType = node.element_type;
	if (!elementType) return null;
	const hostId = 'host_id' in node ? (node.host_id as string | undefined) : undefined;

	let type: string | undefined = elementType;
	while (type) {
		const meta = entities.getMetadata(type);
		if (meta.is_taggable) {
			if (type === 'Host') return hostId ? { entityType: 'Host', entityId: hostId } : null;
			if (type === 'Service') return { entityType: 'Service', entityId: nodeId };
			return null;
		}
		type = meta.parent_taggable_entity;
	}
	return null;
}

// Selection context for multi-select operations
export interface NodeSelectionIds {
	hostIds: string[];
	serviceIds: string[];
}

/**
 * Get the host and service IDs represented by an element node.
 * Handles both Interface nodes (services bound to the interface) and
 * Service nodes (the service itself) uniformly.
 */
export function getNodeSelectionIds(
	nodeId: string,
	node: TopologyNode,
	topology: RenderableTopology
): NodeSelectionIds {
	const resolved = resolveElementNode(nodeId, node, topology);
	const hostIds = resolved.hostId ? [resolved.hostId] : [];

	if (resolved.elementType === 'Service') {
		return { hostIds, serviceIds: resolved.services.map((s) => s.id) };
	}
	if (resolved.elementType === 'Host') {
		return { hostIds, serviceIds: resolved.services.map((s) => s.id) };
	}
	// Interface node: services bound to this specific interface on this host
	const serviceIds = topology.services
		.filter(
			(s) =>
				s.host_id &&
				hostIds.includes(s.host_id) &&
				s.bindings.some((b) => b.ip_address_id === resolved.ipAddressId || b.ip_address_id === null)
		)
		.map((s) => s.id);
	return { hostIds, serviceIds };
}

// Container contents — shared utility for fading, hiding, counting
export type { ContainerContents };

/**
 * Walk topology nodes and return all entities inside a container,
 * including entities in nested subcontainers. Works generically across
 * views — uses container_id/subnet_id for elements and
 * parent_container_id for subcontainers.
 */
export function getContainerContents(
	containerId: string,
	topologyNodes: TopologyNode[]
): ContainerContents {
	const hostIds = new Set<string>();
	const serviceIds = new Set<string>();
	const interfaceIds = new Set<string>();
	const elementNodeIds = new Set<string>();
	const subcontainerIds = new Set<string>();

	// Collect subcontainer IDs (direct children of this container)
	for (const nd of topologyNodes) {
		if (nd.node_type === 'Container') {
			const parentContainerId = (nd as Record<string, unknown>).parent_container_id as
				string | undefined;
			if (parentContainerId === containerId) {
				subcontainerIds.add(nd.id);
			}
		}
	}

	// Collect element nodes whose parent is this container or a subcontainer
	const containerSet = new Set([containerId, ...subcontainerIds]);
	for (const nd of topologyNodes) {
		if (nd.node_type !== 'Element') continue;

		const parentId = (nd as Record<string, unknown>).container_id as string | undefined;
		if (!parentId || !containerSet.has(parentId)) continue;

		elementNodeIds.add(nd.id);

		const hostId = (nd as Record<string, unknown>).host_id as string | undefined;
		if (hostId) hostIds.add(hostId);

		if (nd.element_type === 'Service') {
			serviceIds.add(nd.id);
		} else if (nd.element_type === 'Interface') {
			const ifaceId = (nd as Record<string, unknown>).interface_id as string | undefined;
			if (ifaceId) interfaceIds.add(ifaceId);
		} else if (nd.element_type === 'Host') {
			// Host elements: no additional IDs needed beyond hostId (already added above)
		}
	}

	return { hostIds, serviceIds, interfaceIds, elementNodeIds, subcontainerIds };
}

/**
 * Service entity IDs that are *rendered inside* element nodes in a container
 * tree but don't have their own element-node entry in `topology.nodes`.
 * Examples:
 *   - L3: services rendered inside IP-address elements (bound services)
 *   - Workloads: services rendered inside Host elements (InlineOn'd by
 *     element rules — node removed, shown via host_id filter in the renderer)
 *
 * Walks the same element nodes the renderer resolves, using the same
 * host_id/binding relationships. Services that ARE element nodes themselves
 * are skipped — counted there, not here.
 */
export function resolveInlineServiceIds(
	elementNodeIds: Set<string>,
	topology: RenderableTopology
): Set<string> {
	const out = new Set<string>();
	const { nodesById, servicesByHostId } = getTopologyIndex(topology);

	const elementServiceIds = new Set<string>();
	for (const id of elementNodeIds) {
		const node = nodesById.get(id);
		if (node?.node_type === 'Element' && node.element_type === 'Service') {
			elementServiceIds.add(id);
		}
	}

	for (const id of elementNodeIds) {
		const node = nodesById.get(id);
		if (!node || node.node_type !== 'Element') continue;
		const hostId = (node as { host_id?: string }).host_id;
		if (!hostId) continue;
		const hostServices = servicesByHostId.get(hostId) ?? [];

		if (node.element_type === 'IPAddress') {
			const ipAddressId = (node as { ip_address_id?: string }).ip_address_id;
			for (const s of hostServices) {
				if (!s.bindings.some((b) => b.ip_address_id === ipAddressId || b.ip_address_id === null)) {
					continue;
				}
				if (!elementServiceIds.has(s.id)) out.add(s.id);
			}
		} else if (node.element_type === 'Host') {
			// The card also draws the services of hosts inlined on it (a guest's network
			// identities, a runtime's macvlan containers).
			const memberHostIds = ((node as { inline_groups?: InlineGroup[] }).inline_groups ?? [])
				.filter((g) => g.entity_type === 'Host')
				.map((g) => g.entity_id);
			const memberServices = memberHostIds.flatMap((h) => servicesByHostId.get(h) ?? []);
			for (const s of [...hostServices, ...memberServices]) {
				if (!elementServiceIds.has(s.id)) out.add(s.id);
			}
		}
	}

	return out;
}

/**
 * The container hosts (macvlan, ipvlan) a ContainerRuntime edge stands for. An edge to a
 * container host names no containerized services; its containers are the runtime's hosts with an
 * address on the subnet the edge reaches. Empty for an edge to bridge-network containers.
 */
export function containerHostsOfEdge(
	topology: RenderableTopology,
	edge: TopologyEdge
): RenderableTopology['hosts'] {
	if (edge.edge_type !== 'ContainerRuntime' || edge.containerized_service_ids.length > 0) {
		return [];
	}
	return topology.hosts.filter(
		(h) =>
			h.virtualization_service_id === edge.service_id &&
			topology.ip_addresses.some(
				(ip) => ip.host_id === h.id && edge.subnet_ids.includes(ip.subnet_id)
			)
	);
}

/**
 * The identity hosts a NetworkIdentity edge reaches: the host of the address the backend drew it
 * to. Edge elevation can replace that endpoint with the id of a box that accepts edges, and then
 * every identity of the edge's Network Identities service is returned.
 */
export function identityHostsOfEdge(
	topology: RenderableTopology,
	edge: TopologyEdge
): RenderableTopology['hosts'] {
	if (edge.edge_type !== 'NetworkIdentity') return [];
	const address = topology.ip_addresses.find((ip) => ip.id === edge.target);
	const addressHost = address ? topology.hosts.find((h) => h.id === address.host_id) : undefined;
	if (addressHost) return [addressHost];
	return topology.hosts.filter((h) => h.virtualization_service_id === edge.identities_service_id);
}

// Entity→Node index — canonical resolver for mapping entity IDs to topology node IDs
export interface EntityNodeIndex {
	hostIdToNodes: Map<string, string[]>;
	hostIdToContainerIds: Map<string, Set<string>>;
	ipAddressIdToNodes: Map<string, string[]>;
	serviceIdToNodes: Map<string, string[]>;
	interfaceIdToNodes: Map<string, string[]>;
	allElementNodeIds: Set<string>;
	allContainerNodeIds: Set<string>;
}

/**
 * Build an index mapping entity IDs to the topology node IDs that represent them.
 * Single pass over topology.nodes. Use this instead of ad-hoc entity→node lookups.
 */
export function buildEntityNodeIndex(nodes: TopologyNode[]): EntityNodeIndex {
	const hostIdToNodes = new Map<string, string[]>();
	const hostIdToContainerIds = new Map<string, Set<string>>();
	const ipAddressIdToNodes = new Map<string, string[]>();
	const serviceIdToNodes = new Map<string, string[]>();
	const interfaceIdToNodes = new Map<string, string[]>();
	const allElementNodeIds = new Set<string>();
	const allContainerNodeIds = new Set<string>();

	for (const nd of nodes) {
		if (nd.node_type === 'Container') {
			allContainerNodeIds.add(nd.id);
			// Map entity_id → container node for ownership tracking
			const entityId = 'entity_id' in nd ? (nd.entity_id as string | undefined) : undefined;
			if (entityId) {
				const containerSet = hostIdToContainerIds.get(entityId);
				if (containerSet) containerSet.add(nd.id);
				else hostIdToContainerIds.set(entityId, new Set([nd.id]));
			}
			continue;
		}
		if (nd.node_type !== 'Element') continue;

		allElementNodeIds.add(nd.id);

		const hostId = 'host_id' in nd ? (nd.host_id as string | undefined) : undefined;
		if (hostId) {
			const existing = hostIdToNodes.get(hostId);
			if (existing) existing.push(nd.id);
			else hostIdToNodes.set(hostId, [nd.id]);
		}

		if (nd.element_type === 'IPAddress') {
			const ipAddrId = 'ip_address_id' in nd ? (nd.ip_address_id as string | undefined) : undefined;
			if (ipAddrId) {
				const existing = ipAddressIdToNodes.get(ipAddrId);
				if (existing) existing.push(nd.id);
				else ipAddressIdToNodes.set(ipAddrId, [nd.id]);
			}
		} else if (nd.element_type === 'Interface') {
			const ifaceId = 'interface_id' in nd ? (nd.interface_id as string | undefined) : undefined;
			if (ifaceId) {
				const existing = interfaceIdToNodes.get(ifaceId);
				if (existing) existing.push(nd.id);
				else interfaceIdToNodes.set(ifaceId, [nd.id]);
			}
		} else if (nd.element_type === 'Service') {
			const existing = serviceIdToNodes.get(nd.id);
			if (existing) existing.push(nd.id);
			else serviceIdToNodes.set(nd.id, [nd.id]);
		}
	}

	return {
		hostIdToNodes,
		hostIdToContainerIds,
		ipAddressIdToNodes,
		serviceIdToNodes,
		interfaceIdToNodes,
		allElementNodeIds,
		allContainerNodeIds
	};
}

// Public API
export function resolveElementNode(
	nodeId: string,
	node: TopologyNode,
	topology: RenderableTopology
): ElementRenderContext {
	if (node.node_type !== 'Element') throw new Error(`Expected Element, got ${node.node_type}`);
	const elementType = node.element_type;
	if (!elementType || !(elementType in elementResolvers)) {
		console.warn(`[resolveElementNode] Unknown element_type: ${elementType} for node ${nodeId}`);
		return {
			elementType: elementType ?? ('Unknown' as ElementEntityTypeDiscriminant),
			host: undefined,
			ipAddress: undefined,
			snmpInterface: undefined,
			services: [],
			hostId: undefined,
			ipAddressId: undefined,
			interfaceId: undefined,
			subnetId: '',
			isInfra: false
		};
	}
	return elementResolvers[elementType](nodeId, node, topology);
}

export function resolveContainerNode(
	nodeId: string,
	node: TopologyNode,
	topology: RenderableTopology
): ContainerRenderContext {
	if (node.node_type !== 'Container') throw new Error(`Expected Container, got ${node.node_type}`);
	return resolveContainer(nodeId, node, topology);
}

/** An entity a topology node stands for, as the filter extractors and freshness helpers read it. */
export type NodeEntity = FreshnessSubject & { id: string; site_id?: string };

/**
 * The entity an element card depicts: its service, address or interface, and otherwise its host.
 * One owner for the card's stale pill and its filter-hover match, so the two cannot disagree.
 */
export function elementEntity(resolved: ElementRenderContext): NodeEntity | undefined {
	switch (resolved.elementType) {
		case 'Service':
			return resolved.services[0] ?? resolved.host;
		case 'IPAddress':
			return resolved.ipAddress ?? resolved.host;
		case 'Interface':
			return resolved.snmpInterface ?? resolved.host;
		default:
			return resolved.host;
	}
}

/**
 * The entity a card is judged by for a filter on `entityType`: its own when the filter is on its
 * type, its host when a Host filter meets an IPAddress or Interface card. The filter-value hover
 * ring and the view's element marks both read it, so a coloured card is always one the matching
 * chip rings.
 */
export function cardEntityForFilter(
	resolved: ElementRenderContext,
	entityType: string
): NodeEntity | undefined {
	if (resolved.elementType === entityType) return elementEntity(resolved);
	if (
		entityType === 'Host' &&
		(resolved.elementType === 'IPAddress' || resolved.elementType === 'Interface')
	) {
		return resolved.host;
	}
	return undefined;
}

/**
 * The entity a container stands for (a host box, a subnet box), found by its `container_type`
 * and `entity_id`. Grouping containers (categories, tags, stacks) have no entity collection and
 * resolve to `undefined`.
 */
export function containerEntity(
	node: TopologyNode,
	topology: RenderableTopology
): NodeEntity | undefined {
	if (node.node_type !== 'Container' || !node.container_type) return undefined;
	const entityId = node.entity_id ?? node.id;
	return entityCollection(topology, node.container_type)?.find((e) => e.id === entityId) as
		NodeEntity | undefined;
}
