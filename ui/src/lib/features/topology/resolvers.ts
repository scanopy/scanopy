import type { components } from '#lib/api/schema';
import type { ElementNode, RenderableTopology, TopologyEdge, TopologyNode } from './types/base';
import { containerTypes, entities } from '#lib/shared/stores/metadata.ts';
import {
	collectionLookup,
	type EntityLookup,
	type EntityType
} from '#lib/shared/entity-fields/index.ts';
import { hostDisplayName } from '#lib/features/hosts/host-display-name.ts';
import { getTopologyIndex } from './entity-index';
import type { FreshnessSubject } from '#lib/shared/utils/freshness.ts';
import type { Interface } from '#lib/features/hosts/types/base.ts';
import type { InterfaceRelations } from '#lib/features/hosts/components/InterfaceRelationRows.svelte';

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
	Vlan: 'vlans',
	Tag: 'entity_tags'
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
}

export interface ContainerRenderContext {
	tags: string[];
	title: string | null;
	containerType: string;
}

// Exhaustive resolver maps — TypeScript errors if a variant is missing. An element's own entity
// comes from its `entity_id`, its host from `host_id`.
const elementResolvers: Record<
	ElementEntityTypeDiscriminant,
	(nodeId: string, node: ElementNode, topology: RenderableTopology) => ElementRenderContext
> = {
	IPAddress: (_nodeId, node, topology) => {
		const index = getTopologyIndex(topology);
		const hostId = node.host_id;
		const ipAddressId = node.entity_id ?? undefined;
		const subnetId = 'subnet_id' in node ? node.subnet_id : '';

		const host = index.hostsById.get(hostId);
		const ipAddress = ipAddressId ? index.ipAddressesById.get(ipAddressId) : undefined;
		// The host's services bound to this address or to every address.
		const services = (index.servicesByHostId.get(hostId) ?? []).filter((s) =>
			s.bindings.some((b) => b.ip_address_id === ipAddressId || b.ip_address_id === null)
		);

		return {
			elementType: 'IPAddress',
			host,
			ipAddress,
			snmpInterface: undefined,
			services,
			hostId,
			ipAddressId,
			interfaceId: undefined,
			subnetId
		};
	},
	Service: (_nodeId, node, topology) => {
		const index = getTopologyIndex(topology);
		const hostId = node.host_id;
		const service = node.entity_id ? index.servicesById.get(node.entity_id) : undefined;

		return {
			elementType: 'Service',
			host: index.hostsById.get(hostId),
			ipAddress: undefined,
			snmpInterface: undefined,
			services: service ? [service] : [],
			hostId,
			ipAddressId: undefined,
			interfaceId: undefined,
			subnetId: ''
		};
	},
	Host: (_nodeId, node, topology) => {
		const index = getTopologyIndex(topology);
		const hostId = node.host_id;

		return {
			elementType: 'Host',
			host: index.hostsById.get(hostId),
			ipAddress: undefined,
			snmpInterface: undefined,
			services: index.servicesByHostId.get(hostId) ?? [],
			hostId,
			ipAddressId: undefined,
			interfaceId: undefined,
			subnetId: ''
		};
	},
	Interface: (_nodeId, node, topology) => {
		const index = getTopologyIndex(topology);
		const hostId = node.host_id;
		const interfaceId = node.entity_id ?? undefined;

		return {
			elementType: 'Interface',
			host: index.hostsById.get(hostId),
			ipAddress: undefined,
			snmpInterface: interfaceId ? index.interfacesById.get(interfaceId) : undefined,
			services: [],
			hostId,
			ipAddressId: undefined,
			interfaceId,
			subnetId: ''
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
 * and then to the container's contents for legacy/indirect resolution.
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

// Resolve the taggable entity behind an element node: the element's own entity when its type is
// taggable, otherwise the first taggable entity up its parent_taggable_entity chain (per entity
// metadata). An element names one ancestor, its host, so only a Host ancestor resolves. Returns
// null for containers, unknown elements, or chains with no taggable ancestor.
export interface TagTarget {
	entityType: EntityType;
	entityId: string;
}

export function resolveTagTarget(node: TopologyNode): TagTarget | null {
	if (node.node_type !== 'Element' || !node.element_type) return null;
	const elementType = node.element_type as EntityType;
	if (entities.getMetadata(elementType).is_taggable) {
		return node.entity_id ? { entityType: elementType, entityId: node.entity_id } : null;
	}

	let type = entities.getMetadata(elementType).parent_taggable_entity;
	while (type) {
		const meta = entities.getMetadata(type);
		if (meta.is_taggable) {
			return type === 'Host' && node.host_id
				? { entityType: 'Host', entityId: node.host_id }
				: null;
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
	/** Every element node on a host, whatever its type: a host's fan-out. */
	hostIdToNodes: Map<string, string[]>;
	/** Element nodes keyed by the entity they stand for (`entity_id`). */
	entityIdToNodes: Map<string, string[]>;
	/** Container nodes keyed by the entity they stand for (`entity_id`). */
	entityIdToContainerIds: Map<string, Set<string>>;
	allElementNodeIds: Set<string>;
	allContainerNodeIds: Set<string>;
}

function pushNode(map: Map<string, string[]>, key: string, nodeId: string): void {
	const existing = map.get(key);
	if (existing) existing.push(nodeId);
	else map.set(key, [nodeId]);
}

/**
 * Build an index mapping entity IDs to the topology node IDs that represent them.
 * Single pass over topology.nodes. Use this instead of ad-hoc entity→node lookups.
 */
export function buildEntityNodeIndex(nodes: TopologyNode[]): EntityNodeIndex {
	const hostIdToNodes = new Map<string, string[]>();
	const entityIdToNodes = new Map<string, string[]>();
	const entityIdToContainerIds = new Map<string, Set<string>>();
	const allElementNodeIds = new Set<string>();
	const allContainerNodeIds = new Set<string>();

	for (const nd of nodes) {
		if (nd.node_type === 'Container') {
			allContainerNodeIds.add(nd.id);
			if (nd.entity_id) {
				const containerSet = entityIdToContainerIds.get(nd.entity_id);
				if (containerSet) containerSet.add(nd.id);
				else entityIdToContainerIds.set(nd.entity_id, new Set([nd.id]));
			}
			continue;
		}
		if (nd.node_type !== 'Element') continue;

		allElementNodeIds.add(nd.id);
		if (nd.host_id) pushNode(hostIdToNodes, nd.host_id, nd.id);
		if (nd.entity_id) pushNode(entityIdToNodes, nd.entity_id, nd.id);
	}

	return {
		hostIdToNodes,
		entityIdToNodes,
		entityIdToContainerIds,
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
			subnetId: ''
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

/** An entity a topology node stands for, as the metadata filters and freshness helpers read it. */
export type NodeEntity = FreshnessSubject & { id: string; site_id?: string };

/** The host an element card sits on, from its `host_id`. */
function elementHost(node: TopologyNode, topology: RenderableTopology): NodeEntity | undefined {
	if (node.node_type !== 'Element' || !node.host_id) return undefined;
	return getTopologyIndex(topology).hostsById.get(node.host_id);
}

/**
 * The entity an element card depicts: its own entity, and otherwise its host.
 * One owner for the card's stale pill and its filter-hover match, so the two cannot disagree.
 */
export function elementEntity(
	node: TopologyNode,
	topology: RenderableTopology
): NodeEntity | undefined {
	return nodeEntity(node, topology)?.entity ?? elementHost(node, topology);
}

/**
 * The entity a card is judged by for a filter on `entityType`: its own when the filter is on its
 * type, its host when the filter is on its type's parent taggable entity (a Host filter meeting an
 * IPAddress or Interface card). The filter-value hover ring and the view's element marks both read
 * it, so a coloured card is always one the matching chip rings.
 */
export function cardEntityForFilter(
	node: TopologyNode,
	topology: RenderableTopology,
	entityType: string
): NodeEntity | undefined {
	if (node.node_type !== 'Element') return undefined;
	if (node.element_type === entityType) return elementEntity(node, topology);
	if (entities.getMetadata(node.element_type).parent_taggable_entity === entityType) {
		return elementHost(node, topology);
	}
	return undefined;
}

/**
 * The entity a container stands for (a host box, a subnet box, a VLAN box, a hypervisor's
 * service), with its type. Grouping containers (categories, tags, stacks) resolve to `null`.
 */
export function containerEntity(
	node: TopologyNode,
	topology: RenderableTopology
): { entityType: EntityType; entity: NodeEntity } | null {
	if (node.node_type !== 'Container') return null;
	return nodeEntity(node, topology);
}

/** Id-keyed views of entity collections, memoized on the collection array's identity. */
const collectionsById = new WeakMap<object, Map<string, { id: string }>>();

function entityById(
	topology: RenderableTopology,
	entityType: string,
	id: string
): { id: string } | undefined {
	const collection = entityCollection(topology, entityType);
	if (!collection) return undefined;
	let byId = collectionsById.get(collection);
	if (!byId) {
		byId = new Map(collection.map((e) => [e.id, e]));
		collectionsById.set(collection, byId);
	}
	return byId.get(id);
}

/**
 * The entity a node stands for, from what the backend put on it: an element's `element_type` and
 * `entity_id`, or a container's `entity_id` and its container type's `entity_type` metadata.
 * `null` for grouping containers and for an element whose entity this topology doesn't hold.
 *
 * The single lookup from a drawn shape to what it describes; new consumers use this rather than
 * switching on element or container types.
 */
export function nodeEntity(
	node: TopologyNode,
	topology: RenderableTopology
): { entityType: EntityType; entity: NodeEntity } | null {
	const entityType: EntityType | null | undefined =
		node.node_type === 'Element'
			? (node.element_type as EntityType)
			: containerTypes.getMetadata(node.container_type ?? null).entity_type;
	if (!entityType || !node.entity_id) return null;
	const entity = entityById(topology, entityType, node.entity_id);
	return entity ? { entityType, entity: entity as NodeEntity } : null;
}

/** An `EntityLookup` over a topology's entity collections, for resolving registry fields. */
export function topologyLookup(topology: RenderableTopology): EntityLookup {
	const collections = Object.fromEntries(
		Object.keys(ENTITY_COLLECTIONS).map((type) => [type, entityCollection(topology, type)])
	);
	return collectionLookup(collections);
}

/**
 * The entities an interface links to on this topology: its IP address and that address's subnet,
 * its tagged VLANs, and its resolved neighbours (GH #701: a port can have several).
 */
export function interfaceRelations(
	iface: Interface,
	topology: RenderableTopology
): Required<InterfaceRelations> {
	const linkedIpAddress = iface.ip_address_id
		? (topology.ip_addresses.find((i) => i.id === iface.ip_address_id) ?? null)
		: null;
	const linkedSubnet = linkedIpAddress
		? (topology.subnets.find((s) => s.id === linkedIpAddress.subnet_id) ?? null)
		: null;
	const neighbours = (topology.neighbours ?? [])
		.filter((n) => n.interface_id === iface.id)
		.map((row) => {
			if (row.neighbor.type === 'Host') {
				return {
					id: row.id,
					neighborHost: topology.hosts.find((h) => h.id === row.neighbor.id) ?? null,
					neighborInterface: null
				};
			}
			const remote = topology.interfaces.find((e) => e.id === row.neighbor.id) ?? null;
			return {
				id: row.id,
				neighborInterface: remote,
				neighborHost: remote ? (topology.hosts.find((h) => h.id === remote.host_id) ?? null) : null
			};
		});
	const taggedVlans = (iface.vlan_ids ?? []).flatMap((id) => {
		const vlan = topology.vlans.find((v) => v.id === id);
		return vlan ? [vlan] : [];
	});
	return { linkedIpAddress, linkedSubnet, neighbours, taggedVlans };
}
