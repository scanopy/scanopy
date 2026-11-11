import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
	hiddenEntityIds,
	searchMatchNodeIds,
	tagHiddenNodeIds,
	updateSearchFilter,
	updateTagFilter
} from '#lib/features/topology/interactions.ts';
import { getNodeSelectionIds, resolveTagTarget } from '#lib/features/topology/resolvers.ts';
import type { RenderableTopology, TopologyNode } from '#lib/features/topology/types/base.ts';
import type { Site } from '#lib/features/sites/types.ts';

/**
 * Cards whose node ids are not their entities' ids, so every lookup has to go through `entity_id`:
 *  - a Workloads host box for `pve` holding a Hypervisor box for its Proxmox service, with a VM
 *    service card inside that, and a Host card for a guest `vm-host` beside a service on it;
 *  - an L3 subnet box holding an IP card for `pve` and an L2 port card.
 */
const SITE_ID = 'net-1';
const site = { id: SITE_ID, effective_stale_after_hours: 24 } as Site;
const STALE = new Date(Date.now() - 30 * 24 * 60 * 60 * 1000).toISOString();
const discovery = { type: 'Discovery' };

function buildTopology(): RenderableTopology {
	const base = { site_id: SITE_ID, tags: [] as string[], source: discovery };
	const binding = (ip: string | null) => ({ id: `b-${ip}`, ip_address_id: ip, port_id: null });
	return {
		id: 'topo-1',
		site_id: SITE_ID,
		hosts: [
			{ ...base, id: 'pve', name: 'pve-node', display_name: 'pve-node' },
			{ ...base, id: 'vm-host', name: 'guest-vm', display_name: 'guest-vm' },
			{ ...base, id: 'plc', name: '', display_name: null }
		],
		services: [
			{
				...base,
				id: 'svc-proxmox',
				name: 'Proxmox VE',
				host_id: 'pve',
				tags: ['tag-infra'],
				last_seen_at: STALE,
				bindings: [binding('ip-pve')]
			},
			{ ...base, id: 'svc-vm', name: 'web', host_id: 'pve', bindings: [binding(null)] },
			{ ...base, id: 'svc-other-ip', name: 'db', host_id: 'pve', bindings: [binding('ip-x')] },
			{ ...base, id: 'svc-guest', name: 'nginx', host_id: 'vm-host', bindings: [] }
		],
		ip_addresses: [
			{ id: 'ip-pve', host_id: 'pve', site_id: SITE_ID, ip_address: '10.0.0.5', subnet_id: 's1' },
			{ id: 'ip-plc', host_id: 'plc', site_id: SITE_ID, ip_address: '10.0.9.7', subnet_id: 's1' }
		],
		interfaces: [
			{ id: 'if-1', host_id: 'pve', site_id: SITE_ID, if_name: 'eth0' },
			{ id: 'if-2', host_id: 'pve', site_id: SITE_ID, if_descr: 'Port 1/0/8' }
		],
		subnets: [{ ...base, id: 's1', name: 'lan', cidr: '10.0.0.0/24' }],
		ports: [],
		bindings: [],
		dependencies: [],
		vlans: [],
		edges: [],
		entity_tags: [],
		nodes: [
			{ id: 'c-pve', node_type: 'Container', container_type: 'Host', entity_id: 'pve' },
			{
				id: 'c-hypervisor',
				node_type: 'Container',
				container_type: 'Hypervisor',
				entity_id: 'svc-proxmox',
				parent_container_id: 'c-pve'
			},
			{
				id: 'n-svc-vm',
				node_type: 'Element',
				element_type: 'Service',
				host_id: 'pve',
				entity_id: 'svc-vm',
				container_id: 'c-hypervisor'
			},
			{
				id: 'n-guest',
				node_type: 'Element',
				element_type: 'Host',
				host_id: 'vm-host',
				entity_id: 'vm-host',
				container_id: 'c-pve'
			},
			{
				id: 'n-svc-guest',
				node_type: 'Element',
				element_type: 'Service',
				host_id: 'vm-host',
				entity_id: 'svc-guest',
				container_id: 'c-pve'
			},
			{ id: 'c-subnet', node_type: 'Container', container_type: 'Subnet', entity_id: 's1' },
			{
				id: 'n-ip',
				node_type: 'Element',
				element_type: 'IPAddress',
				host_id: 'pve',
				entity_id: 'ip-pve',
				subnet_id: 's1',
				container_id: 'c-subnet'
			},
			{
				id: 'n-port',
				node_type: 'Element',
				element_type: 'Interface',
				host_id: 'pve',
				entity_id: 'if-1',
				interface_id: 'if-1'
			},
			{
				id: 'n-port-8',
				node_type: 'Element',
				element_type: 'Interface',
				host_id: 'pve',
				entity_id: 'if-2',
				interface_id: 'if-2'
			},
			{
				id: 'c-vlan',
				node_type: 'Container',
				container_type: 'VLAN',
				header: 'VLAN 20 (Shop floor)',
				parent_container_id: 'c-pve'
			}
		],
		// What the backend associates: the device and address reached through port 8.
		node_entities: { 'n-port-8': ['plc', 'ip-plc'] }
	} as unknown as RenderableTopology;
}

const node = (topo: RenderableTopology, id: string) =>
	topo.nodes.find((n) => n.id === id) as TopologyNode;

beforeEach(() => {
	tagHiddenNodeIds.set(new Set());
	hiddenEntityIds.set(new Set());
	searchMatchNodeIds.set([]);
});

describe('the taggable entity behind a card', () => {
	it('is the card’s own entity for Host and Service cards', () => {
		const topo = buildTopology();
		expect(resolveTagTarget(node(topo, 'n-guest'))).toEqual({
			entityType: 'Host',
			entityId: 'vm-host'
		});
		expect(resolveTagTarget(node(topo, 'n-svc-vm'))).toEqual({
			entityType: 'Service',
			entityId: 'svc-vm'
		});
	});

	it('is the host for IP address and interface cards', () => {
		const topo = buildTopology();
		const host = { entityType: 'Host', entityId: 'pve' };
		expect(resolveTagTarget(node(topo, 'n-ip'))).toEqual(host);
		expect(resolveTagTarget(node(topo, 'n-port'))).toEqual(host);
	});

	it('is nothing for a container', () => {
		const topo = buildTopology();
		expect(resolveTagTarget(node(topo, 'c-pve'))).toBeNull();
	});
});

describe('the hosts and services a selected card stands for', () => {
	it('an IP card selects its host and the services bound to it or to every address', () => {
		const topo = buildTopology();
		const ids = getNodeSelectionIds('n-ip', node(topo, 'n-ip'), topo);
		expect(ids.hostIds).toEqual(['pve']);
		expect(ids.serviceIds.sort()).toEqual(['svc-proxmox', 'svc-vm']);
	});

	it('a service card selects its own service', () => {
		const topo = buildTopology();
		expect(getNodeSelectionIds('n-svc-vm', node(topo, 'n-svc-vm'), topo)).toEqual({
			hostIds: ['pve'],
			serviceIds: ['svc-vm']
		});
	});

	it('a host card selects every service on its host', () => {
		const topo = buildTopology();
		expect(getNodeSelectionIds('n-guest', node(topo, 'n-guest'), topo)).toEqual({
			hostIds: ['vm-host'],
			serviceIds: ['svc-guest']
		});
	});
});

describe('search resolves matches through entity ids', () => {
	it('a host search matches the host’s card and the service cards on it', () => {
		updateSearchFilter(buildTopology(), 'guest-vm', 'Workloads');
		const matches = get(searchMatchNodeIds);
		expect(matches).toContain('n-guest');
		expect(matches).toContain('n-svc-guest');
		expect(matches).not.toContain('n-svc-vm');
	});

	it('an address search matches the IP card, and a subnet search the subnet box', () => {
		updateSearchFilter(buildTopology(), '10.0.0.5', 'L3Logical');
		expect(get(searchMatchNodeIds)).toContain('n-ip');
		updateSearchFilter(buildTopology(), '10.0.0.0/24', 'L3Logical');
		expect(get(searchMatchNodeIds)).toContain('c-subnet');
	});

	it('in L2 an address matches the switch port it is reached through, and no other port', () => {
		updateSearchFilter(buildTopology(), '10.0.9.7', 'L2Physical');
		const matches = get(searchMatchNodeIds);
		expect(matches).toContain('n-port-8');
		expect(matches).not.toContain('n-port');
	});

	it('a port is found by the description it is labelled with', () => {
		updateSearchFilter(buildTopology(), '1/0/8', 'L2Physical');
		expect(get(searchMatchNodeIds)).toContain('n-port-8');
	});

	it('a subcontainer is found by the header drawn on it', () => {
		updateSearchFilter(buildTopology(), 'shop floor', 'L2Physical');
		expect(get(searchMatchNodeIds)).toContain('c-vlan');
	});
});

describe('hiding a hypervisor service', () => {
	it('by tag keeps the Hypervisor box and what is inside it', () => {
		updateTagFilter(
			buildTopology(),
			{ hidden_tag_ids: { Service: ['tag-infra'] } },
			'Workloads',
			undefined,
			[],
			site
		);
		expect(get(hiddenEntityIds).has('svc-proxmox')).toBe(true);
		const hidden = get(tagHiddenNodeIds);
		expect(hidden.has('c-hypervisor')).toBe(false);
		expect(hidden.has('n-svc-vm')).toBe(false);
	});

	it('by metadata keeps the Hypervisor box and what is inside it', () => {
		updateTagFilter(
			buildTopology(),
			undefined,
			'Workloads',
			{ Service: { Staleness: ['stale'] } },
			[],
			site
		);
		expect(get(hiddenEntityIds).has('svc-proxmox')).toBe(true);
		const hidden = get(tagHiddenNodeIds);
		expect(hidden.has('c-hypervisor')).toBe(false);
		expect(hidden.has('n-svc-vm')).toBe(false);
	});
});
