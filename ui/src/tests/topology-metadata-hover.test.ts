import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { matchesHoveredMetadata, type HoveredMetadata } from '$lib/features/topology/interactions';
import {
	containerEntity,
	elementEntity,
	resolveElementNode
} from '$lib/features/topology/resolvers';
import type { RenderableTopology, TopologyNode } from '$lib/features/topology/types/base';
import type { Site } from '$lib/features/sites/types';

const HOUR_MS = 60 * 60 * 1000;
const NOW = new Date('2026-07-22T12:00:00Z').getTime();
const SITE_ID = 'net-1';
const site = { id: SITE_ID, effective_stale_after_hours: 24 * 28 } as Site;
const discovery = { type: 'Discovery' };

const seenHoursAgo = (h: number) => new Date(NOW - h * HOUR_MS).toISOString();

function hover(
	entityType: HoveredMetadata['entityTypes'][number],
	filterType: string,
	valueId: string
): HoveredMetadata {
	return { entityTypes: [entityType], filterType, valueId, color: 'Amber' };
}

/**
 * One stale and one fresh host, drawn as Workloads host boxes, with an L3 subnet box, an IP card
 * and an L2 port card whose link is recorded only on the far side.
 */
function buildTopology(): RenderableTopology {
	return {
		id: 'topo-1',
		site_id: SITE_ID,
		hosts: [
			{
				id: 'stale-host',
				site_id: SITE_ID,
				last_seen_at: seenHoursAgo(24 * 45),
				source: discovery,
				tags: []
			},
			{
				id: 'fresh-host',
				site_id: SITE_ID,
				last_seen_at: seenHoursAgo(1),
				source: discovery,
				tags: []
			}
		],
		subnets: [
			{
				id: 'subnet-1',
				site_id: SITE_ID,
				last_seen_at: seenHoursAgo(1),
				source: discovery,
				tags: []
			}
		],
		ip_addresses: [
			{
				id: 'ip-stale',
				host_id: 'stale-host',
				site_id: SITE_ID,
				last_seen_at: seenHoursAgo(24 * 45)
			}
		],
		interfaces: [
			{ id: 'if-1', host_id: 'fresh-host', site_id: SITE_ID, last_seen_at: seenHoursAgo(1) }
		],
		neighbours: [
			{ id: 'row-1', interface_id: 'if-2', neighbor: { type: 'Interface', id: 'if-1' } }
		],
		services: [],
		ports: [],
		bindings: [],
		dependencies: [],
		vlans: [],
		edges: [],
		entity_tags: [],
		nodes: [
			{
				id: 'c-stale-host',
				node_type: 'Container',
				container_type: 'Host',
				entity_id: 'stale-host'
			},
			{
				id: 'c-fresh-host',
				node_type: 'Container',
				container_type: 'Host',
				entity_id: 'fresh-host'
			},
			{ id: 'subnet-1', node_type: 'Container', container_type: 'Subnet' },
			{ id: 'c-category', node_type: 'Container', container_type: 'ServiceCategory' },
			{
				id: 'n-ip',
				node_type: 'Element',
				element_type: 'IPAddress',
				host_id: 'stale-host',
				ip_address_id: 'ip-stale'
			},
			{
				id: 'n-if',
				node_type: 'Element',
				element_type: 'Interface',
				host_id: 'fresh-host',
				interface_id: 'if-1'
			}
		]
	} as unknown as RenderableTopology;
}

const node = (topo: RenderableTopology, id: string) =>
	topo.nodes.find((n) => n.id === id) as TopologyNode;

beforeEach(() => {
	vi.useFakeTimers();
	vi.setSystemTime(NOW);
});
afterEach(() => vi.useRealTimers());

describe('the entity a topology node stands for', () => {
	it('resolves a host box by entity_id, a subnet box by its id, and a grouping box to nothing', () => {
		const topo = buildTopology();
		expect(containerEntity(node(topo, 'c-stale-host'), topo)?.id).toBe('stale-host');
		expect(containerEntity(node(topo, 'subnet-1'), topo)?.id).toBe('subnet-1');
		expect(containerEntity(node(topo, 'c-category'), topo)).toBeUndefined();
	});

	it('resolves an IP card to its address and a port card to its interface', () => {
		const topo = buildTopology();
		const ipCard = resolveElementNode('n-ip', node(topo, 'n-ip'), topo);
		const portCard = resolveElementNode('n-if', node(topo, 'n-if'), topo);
		expect(elementEntity(ipCard)?.id).toBe('ip-stale');
		expect(elementEntity(portCard)?.id).toBe('if-1');
	});
});

describe('filter-value hover matching', () => {
	it('matches the stale host box and not the fresh one', () => {
		const topo = buildTopology();
		const stale = hover('Host', 'Staleness', 'stale');
		const staleBox = containerEntity(node(topo, 'c-stale-host'), topo);
		const freshBox = containerEntity(node(topo, 'c-fresh-host'), topo);
		expect(matchesHoveredMetadata(staleBox, 'Host', stale, site, topo)).toBe(true);
		expect(matchesHoveredMetadata(freshBox, 'Host', stale, site, topo)).toBe(false);
	});

	it('matches an IP card on its own staleness', () => {
		const topo = buildTopology();
		const ipCard = elementEntity(resolveElementNode('n-ip', node(topo, 'n-ip'), topo));
		expect(
			matchesHoveredMetadata(
				ipCard,
				'IPAddress',
				hover('IPAddress', 'Staleness', 'stale'),
				site,
				topo
			)
		).toBe(true);
	});

	// LinkState reads the topology's neighbour rows; this port is linked only from the far side.
	it('matches a linked port only when the topology is supplied', () => {
		const topo = buildTopology();
		const port = elementEntity(resolveElementNode('n-if', node(topo, 'n-if'), topo));
		const linked = hover('Interface', 'LinkState', 'Linked');
		expect(matchesHoveredMetadata(port, 'Interface', linked, site, topo)).toBe(true);
		expect(matchesHoveredMetadata(port, 'Interface', linked, site, undefined)).toBe(false);
	});

	it('matches nothing for a grouping box', () => {
		const topo = buildTopology();
		const grouping = containerEntity(node(topo, 'c-category'), topo);
		expect(
			matchesHoveredMetadata(grouping, 'Host', hover('Host', 'Staleness', 'stale'), site, topo)
		).toBe(false);
	});
});
