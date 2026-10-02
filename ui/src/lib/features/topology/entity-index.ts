/**
 * Id-keyed lookups over a topology, built once and shared.
 *
 * The resolvers and node components need to go from an id on a graph node to
 * the entity it depicts. They did that with `topology.hosts.find(...)`,
 * `topology.interfaces.find(...)`, `topology.services.filter(...)` and friends —
 * linear scans, run once per node, inside a `$derived`. On a large L2 graph
 * (hundreds of host containers, thousands of interface elements) that is
 * quadratic, and it re-runs whenever the topology object changes.
 *
 * This module builds the maps once per topology and memoizes them on the
 * topology's **object identity**. `enriched.ts` produces a new topology object
 * whenever the underlying query data changes, so identity-keyed memoization
 * invalidates exactly when the data does — there is no cache to bust by hand.
 * A `WeakMap` means a superseded topology's index is collected with it.
 */

import type { RenderableTopology, TopologyNode } from './types/base';
import type { Service } from '$lib/features/services/types/base';
import type { Host, IPAddress, Interface, Port } from '$lib/features/hosts/types/base';
import type { Subnet } from '$lib/features/subnets/types/base';

/** Entities reachable from a container, including nested subcontainers. */
export interface ContainerContents {
	hostIds: Set<string>;
	serviceIds: Set<string>;
	interfaceIds: Set<string>;
	elementNodeIds: Set<string>;
	subcontainerIds: Set<string>;
}

export interface TopologyIndex {
	hostsById: Map<string, Host>;
	ipAddressesById: Map<string, IPAddress>;
	servicesById: Map<string, Service>;
	interfacesById: Map<string, Interface>;
	portsById: Map<string, Port>;
	subnetsById: Map<string, Subnet>;
	nodesById: Map<string, TopologyNode>;

	/** Services on a host, in `topology.services` order. */
	servicesByHostId: Map<string, Service[]>;

	/** Element nodes whose immediate `container_id` is this container. */
	directElementsByContainerId: Map<string, TopologyNode[]>;

	/**
	 * Tags of whatever entity a container can represent, keyed by entity id.
	 * Hosts, subnets and services share one map because `resolveContainerTags`
	 * looks up a container's entity without knowing which type it is.
	 */
	entityTags: Map<string, string[]>;

	/** Memoized `getContainerContents`, keyed by container id. */
	containerContents(containerId: string): ContainerContents;
}

function pushInto<T>(map: Map<string, T[]>, key: string, value: T): void {
	const existing = map.get(key);
	if (existing) existing.push(value);
	else map.set(key, [value]);
}

function buildContainerContents(
	containerId: string,
	childElementsByContainer: Map<string, TopologyNode[]>,
	subcontainersByParent: Map<string, TopologyNode[]>
): ContainerContents {
	const hostIds = new Set<string>();
	const serviceIds = new Set<string>();
	const interfaceIds = new Set<string>();
	const elementNodeIds = new Set<string>();
	const subcontainerIds = new Set<string>();

	for (const sub of subcontainersByParent.get(containerId) ?? []) {
		subcontainerIds.add(sub.id);
	}

	for (const scope of [containerId, ...subcontainerIds]) {
		for (const nd of childElementsByContainer.get(scope) ?? []) {
			// The map only ever holds elements, but narrow so `element_type` is
			// reachable on the union rather than casting it into existence.
			if (nd.node_type !== 'Element') continue;
			elementNodeIds.add(nd.id);

			const hostId = (nd as Record<string, unknown>).host_id as string | undefined;
			if (hostId) hostIds.add(hostId);

			if (nd.element_type === 'Service') {
				serviceIds.add(nd.id);
			} else if (nd.element_type === 'Interface') {
				const ifaceId = (nd as Record<string, unknown>).interface_id as string | undefined;
				if (ifaceId) interfaceIds.add(ifaceId);
			}
			// Host elements contribute only their hostId, added above.
		}
	}

	return { hostIds, serviceIds, interfaceIds, elementNodeIds, subcontainerIds };
}

export function buildTopologyIndex(topology: RenderableTopology): TopologyIndex {
	const hostsById = new Map<string, Host>();
	const ipAddressesById = new Map<string, IPAddress>();
	const servicesById = new Map<string, Service>();
	const interfacesById = new Map<string, Interface>();
	const portsById = new Map<string, Port>();
	const subnetsById = new Map<string, Subnet>();
	const nodesById = new Map<string, TopologyNode>();
	const servicesByHostId = new Map<string, Service[]>();
	const entityTags = new Map<string, string[]>();

	for (const h of topology.hosts) {
		hostsById.set(h.id, h);
		entityTags.set(h.id, h.tags);
	}
	for (const s of topology.subnets) {
		subnetsById.set(s.id, s);
		entityTags.set(s.id, s.tags);
	}
	for (const s of topology.services) {
		servicesById.set(s.id, s);
		entityTags.set(s.id, s.tags);
		if (s.host_id) pushInto(servicesByHostId, s.host_id, s);
	}
	for (const i of topology.ip_addresses) ipAddressesById.set(i.id, i);
	for (const i of topology.interfaces) interfacesById.set(i.id, i);
	for (const p of topology.ports) portsById.set(p.id, p);

	// Parent lookups for container traversal. Elements point up via
	// `container_id`; subcontainers via `parent_container_id`.
	const childElementsByContainer = new Map<string, TopologyNode[]>();
	const subcontainersByParent = new Map<string, TopologyNode[]>();
	for (const nd of topology.nodes) {
		nodesById.set(nd.id, nd);
		if (nd.node_type === 'Element') {
			const parentId = (nd as Record<string, unknown>).container_id as string | undefined;
			if (parentId) pushInto(childElementsByContainer, parentId, nd);
		} else if (nd.node_type === 'Container') {
			const parentId = (nd as Record<string, unknown>).parent_container_id as string | undefined;
			if (parentId) pushInto(subcontainersByParent, parentId, nd);
		}
	}

	const contentsCache = new Map<string, ContainerContents>();

	return {
		hostsById,
		ipAddressesById,
		servicesById,
		interfacesById,
		portsById,
		subnetsById,
		nodesById,
		servicesByHostId,
		directElementsByContainerId: childElementsByContainer,
		entityTags,
		containerContents(containerId: string): ContainerContents {
			const cached = contentsCache.get(containerId);
			if (cached) return cached;
			const built = buildContainerContents(
				containerId,
				childElementsByContainer,
				subcontainersByParent
			);
			contentsCache.set(containerId, built);
			return built;
		}
	};
}

const indexCache = new WeakMap<RenderableTopology, TopologyIndex>();

/**
 * The index for a topology, built on first use.
 *
 * Safe to call in a hot path: repeated calls with the same topology object are
 * a `WeakMap` hit.
 */
export function getTopologyIndex(topology: RenderableTopology): TopologyIndex {
	const cached = indexCache.get(topology);
	if (cached) return cached;
	const built = buildTopologyIndex(topology);
	indexCache.set(topology, built);
	return built;
}
