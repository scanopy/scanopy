import { describe, it, expect } from 'vitest';
import viewsJson from '$lib/data/views.json';
import hostVirtualizationsJson from '$lib/data/host-virtualizations.json';
import ifOperStatusesJson from '$lib/data/if-oper-statuses.json';
import type { components } from '$lib/api/schema';
import type { Site } from '$lib/features/sites/types';
import type { RenderableTopology, TopologyNode } from '$lib/features/topology/types/base';
import { buildElementRender } from '$lib/features/topology/element-render-data';
import { type MarkChannel } from '$lib/features/topology/element-marks';
import { viewElementConfig } from '$lib/features/topology/view-filters';
import { cardEntityForFilter, resolveElementNode } from '$lib/features/topology/resolvers';
import { matchesHoveredMetadata } from '$lib/features/topology/interactions';

/**
 * A colour on a topology card has to be decodable in the view it is drawn in: some chip in that
 * view's filter panel carries the same colour, and hovering it rings this card. Checked for every
 * view in the views fixture against cards built from every classification the fixtures define
 * (each host virtualization type, each port status, stale and current), through the same
 * render function and hover matcher the canvas uses.
 *
 * Every card colour reaches `ElementNode` through `ElementRenderResult.marks`, so a colour added
 * outside the view's element marks has no path to the card; one added through a mark the view's
 * filters cannot decode fails here.
 */

type ViewElementConfig = components['schemas']['ViewElementConfig'];
type Color = components['schemas']['Color'];

const SITE: Site = {
	id: 'net',
	effective_stale_after_hours: 24
} as unknown as Site;
const NOW = new Date().toISOString();
const LONG_AGO = new Date(Date.now() - 30 * 24 * 60 * 60 * 1000).toISOString();

const ages = [
	['current', NOW],
	['stale', LONG_AGO]
] as const;
const virtualizations: Array<string | null> = [
	null,
	...hostVirtualizationsJson.map((v) => v.id as string)
];
const operStatuses: Array<string | null> = [null, ...ifOperStatusesJson.map((s) => s.id as string)];

/**
 * One host per virtualization type and age, each with an address, a service and a port per
 * status, so every element type a view draws has a card for every classification.
 */
function buildTopology(): RenderableTopology {
	const hosts: Record<string, unknown>[] = [];
	const ipAddresses: Record<string, unknown>[] = [];
	const services: Record<string, unknown>[] = [];
	const interfaces: Record<string, unknown>[] = [];
	const base = { site_id: SITE.id, tags: [], source: { type: 'Discovery' } };

	for (const [age, seen] of ages) {
		for (const virt of virtualizations) {
			const hostId = `host-${virt ?? 'bare'}-${age}`;
			hosts.push({
				...base,
				id: hostId,
				name: hostId,
				last_seen_at: seen,
				virtualization_metadata: virt ? { type: virt, details: {} } : null,
				virtualization_service_id: virt ? 'manager' : null
			});
			ipAddresses.push({
				...base,
				id: `${hostId}-ip`,
				host_id: hostId,
				subnet_id: 'subnet',
				ip_address: '10.0.0.1',
				last_seen_at: seen
			});
			services.push({
				...base,
				id: `${hostId}-svc`,
				host_id: hostId,
				name: 'svc',
				service_definition: 'SSH',
				bindings: [],
				last_seen_at: seen,
				virtualization_service_id: virt ? 'manager' : null
			});
			for (const status of operStatuses) {
				interfaces.push({
					...base,
					id: `${hostId}-if-${status ?? 'none'}`,
					host_id: hostId,
					oper_status: status,
					last_seen_at: seen
				});
			}
		}
	}

	return {
		id: 'topo',
		site_id: SITE.id,
		options: { request: { element_rules: [] } },
		hosts,
		subnets: [],
		ip_addresses: ipAddresses,
		services,
		ports: [],
		bindings: [],
		interfaces,
		dependencies: [],
		vlans: [],
		entity_tags: [],
		neighbours: [],
		nodes: [],
		edges: []
	} as unknown as RenderableTopology;
}

/** An element node of `elementType` for every matching entity in the topology. */
function cardNodes(topology: RenderableTopology, elementType: string): TopologyNode[] {
	const element = (id: string, hostId: string, extra: Record<string, unknown> = {}) =>
		({
			id,
			node_type: 'Element',
			element_type: elementType,
			host_id: hostId,
			header: hostId,
			...extra
		}) as unknown as TopologyNode;
	switch (elementType) {
		case 'Host':
			return topology.hosts.map((h) => element(`${h.id}-card`, h.id));
		case 'Service':
			return topology.services.map((s) => element(s.id, s.host_id));
		case 'IPAddress':
			return topology.ip_addresses.map((ip) =>
				element(`${ip.id}-card`, ip.host_id, { ip_address_id: ip.id, subnet_id: 'subnet' })
			);
		case 'Interface':
			return topology.interfaces.map((i) =>
				element(`${i.id}-card`, i.host_id, { interface_id: i.id })
			);
		default:
			throw new Error(`no card builder for ${elementType}`);
	}
}

/** The chips in `config` whose colour is `color` and whose hover rings this card. */
function decodingChips(
	config: ViewElementConfig,
	resolved: ReturnType<typeof resolveElementNode>,
	topology: RenderableTopology,
	color: Color
): string[] {
	const chips: string[] = [];
	for (const filter of config.metadata_filters ?? []) {
		for (const value of filter.values) {
			if (value.color !== color) continue;
			const hovered = {
				entityTypes: filter.entities,
				filterType: filter.filter_type,
				valueId: value.id,
				color: value.color
			};
			for (const entityType of filter.entities) {
				const entity = cardEntityForFilter(resolved, entityType);
				if (entity && matchesHoveredMetadata(entity, entityType, hovered, SITE, topology)) {
					chips.push(`${filter.entities.join('+')}/${filter.filter_type}/${value.id}`);
				}
			}
		}
	}
	return chips;
}

const VIEWS = viewsJson.map((v) => v.id as string);

describe('topology card colours are decodable in their view', () => {
	const topology = buildTopology();

	for (const view of VIEWS) {
		const config = viewElementConfig(view);

		it(`${view}: every coloured card has a chip of that colour whose hover rings it`, () => {
			expect(config).toBeDefined();
			if (!config) return;
			const painted = new Set<string>();

			for (const { entity_type } of config.element_entities) {
				for (const node of cardNodes(topology, entity_type)) {
					const result = buildElementRender({
						nodeId: node.id,
						node,
						topology,
						activeView: view,
						options: topology.options as never,
						hiddenEntityIds: new Set(),
						expandedInlineGroups: new Set(),
						sites: [SITE]
					});
					const resolved = resolveElementNode(node.id, node, topology);
					for (const [channel, color] of Object.entries(result.marks) as [MarkChannel, Color][]) {
						painted.add(channel);
						expect(
							decodingChips(config, resolved, topology, color),
							`${view} paints ${channel} ${color} on ${node.id} with no chip to decode it`
						).not.toHaveLength(0);
					}
				}
			}

			// Not vacuous: every channel the view declares a mark on was painted on some card.
			for (const mark of config.element_marks ?? []) {
				expect(
					painted,
					`${view}'s ${mark.channel} mark never painted: no card carries it, or the view offers no chip for ${mark.entity}/${mark.filter_type}/${mark.value}`
				).toContain(mark.channel);
			}
		});
	}

	it('L3 does not colour a virtualized host title', () => {
		const vm = topology.hosts.find((h) => h.virtualization_metadata != null);
		const node = cardNodes(topology, 'IPAddress').find(
			(n) => (n as { host_id?: string }).host_id === vm?.id
		)!;
		const result = buildElementRender({
			nodeId: node.id,
			node,
			topology,
			activeView: 'L3Logical',
			options: topology.options as never,
			hiddenEntityIds: new Set(),
			expandedInlineGroups: new Set(),
			sites: [SITE]
		});
		expect(result.marks.Title).toBeUndefined();
	});
});
