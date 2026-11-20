import { describe, it, expect } from 'vitest';
import { isEdgeHidden } from '#lib/features/topology/layout/edge-classification.ts';
import type { components } from '#lib/api/schema';

type TopologyEdge = components['schemas']['Edge'];

const edge = (edge_type: string) =>
	({
		edge_type,
		source: 'a',
		target: 'b',
		source_handle: 'Right',
		target_handle: 'Left',
		label: null,
		view_config: { type: 'disabled' as const }
	}) as unknown as TopologyEdge;

const state = (over: Partial<Parameters<typeof isEdgeHidden>[1]> = {}) => ({
	isPreview: false,
	simplified: false,
	isPhysical: false,
	hiddenEdgeTypes: [] as string[],
	...over
});

describe('isEdgeHidden below the detail zoom', () => {
	it('keeps physical links and hides logical edges', () => {
		expect(isEdgeHidden(edge('PhysicalLink'), state({ simplified: true, isPhysical: true }))).toBe(
			false
		);
		expect(isEdgeHidden(edge('RequestPath'), state({ simplified: true }))).toBe(true);
	});

	it('still hides a physical link whose type the user turned off', () => {
		expect(
			isEdgeHidden(
				edge('PhysicalLink'),
				state({ simplified: true, isPhysical: true, hiddenEdgeTypes: ['PhysicalLink'] })
			)
		).toBe(true);
	});

	it('draws a dependency preview at any zoom', () => {
		expect(isEdgeHidden(edge('RequestPath'), state({ simplified: true, isPreview: true }))).toBe(
			false
		);
	});
});
