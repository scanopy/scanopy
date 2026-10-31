import { describe, it, expect } from 'vitest';
import type { Site } from '$lib/features/sites/types';
import type { Host } from '$lib/features/hosts/types/base';
import type { RenderableTopology, TopologyNode } from '$lib/features/topology/types/base';
import type { TooltipContent } from '$lib/shared/actions/tooltip';
import {
	buildElementRender,
	type ElementRenderInputs
} from '$lib/features/topology/element-render-data';
import { topologyLookup } from '$lib/features/topology/resolvers';
import { entityTooltip, hostTooltip } from '$lib/shared/entity-tooltip';
import { inlineGroupKey } from '$lib/features/topology/collapse';
import { hiddenEntityTypesIn } from '$lib/features/topology/queries';
import {
	elementWidthPx,
	ICON_ELEMENT_WIDTH_PX
} from '$lib/features/topology/pipeline/build-flow-nodes';
import { formatBitsPerSecond, formatPort } from '$lib/shared/utils/formatting';
import { deviceTypes, hostVirtualizations } from '$lib/shared/stores/metadata';

/**
 * What an icon card carries now that it is the only card: a port carries its oper status, a
 * manager box opens and closes, and the facts the detailed card used to print (an
 * address's name, each service's ports, a network identity's virtualization) reach the tooltip.
 */

const SITE = { id: 'site', effective_stale_after_hours: 24 } as unknown as Site;
const NOW = new Date().toISOString();
const base = { site_id: SITE.id, tags: [], last_seen_at: NOW, source: { type: 'Discovery' } };

const guestNode = {
	id: 'guest-card',
	node_type: 'Element',
	element_type: 'Host',
	host_id: 'guest',
	entity_id: 'guest',
	inline_groups: [
		{ entity_id: 'docker', entity_type: 'Service', group_id: 'docker', role: 'Header' },
		{ entity_id: 'nginx', entity_type: 'Service', group_id: 'docker', role: 'Member' }
	]
} as unknown as TopologyNode;

const addressNode = {
	id: 'ip-card',
	node_type: 'Element',
	element_type: 'IPAddress',
	host_id: 'guest',
	entity_id: 'ip',
	ip_address_id: 'ip',
	subnet_id: 'lan',
	header: 'guest'
} as unknown as TopologyNode;

const portNode = {
	id: 'if-card',
	node_type: 'Element',
	element_type: 'Interface',
	host_id: 'switch',
	entity_id: 'if',
	interface_id: 'if',
	header: 'Gi1/0/1'
} as unknown as TopologyNode;

function buildTopology(): RenderableTopology {
	return {
		id: 'topo',
		site_id: SITE.id,
		options: { request: { element_rules: [] } },
		hosts: [
			{ ...base, id: 'guest', name: 'guest', display_name: 'guest' },
			{ ...base, id: 'switch', name: 'switch', display_name: 'switch' }
		],
		subnets: [
			{ ...base, id: 'lan', cidr: '10.0.0.0/24', vlan_ids: ['vlan-10'] },
			{ ...base, id: 'dmz', cidr: '10.0.1.0/24', vlan_ids: [] }
		],
		ip_addresses: [
			{
				...base,
				id: 'ip',
				host_id: 'guest',
				subnet_id: 'lan',
				ip_address: '10.0.0.5',
				name: 'mgmt'
			},
			{ ...base, id: 'ip-2', host_id: 'guest', subnet_id: 'lan', ip_address: '10.0.0.6' }
		],
		services: [
			{
				...base,
				id: 'web',
				host_id: 'guest',
				name: 'web',
				service_definition: 'Nginx',
				bindings: [
					{ type: 'Port', port_id: 'p80', ip_address_id: 'ip' },
					{ type: 'Port', port_id: 'p8080', ip_address_id: 'ip-2' }
				]
			},
			{
				...base,
				id: 'docker',
				host_id: 'guest',
				name: 'Docker Daemon',
				service_definition: 'Docker',
				bindings: []
			},
			{
				...base,
				id: 'nginx',
				host_id: 'guest',
				name: 'nginx',
				service_definition: 'Nginx',
				bindings: []
			}
		],
		ports: [
			{ id: 'p80', number: 80, protocol: 'Tcp' },
			{ id: 'p8080', number: 8080, protocol: 'Tcp' }
		],
		bindings: [],
		interfaces: [
			{
				...base,
				id: 'if',
				host_id: 'switch',
				display_name: 'Gi1/0/1',
				oper_status: 'Up',
				speed_bps: 1_000_000_000,
				filter_values: { LinkState: 'Linked', OperStatus: 'Up' }
			}
		],
		dependencies: [],
		vlans: [{ ...base, id: 'vlan-10', vlan_number: 10, subnet_ids: ['lan'] }],
		entity_tags: [],
		neighbours: [],
		nodes: [guestNode, addressNode, portNode],
		edges: []
	} as unknown as RenderableTopology;
}

function render(node: TopologyNode, view: string, extra: Partial<ElementRenderInputs> = {}) {
	const topology = buildTopology();
	return buildElementRender({
		nodeId: node.id,
		node,
		topology,
		activeView: view,
		options: topology.options as never,
		hiddenEntityIds: new Set(),
		expandedInlineGroups: new Set(),
		sites: [SITE],
		...extra
	});
}

const rowValues = (content: TooltipContent) =>
	content.sections.flatMap((s) => ('rows' in s && s.rows ? s.rows.map((r) => r.value) : []));
const listItems = (content: TooltipContent) =>
	content.sections.flatMap((s) => ('items' in s && s.items ? s.items : []));

describe('a port draws as an icon card', () => {
	it('is built at the width of a plain icon card', () => {
		expect(elementWidthPx(portNode)).toBe(ICON_ELEMENT_WIDTH_PX);
	});

	it('carries its oper status as the Status mark, and its speed on hover', () => {
		const result = render(portNode, 'L2Physical');
		expect(result.iconCard?.glyph.type).toBe('port');
		expect(result.iconCard?.label).toBe('Gi1/0/1');
		expect(result.marks.Status?.id).toBe('Up');
		const rows = result.iconCard!.tooltip().sections.flatMap((s) => ('rows' in s ? s.rows : []));
		expect(rows.some((r) => r?.chips?.some((c) => c.type === 'Tag'))).toBe(true);
		expect(rowValues(result.iconCard!.tooltip())).toContain(formatBitsPerSecond(1_000_000_000));
	});
});

describe('a manager box opens and closes', () => {
	it('is collapsed until the viewer opens it', () => {
		const closed = render(guestNode, 'Workloads');
		expect(closed.iconCard?.groups[0].collapsed).toBe(true);

		const open = render(guestNode, 'Workloads', {
			expandedInlineGroups: new Set([inlineGroupKey(guestNode.id, 'docker')])
		});
		expect(open.iconCard?.groups[0].collapsed).toBe(false);
		expect(open.iconCard?.groups[0].members).toHaveLength(1);
	});
});

describe('facts the detailed card printed reach the tooltip', () => {
	it("names an address card's address", () => {
		const result = render(addressNode, 'L3Logical');
		expect(rowValues(result.iconCard!.tooltip())).toContain('mgmt');
	});

	it('lists each service with the ports it listens on at this address only, set apart', () => {
		const items = listItems(render(addressNode, 'L3Logical').iconCard!.tooltip());
		const web = items.find((i) => typeof i !== 'string' && i.text === 'web');
		const detail = typeof web === 'string' ? undefined : web?.detail;
		expect(detail).toContain(formatPort({ number: 80, protocol: 'Tcp' } as never));
		expect(detail).not.toContain(formatPort({ number: 8080, protocol: 'Tcp' } as never));
	});

	it("gives a network identity's virtualization a row", () => {
		const identity = {
			...base,
			id: 'mv',
			name: 'mv',
			virtualization_metadata: { type: 'NetworkIdentity', details: {} }
		} as unknown as Host;
		expect(rowValues(hostTooltip(identity, { services: [] }))).toContain(
			hostVirtualizations.getName('NetworkIdentity')
		);
	});
});

describe('a container tooltip comes from the entity it stands for', () => {
	const ctx = () => ({ lookup: topologyLookup(buildTopology()) });

	it("shows a subnet's summary fields", () => {
		const subnet = buildTopology().subnets[0];
		expect(rowValues(entityTooltip('Subnet', subnet, ctx()))).toContain('10.0.0.0/24');
	});

	it("names a subnet's VLANs, and adds no VLAN row to a subnet without any", () => {
		const [lan, dmz] = buildTopology().subnets;
		expect(rowValues(entityTooltip('Subnet', lan, ctx()))).toContain('10');
		const vlanLabel = (s: object) =>
			entityTooltip('Subnet', s, ctx()).sections.flatMap((sec) =>
				'rows' in sec && sec.rows ? sec.rows.map((r) => r.label) : []
			);
		const vlanRow = vlanLabel(lan).find((l) => !vlanLabel(dmz).includes(l));
		expect(vlanRow).toBeDefined();
	});

	it('shows a host the way its card does', () => {
		const host = buildTopology().hosts.find((h) => h.id === 'switch')!;
		expect(rowValues(entityTooltip('Host', host, ctx()))).toContain(
			deviceTypes.getName('Unidentified')
		);
	});
});

describe('saved entity hides', () => {
	it('ignore a type the view no longer draws, and keep one it does', () => {
		const options = {
			request: { hide_entities: { L3Logical: ['Port', 'Service'] } }
		} as never;
		expect(hiddenEntityTypesIn(options, 'L3Logical')).toEqual(['Service']);
	});
});
