import { describe, it, expect } from 'vitest';
import { PhysicalLinkEdgeDisplay } from '$lib/shared/components/forms/selection/display/PhysicalLinkEdgeDisplay.svelte';
import type { RenderableTopology, TopologyEdge } from '$lib/features/topology/types/base';

const topology = {
	hosts: [
		{ id: 'h1', display_name: 'switch-core' },
		{ id: 'h2', display_name: 'switch-top-of-rack' }
	],
	interfaces: [
		{ id: 'i1', host_id: 'h1', display_name: 'Te1/1/1' },
		{ id: 'i2', host_id: 'h2', display_name: 'Te1/0/49' },
		{ id: 'i3', host_id: 'h1', display_name: 'Te1/1/2' },
		{ id: 'i4', host_id: 'h2', display_name: 'Te1/0/50' }
	]
} as unknown as RenderableTopology;

const physicalLink = (id: string, source: string, target: string) =>
	({
		id,
		edge_type: 'PhysicalLink',
		source_entity_id: source,
		target_entity_id: target,
		protocol: 'LLDP'
	}) as unknown as TopologyEdge;

const context = { topology };

/**
 * Two switches joined by a port-channel draw several links between the same pair of hosts. In the
 * aggregated inspector each one needs a row of its own.
 */
describe('PhysicalLinkEdgeDisplay', () => {
	it('tells parallel links between the same hosts apart by their ports', () => {
		const a = PhysicalLinkEdgeDisplay.getLabel(physicalLink('e1', 'i1', 'i2'), context);
		const b = PhysicalLinkEdgeDisplay.getLabel(physicalLink('e2', 'i3', 'i4'), context);
		expect(a).not.toBe(b);
		expect(a).toContain('Te1/1/1');
		expect(a).toContain('Te1/0/49');
	});

	it('names the hosts and protocol in the description', () => {
		const description = PhysicalLinkEdgeDisplay.getDescription!(
			physicalLink('e1', 'i1', 'i2'),
			context
		);
		expect(description).toContain('switch-core');
		expect(description).toContain('switch-top-of-rack');
		expect(description).toContain('LLDP');
	});

	it('labels a neighbor link, whose ports are unresolved, by its hosts', () => {
		const edge = {
			id: 'e3',
			edge_type: 'NeighborLink',
			source_host_id: 'h1',
			target_host_id: 'h2'
		} as unknown as TopologyEdge;
		const label = PhysicalLinkEdgeDisplay.getLabel(edge, context);
		expect(label).toContain('switch-core');
		expect(label).toContain('switch-top-of-rack');
	});
});
