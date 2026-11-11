import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
	updateTagFilter,
	tagHiddenNodeIds,
	hiddenEntityIds
} from '#lib/features/topology/interactions.ts';
import type { RenderableTopology, Topology } from '#lib/features/topology/types/base.ts';
import { entityBundleFrom, toRenderableTopology } from '#lib/features/topology/enriched.ts';

const SITE_ID = 'net-1';

/**
 * The L2 bundle as it arrives with the link-state filter already applied server-side.
 *
 * The shape that matters, taken from the live database: of the interfaces that survive, some own a
 * resolved row (`p-own-*`) and some are only ever another port's `neighbor_interface_id`
 * (`p-far-*`). The backend judges link state in both directions and stamps it on each interface
 * (`filter_values`), so every survivor arrives `Linked`, the far ends included.
 */
function filteredL2Bundle(): RenderableTopology {
	const iface = (id: string, host: string) => ({
		id,
		host_id: host,
		site_id: SITE_ID,
		tags: [],
		filter_values: { LinkState: 'Linked' }
	});
	const node = (id: string, host: string) => ({
		id,
		node_type: 'Element',
		element_type: 'Interface',
		interface_id: id,
		entity_id: id,
		host_id: host
	});

	return {
		id: 'topo-1',
		site_id: SITE_ID,
		hosts: [
			{ id: 'switch-a', site_id: SITE_ID, tags: [] },
			{ id: 'switch-b', site_id: SITE_ID, tags: [] }
		],
		interfaces: [
			iface('p-own-1', 'switch-a'),
			iface('p-own-2', 'switch-a'),
			iface('p-far-1', 'switch-b'),
			iface('p-far-2', 'switch-b')
		],
		neighbours: [
			{
				id: 'r1',
				interface_id: 'p-own-1',
				neighbor: { type: 'Interface', id: 'p-far-1' }
			},
			{
				id: 'r2',
				interface_id: 'p-own-2',
				neighbor: { type: 'Interface', id: 'p-far-2' }
			}
		],
		filtered_out: { Interface: { LinkState: 103 } },
		nodes: [
			node('p-own-1', 'switch-a'),
			node('p-own-2', 'switch-a'),
			node('p-far-1', 'switch-b'),
			node('p-far-2', 'switch-b')
		],
		edges: [],
		services: [],
		subnets: [],
		ip_addresses: [],
		ports: [],
		bindings: [],
		dependencies: [],
		vlans: [],
		entity_tags: [],
		name: 'My Site'
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
	} as any;
}

beforeEach(() => {
	tagHiddenNodeIds.set(new Set());
	hiddenEntityIds.set(new Set());
});

describe('hiding unlinked interfaces in L2', () => {
	/**
	 * The reported symptom: hide `Unlinked` and every interface disappears, leaving empty host
	 * boxes. The server has already removed the unlinked ports by the time this runs, so the
	 * client pass must find nothing left to hide — if it hides anything here it is hiding a port
	 * that is linked.
	 */
	it('hides nothing, because the server already removed the unlinked ports', () => {
		updateTagFilter(
			filteredL2Bundle(),
			undefined,
			'L2Physical',
			{ Interface: { LinkState: ['Unlinked'] } },
			[],
			undefined
		);

		expect([...get(tagHiddenNodeIds)]).toEqual([]);
		expect([...get(hiddenEntityIds)]).toEqual([]);
	});

	/**
	 * End to end from the API response rather than from a hand-built topology.
	 *
	 * The topology tab once mapped the `TopologyData` response into its entity bundle field by
	 * field and dropped one, which emptied every host container in L2. A port that loses its
	 * stamped `filter_values` in that mapping carries no link state and the filter misjudges it;
	 * the tests above build the topology directly and so could never see a field lost there.
	 */
	it('keeps linked ports when the topology is built from the API response', () => {
		const built = filteredL2Bundle();
		const response = {
			hosts: built.hosts,
			services: built.services,
			subnets: built.subnets,
			ip_addresses: built.ip_addresses,
			ports: built.ports,
			bindings: built.bindings,
			interfaces: built.interfaces,
			neighbours: built.neighbours,
			filtered_out: built.filtered_out,
			dependencies: built.dependencies,
			vlans: built.vlans,
			tags: [],
			nodes: { L2Physical: built.nodes },
			edges: { L2Physical: [] }
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
		} as any;
		const topology = toRenderableTopology(
			{ id: 'topo-1', site_id: SITE_ID } as Topology,
			entityBundleFrom(response),
			'My Site',
			'L2Physical'
		);

		updateTagFilter(
			topology,
			undefined,
			'L2Physical',
			{ Interface: { LinkState: ['Unlinked'] } },
			[],
			undefined
		);

		expect([...get(tagHiddenNodeIds)]).toEqual([]);

		updateTagFilter(
			topology,
			undefined,
			'L2Physical',
			{ Interface: { LinkState: ['Linked'] } },
			[],
			undefined
		);
		expect(get(tagHiddenNodeIds).size).toBe(4);
	});

	/**
	 * The complement, so the tests above cannot pass by the filter simply never running: hiding
	 * `Linked` on the same bundle must remove every one of them.
	 */
	it('hides every port when Linked is the hidden value', () => {
		updateTagFilter(
			filteredL2Bundle(),
			undefined,
			'L2Physical',
			{ Interface: { LinkState: ['Linked'] } },
			[],
			undefined
		);

		expect(get(tagHiddenNodeIds).size).toBe(4);
	});
});
