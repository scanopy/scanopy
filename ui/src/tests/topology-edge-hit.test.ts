/**
 * Edges at simplified zoom. They had no hit area at all, so an edge could only be picked by
 * zooming in. Each now has a bare hit path whose width follows the edge's weight: its speed band,
 * the count an aggregated or bundled edge stands for, and selection.
 */
import { describe, it, expect } from 'vitest';
import {
	simplifiedEdgeWeight,
	type EdgeStrokeShape
} from '#lib/features/topology/layout/edge-classification.ts';
import { TIER_STROKE_WIDTHS, type SpeedStroke } from '#lib/features/topology/edge-speed.ts';
import {
	SIMPLIFIED_EDGE_HIT_MIN_PX,
	SIMPLIFIED_EDGE_MAX_PX,
	SIMPLIFIED_EDGE_MIN_PX,
	simplifiedEdgeHitWidth
} from '#lib/features/topology/pipeline/render-mode.ts';

const plain: EdgeStrokeShape = {
	isAggregated: false,
	aggregatedCount: 1,
	isBundle: false,
	bundleStrokeWidth: 2,
	isOverlay: false
};

const weight = (
	shape: Partial<EdgeStrokeShape> = {},
	speed: SpeedStroke | null = null,
	emphasized = false
) => simplifiedEdgeWeight({ ...plain, ...shape, speed, emphasized });

const tier = (index: number): SpeedStroke => ({ tier: index, ends: null });

describe('simplifiedEdgeWeight', () => {
	it('is 1 for a plain link with no speed', () => {
		expect(weight()).toBe(1);
	});

	it('grows with the speed band', () => {
		const weights = TIER_STROKE_WIDTHS.map((_, i) => weight({}, tier(i)));
		for (let i = 1; i < weights.length; i++) {
			expect(weights[i]).toBeGreaterThanOrEqual(weights[i - 1]);
		}
		expect(weights.at(-1)).toBeGreaterThan(weights[0]);
	});

	it('grows with the number of links an aggregated edge stands for', () => {
		expect(weight({ isAggregated: true, aggregatedCount: 4 })).toBeGreaterThan(
			weight({ isAggregated: true, aggregatedCount: 1 })
		);
		expect(weight({ isAggregated: true, aggregatedCount: 4 })).toBeGreaterThan(weight());
	});

	it('grows with a bundle’s size', () => {
		expect(weight({ isBundle: true, bundleStrokeWidth: 5 })).toBeGreaterThan(
			weight({ isBundle: true, bundleStrokeWidth: 2.5 })
		);
	});

	it('keeps the speed band of an aggregated edge when that is the thicker weight', () => {
		const fastest = tier(TIER_STROKE_WIDTHS.length - 1);
		expect(weight({ isAggregated: true, aggregatedCount: 1 }, fastest)).toBe(weight({}, fastest));
	});

	it('doubles when the edge or its node is selected', () => {
		expect(weight({ isAggregated: true, aggregatedCount: 3 }, null, true)).toBe(
			2 * weight({ isAggregated: true, aggregatedCount: 3 })
		);
	});
});

describe('simplifiedEdgeHitWidth', () => {
	const strokes = [SIMPLIFIED_EDGE_MIN_PX, 2.5, SIMPLIFIED_EDGE_MAX_PX];
	const weights = [1, 1.5, 2.5, 4, 8];

	it('is never narrower than the minimum or than the drawn line', () => {
		for (const stroke of strokes) {
			for (const w of weights) {
				const hit = simplifiedEdgeHitWidth(stroke, w);
				expect(hit).toBeGreaterThanOrEqual(SIMPLIFIED_EDGE_HIT_MIN_PX);
				expect(hit).toBeGreaterThan(stroke * w);
			}
		}
	});

	it('widens a heavy edge’s hit area beyond a light one’s', () => {
		expect(simplifiedEdgeHitWidth(SIMPLIFIED_EDGE_MAX_PX, 4)).toBeGreaterThan(
			simplifiedEdgeHitWidth(SIMPLIFIED_EDGE_MAX_PX, 1)
		);
	});
});
