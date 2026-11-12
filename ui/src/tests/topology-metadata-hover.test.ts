import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import {
	matchesHoveredMetadata,
	type HoveredMetadata
} from '#lib/features/topology/interactions.ts';
import { containerEntity, elementEntity } from '#lib/features/topology/resolvers.ts';
import type { RenderableTopology, TopologyNode } from '#lib/features/topology/types/base.ts';
import type { Site } from '#lib/features/sites/types.ts';
import { getFreshnessTag } from '#lib/shared/utils/freshness.ts';

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
 * and an L2 port card the backend has stamped as linked.
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
			{
				id: 'if-1',
				host_id: 'fresh-host',
				site_id: SITE_ID,
				last_seen_at: seenHoursAgo(1),
				filter_values: { LinkState: 'Linked' }
			}
		],
		neighbours: [],
		services: [
			{
				id: 'svc-proxmox',
				host_id: 'fresh-host',
				site_id: SITE_ID,
				last_seen_at: seenHoursAgo(24 * 45),
				source: discovery,
				bindings: [],
				tags: []
			}
		],
		ports: [],
		bindings: [],
		dependencies: [],
		vlans: [
			{ id: 'vlan-10', site_id: SITE_ID, last_seen_at: seenHoursAgo(24 * 45), source: discovery }
		],
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
			{ id: 'subnet-1', node_type: 'Container', container_type: 'Subnet', entity_id: 'subnet-1' },
			{ id: 'c-category', node_type: 'Container', container_type: 'ServiceCategory' },
			{ id: 'c-vlan', node_type: 'Container', container_type: 'VLAN', entity_id: 'vlan-10' },
			{
				id: 'c-hypervisor',
				node_type: 'Container',
				container_type: 'Hypervisor',
				entity_id: 'svc-proxmox',
				parent_container_id: 'c-fresh-host'
			},
			{
				id: 'n-ip',
				node_type: 'Element',
				element_type: 'IPAddress',
				host_id: 'stale-host',
				ip_address_id: 'ip-stale',
				entity_id: 'ip-stale'
			},
			{
				id: 'n-if',
				node_type: 'Element',
				element_type: 'Interface',
				host_id: 'fresh-host',
				interface_id: 'if-1',
				entity_id: 'if-1'
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
	it('resolves a host box and a subnet box by entity_id, and a grouping box to nothing', () => {
		const topo = buildTopology();
		expect(containerEntity(node(topo, 'c-stale-host'), topo)?.entity.id).toBe('stale-host');
		expect(containerEntity(node(topo, 'subnet-1'), topo)?.entity.id).toBe('subnet-1');
		expect(containerEntity(node(topo, 'c-category'), topo)).toBeNull();
	});

	it('resolves an IP card to its address and a port card to its interface', () => {
		const topo = buildTopology();
		expect(elementEntity(node(topo, 'n-ip'), topo)?.id).toBe('ip-stale');
		expect(elementEntity(node(topo, 'n-if'), topo)?.id).toBe('if-1');
	});

	it('falls back to the host for an IP card with no known address', () => {
		const topo = buildTopology();
		const card = { ...node(topo, 'n-ip'), entity_id: null } as TopologyNode;
		expect(elementEntity(card, topo)?.id).toBe('stale-host');
	});

	// A VLAN box's container type is 'VLAN' but its entities live under 'Vlan'; the pill has to
	// come from the resolved entity type, not the container type.
	it('gives a VLAN box the stale pill of its VLAN', () => {
		const topo = buildTopology();
		const box = containerEntity(node(topo, 'c-vlan'), topo);
		expect(box?.entityType).toBe('Vlan');
		expect(getFreshnessTag(box!.entity, site)).not.toBeNull();
	});
});

describe('filter-value hover matching', () => {
	it('matches the stale host box and not the fresh one', () => {
		const topo = buildTopology();
		const stale = hover('Host', 'Staleness', 'stale');
		const staleBox = containerEntity(node(topo, 'c-stale-host'), topo);
		const freshBox = containerEntity(node(topo, 'c-fresh-host'), topo);
		expect(matchesHoveredMetadata(staleBox?.entity, 'Host', stale, site)).toBe(true);
		expect(matchesHoveredMetadata(freshBox?.entity, 'Host', stale, site)).toBe(false);
	});

	it('rings a Hypervisor box on its hypervisor service', () => {
		const topo = buildTopology();
		const box = containerEntity(node(topo, 'c-hypervisor'), topo);
		expect(box?.entityType).toBe('Service');
		const stale = hover('Service', 'Staleness', 'stale');
		expect(matchesHoveredMetadata(box?.entity, box!.entityType, stale, site)).toBe(true);
	});

	it('matches an IP card on its own staleness', () => {
		const topo = buildTopology();
		const ipCard = elementEntity(node(topo, 'n-ip'), topo);
		expect(
			matchesHoveredMetadata(ipCard, 'IPAddress', hover('IPAddress', 'Staleness', 'stale'), site)
		).toBe(true);
	});

	// LinkState comes stamped on the interface by the backend, which judges both directions.
	it('matches a port card on the link state stamped on it', () => {
		const topo = buildTopology();
		const port = elementEntity(node(topo, 'n-if'), topo);
		const linked = hover('Interface', 'LinkState', 'Linked');
		const unlinked = hover('Interface', 'LinkState', 'Unlinked');
		expect(matchesHoveredMetadata(port, 'Interface', linked, site)).toBe(true);
		expect(matchesHoveredMetadata(port, 'Interface', unlinked, site)).toBe(false);
	});

	it('matches nothing for a grouping box', () => {
		const topo = buildTopology();
		const grouping = containerEntity(node(topo, 'c-category'), topo);
		expect(
			matchesHoveredMetadata(grouping?.entity, 'Host', hover('Host', 'Staleness', 'stale'), site)
		).toBe(false);
	});
});
