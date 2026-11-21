import { describe, it, expect } from 'vitest';
import { isEdgeHidden } from '#lib/features/topology/layout/edge-classification.ts';
import {
	DETAIL_ZOOM,
	SIMPLIFIED_EDGE_MAX_PX,
	SIMPLIFIED_EDGE_MIN_PX,
	simplifiedEdgeWidth
} from '#lib/features/topology/pipeline/render-mode.ts';
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

describe('isEdgeHidden', () => {
	it('hides an edge whose type the user turned off, and draws the rest', () => {
		const hiddenEdgeTypes = ['SameHost'];
		expect(isEdgeHidden(edge('SameHost'), { isPreview: false, hiddenEdgeTypes })).toBe(true);
		expect(isEdgeHidden(edge('RequestPath'), { isPreview: false, hiddenEdgeTypes })).toBe(false);
	});

	it('draws a dependency preview even when its type is turned off', () => {
		expect(
			isEdgeHidden(edge('RequestPath'), { isPreview: true, hiddenEdgeTypes: ['RequestPath'] })
		).toBe(false);
	});
});

describe('simplifiedEdgeWidth', () => {
	it('thickens as the view zooms out, between its floor and cap', () => {
		const zooms = [DETAIL_ZOOM, 0.3, 0.2, 0.1, 0.05, 0.01];
		const widths = zooms.map(simplifiedEdgeWidth);
		for (let i = 1; i < widths.length; i++) expect(widths[i]).toBeGreaterThanOrEqual(widths[i - 1]);
		expect(widths[0]).toBe(SIMPLIFIED_EDGE_MIN_PX);
		expect(widths.at(-1)).toBe(SIMPLIFIED_EDGE_MAX_PX);
	});

	it('never exceeds the cap, even before the zoom is known', () => {
		expect(simplifiedEdgeWidth(0)).toBe(SIMPLIFIED_EDGE_MAX_PX);
		expect(simplifiedEdgeWidth(0.0001)).toBe(SIMPLIFIED_EDGE_MAX_PX);
	});
});
