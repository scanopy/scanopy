import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { get } from 'svelte/store';
import {
	updateTagFilter,
	tagHiddenNodeIds,
	hiddenEntityIds,
	presentFilterValues
} from '$lib/features/topology/interactions';
import type { RenderableTopology } from '$lib/features/topology/types/base';
import type { Network } from '$lib/features/networks/types';

const HOUR_MS = 60 * 60 * 1000;
const NOW = new Date('2026-07-22T12:00:00Z').getTime();
const NETWORK_ID = 'net-1';

const network = { id: NETWORK_ID, effective_stale_after_hours: 24 * 28 } as Network;

function seenHoursAgo(h: number) {
	return new Date(NOW - h * HOUR_MS).toISOString();
}

/**
 * Two hosts, each with one service:
 *  - stale-host: the HOST is past the window while its service was seen
 *    recently — the case that must NOT drag the service down with it.
 *  - fresh-host: the host is current but its service is past the window.
 */
function buildTopology(): RenderableTopology {
	const discovery = { type: 'Discovery' };
	return {
		id: 'topo-1',
		network_id: NETWORK_ID,
		hosts: [
			{
				id: 'stale-host',
				network_id: NETWORK_ID,
				last_seen_at: seenHoursAgo(24 * 45),
				source: discovery,
				tags: []
			},
			{
				id: 'fresh-host',
				network_id: NETWORK_ID,
				last_seen_at: seenHoursAgo(1),
				source: discovery,
				tags: []
			}
		],
		services: [
			{
				id: 'svc-on-stale-host',
				host_id: 'stale-host',
				network_id: NETWORK_ID,
				last_seen_at: seenHoursAgo(1),
				source: discovery,
				tags: []
			},
			{
				id: 'svc-itself-stale',
				host_id: 'fresh-host',
				network_id: NETWORK_ID,
				last_seen_at: seenHoursAgo(24 * 45),
				source: discovery,
				tags: []
			}
		],
		nodes: [
			{
				id: 'svc-on-stale-host',
				node_type: 'Element',
				element_type: 'Service',
				host_id: 'stale-host'
			},
			{
				id: 'svc-itself-stale',
				node_type: 'Element',
				element_type: 'Service',
				host_id: 'fresh-host'
			},
			{ id: 'stale-host', node_type: 'Element', element_type: 'Host', host_id: 'stale-host' },
			{ id: 'fresh-host', node_type: 'Element', element_type: 'Host', host_id: 'fresh-host' }
		],
		edges: [],
		subnets: [],
		ip_addresses: [],
		ports: [],
		bindings: [],
		interfaces: [],
		dependencies: [],
		vlans: [],
		entity_tags: []
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
	} as any;
}

beforeEach(() => {
	vi.useFakeTimers();
	vi.setSystemTime(NOW);
	tagHiddenNodeIds.set(new Set());
	hiddenEntityIds.set(new Set());
});
afterEach(() => vi.useRealTimers());

describe('topology staleness filter', () => {
	// The two spaces the filter resolves into. Before these were unified, entity-space was a
	// Service-only store and every pass had to special-case `entityType === 'Service'`.
	it('records a hidden entity in entity-space and its node in node-space', () => {
		updateTagFilter(
			buildTopology(),
			undefined,
			'Workloads',
			{ Service: { Staleness: ['stale'] } },
			[],
			network
		);
		// It is an element node, so it resolves to node-space by id...
		expect(get(tagHiddenNodeIds).has('svc-itself-stale')).toBe(true);
		// ...and it is an entity either way, so inline render gates see it too.
		expect(get(hiddenEntityIds).has('svc-itself-stale')).toBe(true);
	});

	it('hides an entity that has no node of its own', () => {
		// A Port renders only inline on another node's card. There is nothing to remove from the
		// graph, so node-space stays empty — but the render gate still has to know, which is the
		// case the old Service-only store could not express at all.
		const topo = buildTopology();
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		(topo as any).ports = [
			{
				id: 'port-1',
				network_id: NETWORK_ID,
				host_id: 'fresh-host',
				source: { type: 'Discovery' },
				tags: []
			}
		];

		updateTagFilter(topo, undefined, 'Workloads', undefined, ['Port'], network);

		expect(get(hiddenEntityIds).has('port-1')).toBe(true);
		expect(get(tagHiddenNodeIds).has('port-1')).toBe(false);
	});

	it('hides a service that is stale on its own merits', () => {
		updateTagFilter(
			buildTopology(),
			undefined,
			'Workloads',
			{ Service: { Staleness: ['stale'] } },
			[],
			network
		);
		expect(get(tagHiddenNodeIds).has('svc-itself-stale')).toBe(true);
	});

	// No inheritance in the UI: a service seen recently stays visible even when
	// its host is long stale, matching what the inventory shows for it.
	it('leaves a recently-seen service visible under a stale host', () => {
		updateTagFilter(
			buildTopology(),
			undefined,
			'Workloads',
			{ Service: { Staleness: ['stale'] } },
			[],
			network
		);
		expect(get(tagHiddenNodeIds).has('svc-on-stale-host')).toBe(false);
	});

	// Hiding Current is the mirror image, and confirms the two values partition
	// the set rather than both keying off "stale".
	it('hides exactly the current services when Current is the hidden value', () => {
		updateTagFilter(
			buildTopology(),
			undefined,
			'Workloads',
			{ Service: { Staleness: ['current'] } },
			[],
			network
		);
		const hidden = get(tagHiddenNodeIds);
		expect(hidden.has('svc-on-stale-host')).toBe(true);
		expect(hidden.has('svc-itself-stale')).toBe(false);
	});

	it('hides stale host element nodes', () => {
		updateTagFilter(
			buildTopology(),
			undefined,
			'Workloads',
			{ Host: { Staleness: ['stale'] } },
			[],
			network
		);
		const hidden = get(tagHiddenNodeIds);
		expect(hidden.has('stale-host')).toBe(true);
		expect(hidden.has('fresh-host')).toBe(false);
	});

	// A filter whose entities all share one value can only show everything or
	// hide everything, so the panel drops the group. Both values present here.
	it('reports both staleness values as present when the set is mixed', () => {
		updateTagFilter(buildTopology(), undefined, 'Workloads', {}, [], network);
		const present = get(presentFilterValues).Service?.Staleness ?? [];
		expect([...present].sort()).toEqual(['current', 'stale']);
	});

	it('reports a single value when every entity of the type agrees', () => {
		const topo = buildTopology();
		// Make both services stale, so the filter offers no discrimination.
		topo.services.forEach((s) => {
			(s as { last_seen_at: string }).last_seen_at = seenHoursAgo(24 * 45);
		});
		updateTagFilter(topo, undefined, 'Workloads', {}, [], network);
		expect(get(presentFilterValues).Service?.Staleness).toEqual(['stale']);
	});

	// Without the network the window is unknown, so nothing can be judged —
	// the filter must not silently hide (or keep) everything.
	it('hides nothing when the network is not supplied', () => {
		updateTagFilter(
			buildTopology(),
			undefined,
			'Workloads',
			{ Service: { Staleness: ['stale'] } },
			[],
			undefined
		);
		expect(get(tagHiddenNodeIds).size).toBe(0);
	});
});

/**
 * Server-side filters remove their entities from the response, so the panel can no longer see the
 * hidden value represented in the topology. `presentFilterValues` drops a filter group whose
 * entities all share one value — which, unguarded, would delete the only control capable of
 * bringing those entities back.
 */
describe('hidden values stay offerable once the server stops sending them', () => {
	it('keeps a hidden value represented even when no entity carries it', () => {
		// One interface, linked via its own resolved-adjacency row (GH #701: a port's neighbour is
		// a `Vec` on the topology bundle, not a scalar field on the interface). `Unlinked` is
		// hidden, so the server sent none — exactly the state after a server-side LinkState filter
		// runs.
		const topology = {
			id: 'topo-1',
			nodes: [],
			edges: [],
			hosts: [],
			services: [],
			ip_addresses: [],
			subnets: [],
			interfaces: [{ id: 'if-1' }],
			neighbours: [
				{ id: 'row-1', interface_id: 'if-1', neighbor: { type: 'Interface', id: 'if-2' } }
			]
		} as unknown as RenderableTopology;

		updateTagFilter(topology, undefined, 'L2Physical', {
			Interface: { LinkState: ['Unlinked'] }
		});

		const present = get(presentFilterValues);
		expect(
			present.Interface?.LinkState ?? [],
			'a hidden value must stay offerable, or the user cannot un-hide it'
		).toContain('Unlinked');
	});
});
