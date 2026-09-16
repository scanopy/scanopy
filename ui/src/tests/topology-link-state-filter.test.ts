import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
	updateTagFilter,
	tagHiddenNodeIds,
	hiddenEntityIds
} from '$lib/features/topology/interactions';
import type { RenderableTopology, Topology } from '$lib/features/topology/types/base';
import { entityBundleFrom, toRenderableTopology } from '$lib/features/topology/enriched';

const NETWORK_ID = 'net-1';

/**
 * The L2 bundle as it arrives with the link-state filter already applied server-side.
 *
 * The shape that matters, taken from the live database: of the interfaces that survive, some own a
 * resolved row (`p-own-*`) and some are only ever another port's `neighbor_interface_id`
 * (`p-far-*`) — 50 and 18 respectively on the network this was traced against. The far ends are
 * the fragile half: nothing points *from* them, so any pass that judges link state on the outbound
 * direction alone marks them Unlinked and the filter then deletes one end of every link.
 *
 * `neighbours` is not filtered by the server, so it still carries every resolved row.
 */
function filteredL2Bundle(): RenderableTopology {
	const iface = (id: string, host: string) => ({
		id,
		host_id: host,
		network_id: NETWORK_ID,
		tags: []
	});
	const node = (id: string, host: string) => ({
		id,
		node_type: 'Element',
		element_type: 'Interface',
		interface_id: id,
		host_id: host
	});

	return {
		id: 'topo-1',
		network_id: NETWORK_ID,
		hosts: [
			{ id: 'switch-a', network_id: NETWORK_ID, tags: [] },
			{ id: 'switch-b', network_id: NETWORK_ID, tags: [] }
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
		candidates: [],
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
		name: 'My Network'
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

	/** The far ends specifically — the half a one-directional judgement deletes. */
	it('keeps a port that is only ever named as another port’s neighbour', () => {
		updateTagFilter(
			filteredL2Bundle(),
			undefined,
			'L2Physical',
			{ Interface: { LinkState: ['Unlinked'] } },
			[],
			undefined
		);

		expect(get(tagHiddenNodeIds).has('p-far-1')).toBe(false);
		expect(get(tagHiddenNodeIds).has('p-far-2')).toBe(false);
	});

	/**
	 * The reported bug, end to end from the API response rather than from a hand-built topology.
	 *
	 * The topology tab mapped the `TopologyData` response into its entity bundle field by field and
	 * left `neighbours` out. With no neighbour rows every interface classified `Unlinked`, the hide
	 * pass removed all of them, their subcontainers were pruned as empty, and L2 showed host boxes
	 * with nothing inside. The tests above build the topology directly and so could never see a
	 * field lost in that mapping — this one goes through it.
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
			candidates: built.candidates,
			filtered_out: built.filtered_out,
			dependencies: built.dependencies,
			vlans: built.vlans,
			tags: [],
			nodes: { L2Physical: built.nodes },
			edges: { L2Physical: [] }
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
		} as any;
		const topology = toRenderableTopology(
			{ id: 'topo-1', network_id: NETWORK_ID } as Topology,
			entityBundleFrom(response),
			'My Network',
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

		expect(topology.neighbours).toHaveLength(2);
		expect([...get(tagHiddenNodeIds)]).toEqual([]);
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
