import { describe, it, expect } from 'vitest';
import { getEndpointLabels } from '#lib/features/topology/layout/edge-classification.ts';
import type { components } from '#lib/api/schema';

type TopologyEdge = components['schemas']['Edge'];

function makeEdge(label: string | null, endpointLabels?: [string, string]): TopologyEdge {
	return {
		edge_type: 'PhysicalLink',
		source: 'a',
		target: 'b',
		source_handle: 'Bottom',
		target_handle: 'Top',
		label,
		endpoint_labels: endpointLabels,
		view_config: { type: 'disabled' as const }
	} as TopologyEdge;
}

describe('getEndpointLabels', () => {
	it('returns both ends, source first, when the edge names them', () => {
		const edge = makeEdge('Gi0/1 ↔ Gi0/2', ['Gi0/1', 'Gi0/2']);
		expect(getEndpointLabels(edge, edge.label)).toEqual(['Gi0/1', 'Gi0/2']);
	});

	it('keeps a name that itself contains the separator whole', () => {
		const edge = makeEdge('a ↔ b ↔ c', ['a ↔ b', 'c']);
		expect(getEndpointLabels(edge, edge.label)).toEqual(['a ↔ b', 'c']);
	});

	it('returns null for an edge without endpoint labels, even if its label looks joined', () => {
		const edge = makeEdge('web ↔ db');
		expect(getEndpointLabels(edge, edge.label)).toBeNull();
	});

	it('returns null once the rendered label is stripped (intra-container or aggregated)', () => {
		const edge = makeEdge('Gi0/1 ↔ Gi0/2', ['Gi0/1', 'Gi0/2']);
		expect(getEndpointLabels(edge, null)).toBeNull();
		expect(getEndpointLabels(edge, undefined)).toBeNull();
	});
});
