import { describe, it, expect } from 'vitest';
import type { RenderableTopology } from '$lib/features/topology/types/base';
import { getTopologyIndex } from '$lib/features/topology/entity-index';
import { resolveInlineServiceIds } from '$lib/features/topology/resolvers';

/**
 * These lock the relationship rules the entity index has to preserve now that
 * container traversal and service-by-host lookup go through prebuilt maps
 * instead of scanning `topology.nodes` / `topology.services` per element.
 *
 * The interesting behaviour is the binding wildcard (`ip_address_id: null` means
 * "every IP on this host") and the rule that a service which is already its own
 * element node must not also be counted as inlined.
 */

const SITE_ID = 'net-1';

function service(id: string, hostId: string, bindingIpIds: (string | null)[]) {
	return {
		id,
		host_id: hostId,
		site_id: SITE_ID,
		tags: [],
		bindings: bindingIpIds.map((ip) => ({ ip_address_id: ip }))
	};
}

/**
 * One container holding a subcontainer. The outer container holds the IP element
 * for host-a; the subcontainer holds an IP element for host-b and a Service
 * element that stands on its own.
 */
function buildTopology(): RenderableTopology {
	return {
		id: 'topo-1',
		site_id: SITE_ID,
		hosts: [
			{ id: 'host-a', site_id: SITE_ID, tags: [] },
			{ id: 'host-b', site_id: SITE_ID, tags: [] }
		],
		subnets: [{ id: 'subnet-1', site_id: SITE_ID, cidr: '10.0.0.0/24', tags: [] }],
		ip_addresses: [
			{ id: 'ip-a1', site_id: SITE_ID, subnet_id: 'subnet-1' },
			{ id: 'ip-a2', site_id: SITE_ID, subnet_id: 'subnet-1' },
			{ id: 'ip-b1', site_id: SITE_ID, subnet_id: 'subnet-1' }
		],
		services: [
			// Bound to one specific IP on host-a.
			service('svc-pinned', 'host-a', ['ip-a1']),
			// Wildcard: no IP, so it applies to every IP element on host-a.
			service('svc-wildcard', 'host-a', [null]),
			// Bound to an IP that host-a's element does not depict.
			service('svc-other-ip', 'host-a', ['ip-a2']),
			// Owns its own element node, so it must not be reported as inlined.
			service('svc-standalone', 'host-b', [null])
		],
		ports: [],
		bindings: [],
		interfaces: [],
		dependencies: [],
		vlans: [],
		entity_tags: [],
		name: 'test',
		nodes: [
			{ id: 'container-outer', node_type: 'Container', container_type: 'Subnet' },
			{
				id: 'container-inner',
				node_type: 'Container',
				container_type: 'Stack',
				parent_container_id: 'container-outer'
			},
			{
				id: 'el-a1',
				node_type: 'Element',
				element_type: 'IPAddress',
				host_id: 'host-a',
				ip_address_id: 'ip-a1',
				container_id: 'container-outer'
			},
			{
				id: 'el-b1',
				node_type: 'Element',
				element_type: 'IPAddress',
				host_id: 'host-b',
				ip_address_id: 'ip-b1',
				container_id: 'container-inner'
			},
			{
				id: 'svc-standalone',
				node_type: 'Element',
				element_type: 'Service',
				host_id: 'host-b',
				container_id: 'container-inner'
			}
		],
		edges: []
	} as unknown as RenderableTopology;
}

describe('topology entity index', () => {
	it('rolls subcontainer contents up into the parent container', () => {
		const topology = buildTopology();
		const contents = getTopologyIndex(topology).containerContents('container-outer');

		expect(contents.subcontainerIds).toEqual(new Set(['container-inner']));
		// el-b1 and svc-standalone live in the subcontainer but belong to the
		// outer container's tally.
		expect(contents.elementNodeIds).toEqual(new Set(['el-a1', 'el-b1', 'svc-standalone']));
		expect(contents.hostIds).toEqual(new Set(['host-a', 'host-b']));
		expect(contents.serviceIds).toEqual(new Set(['svc-standalone']));
	});

	it('scopes a container to its own direct children', () => {
		const topology = buildTopology();
		const contents = getTopologyIndex(topology).containerContents('container-inner');

		expect(contents.elementNodeIds).toEqual(new Set(['el-b1', 'svc-standalone']));
		expect(contents.subcontainerIds.size).toBe(0);
	});

	it('inlines wildcard-bound services onto every IP element of their host', () => {
		const topology = buildTopology();
		const inlined = resolveInlineServiceIds(new Set(['el-a1']), topology);

		expect(inlined.has('svc-pinned')).toBe(true);
		expect(inlined.has('svc-wildcard')).toBe(true);
		// Bound to a different IP on the same host — not inlined here.
		expect(inlined.has('svc-other-ip')).toBe(false);
	});

	it('does not inline a service that is already its own element node', () => {
		const topology = buildTopology();
		const inlined = resolveInlineServiceIds(new Set(['el-b1', 'svc-standalone']), topology);

		// svc-standalone is wildcard-bound on host-b, so it would match el-b1 —
		// but it has its own element node and is counted there instead.
		expect(inlined.has('svc-standalone')).toBe(false);
	});

	it('returns a stable index per topology object and a fresh one after a change', () => {
		const topology = buildTopology();
		expect(getTopologyIndex(topology)).toBe(getTopologyIndex(topology));
		// A new topology object (what enriched.ts produces on any data change)
		// must not reuse the previous index.
		expect(getTopologyIndex(buildTopology())).not.toBe(getTopologyIndex(topology));
	});
});
