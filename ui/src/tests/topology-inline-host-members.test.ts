import { describe, it, expect } from 'vitest';
import type { RenderableTopology, TopologyNode } from '$lib/features/topology/types/base';
import type { Service } from '$lib/features/services/types/base';
import {
	buildInlineGroups,
	inspectorServiceSections
} from '$lib/features/topology/element-render-data';
import { resolveInlineServiceIds } from '$lib/features/topology/resolvers';
import { inlineHostsMatching } from '$lib/features/topology/interactions';

/**
 * A guest VM's Network Identities service heads an inline group on the guest's Workloads card,
 * and its members are hosts (one per identity), not services. The card used to look every
 * member up among the guest's own services, so the box rendered empty and the identities'
 * services were invisible to search and highlighting.
 */

const NETWORK_ID = 'net-1';

function service(id: string, hostId: string, definition: string) {
	return { id, host_id: hostId, name: id, service_definition: definition, tags: [], bindings: [] };
}

const guestNode = {
	id: 'guest-element',
	node_type: 'Element',
	element_type: 'Host',
	host_id: 'guest',
	container_id: 'hypervisor',
	inline_groups: [
		{ entity_id: 'identities', entity_type: 'Service', group_id: 'identities', role: 'Header' },
		{ entity_id: 'mv-1', entity_type: 'Host', group_id: 'identities', role: 'Member' },
		{ entity_id: 'mv-2', entity_type: 'Host', group_id: 'identities', role: 'Member' }
	]
};

function buildTopology(): RenderableTopology {
	return {
		id: 'topo-1',
		network_id: NETWORK_ID,
		name: 'test',
		options: { request: { element_rules: [] } },
		hosts: [
			{ id: 'guest', network_id: NETWORK_ID, tags: [], display_name: 'snmp-test' },
			{
				id: 'mv-1',
				network_id: NETWORK_ID,
				tags: [],
				display_name: '192.168.4.101',
				virtualization_metadata: { type: 'NetworkIdentity', details: { interface: 'mv-snmp1' } }
			},
			{ id: 'mv-2', network_id: NETWORK_ID, tags: [], display_name: '192.168.4.102' }
		],
		subnets: [],
		ip_addresses: [],
		services: [
			service('identities', 'guest', 'Network Identities'),
			service('tftp', 'guest', 'TFTP Server'),
			service('mv-1-snmp', 'mv-1', 'SNMP'),
			service('mv-2-snmp', 'mv-2', 'SNMP')
		],
		ports: [],
		bindings: [],
		interfaces: [],
		dependencies: [],
		vlans: [],
		entity_tags: [],
		nodes: [{ id: 'hypervisor', node_type: 'Container', container_type: 'Host' }, guestNode],
		edges: []
	} as unknown as RenderableTopology;
}

const guestServices = buildTopology().services.filter((s) => s.host_id === 'guest') as Service[];

describe('inline groups with host members', () => {
	it('resolves host members, each with its own services, under the manager header', () => {
		const groups = buildInlineGroups(
			guestNode as unknown as TopologyNode,
			guestServices,
			buildTopology(),
			new Set(),
			() => true
		);

		expect(groups).toHaveLength(1);
		expect(groups[0].header?.id).toBe('identities');
		expect(groups[0].hosts.map((h) => [h.host.id, h.services.map((s) => s.id)])).toEqual([
			['mv-1', ['mv-1-snmp']],
			['mv-2', ['mv-2-snmp']]
		]);
	});

	it('marks a group collapsed and still resolves its members for the count', () => {
		const groups = buildInlineGroups(
			guestNode as unknown as TopologyNode,
			guestServices,
			buildTopology(),
			new Set(),
			() => true,
			(groupId) => groupId === 'identities'
		);

		expect(groups[0].collapsed).toBe(true);
		expect(groups[0].hosts).toHaveLength(2);
	});

	it('drops a hidden member host and a hidden member service', () => {
		const groups = buildInlineGroups(
			guestNode as unknown as TopologyNode,
			guestServices,
			buildTopology(),
			new Set(['mv-1']),
			(s) => s.id !== 'mv-2-snmp'
		);

		expect(groups[0].hosts.map((h) => [h.host.id, h.services.length])).toEqual([['mv-2', 0]]);
	});

	it("counts the member hosts' services as rendered inside the guest's card", () => {
		const ids = resolveInlineServiceIds(new Set(['guest-element']), buildTopology());

		expect([...ids].sort()).toEqual(['identities', 'mv-1-snmp', 'mv-2-snmp', 'tftp']);
	});
});

describe('host metadata hover on inline members', () => {
	const groups = () =>
		buildInlineGroups(
			guestNode as unknown as TopologyNode,
			guestServices,
			buildTopology(),
			new Set(),
			() => true
		);
	const hover = (valueId: string) => ({
		entityTypes: ['Host' as const],
		filterType: 'Virtualization',
		valueId,
		color: 'Amber'
	});

	it('marks a network identity as a network identity, not virtualized or bare metal', () => {
		const topology = buildTopology();
		expect(
			inlineHostsMatching(groups(), hover('NetworkIdentity'), () => undefined, topology)
		).toEqual(new Set(['mv-1']));
		expect(inlineHostsMatching(groups(), hover('Virtualized'), () => undefined, topology)).toEqual(
			new Set()
		);
		expect(inlineHostsMatching(groups(), hover('BareMetal'), () => undefined, topology)).toEqual(
			new Set(['mv-2'])
		);
	});
});

describe("inspector services for a guest's host element", () => {
	it('lists own services apart from the box, and keeps service-less identities in it', () => {
		const topology = buildTopology();
		// mv-2 loses its only service to a filter but stays listed, as on the card.
		const { own, groups } = inspectorServiceSections(
			guestNode as unknown as TopologyNode,
			guestServices,
			topology,
			new Set(['mv-2-snmp']),
			null
		);

		expect(own.map((s) => s.id)).toEqual(['tftp']);
		expect(groups[0].header?.id).toBe('identities');
		expect(groups[0].hosts.map((h) => [h.host.id, h.services.map((s) => s.id)])).toEqual([
			['mv-1', ['mv-1-snmp']],
			['mv-2', []]
		]);
	});

	it('keeps a host element services bound to one address or none', () => {
		const bound = {
			...service('web', 'guest', 'Nginx'),
			bindings: [{ type: 'Port', ip_address_id: 'ip-a', port_id: 'p' }]
		};
		const unbound = service('agent', 'guest', 'SNMP');
		const services = [bound, unbound] as unknown as Service[];
		const bare = { ...guestNode, inline_groups: [] } as unknown as TopologyNode;

		const onHost = inspectorServiceSections(bare, services, buildTopology(), new Set(), null);
		expect(onHost.own.map((s) => s.id)).toEqual(['web', 'agent']);

		// An IP-address element narrows to what that address binds (or binds everywhere).
		const onOtherIp = inspectorServiceSections(bare, services, buildTopology(), new Set(), 'ip-b');
		expect(onOtherIp.own.map((s) => s.id)).toEqual([]);
	});
});
