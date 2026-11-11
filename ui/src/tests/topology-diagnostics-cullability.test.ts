/**
 * The diagnostic has to be able to name the fault it was extended for.
 *
 * A customer's report showed `culling: on` beside `mounted == store.nodes` and contained nothing
 * that explained the contradiction — because whether SvelteFlow *can* cull a node depends on two
 * fields nothing was reading. `summariseCullability` is that reading, so these check it counts the
 * two conditions that make a node unconditionally visible.
 */
import { describe, it, expect } from 'vitest';
import { summariseCullability, type CullableNode } from '#lib/features/topology/diagnostics.ts';

const cullable = (): CullableNode => ({
	measured: { width: 250, height: 120 },
	internals: { handleBounds: { source: [], target: [] } }
});

describe('summariseCullability', () => {
	it('counts a fully measured node as cullable', () => {
		const summary = summariseCullability([cullable(), cullable()]);

		expect(summary).toEqual({
			total: 2,
			withMeasured: 2,
			withHandleBounds: 2,
			forceRendered: 0
		});
	});

	it('counts a node with no handle bounds as force-rendered', () => {
		// `forceInitialRender` in `getNodesInside` — the node skips the viewport test entirely.
		// This is what a rebuilt node set looked like before the fix, for every node in it.
		const summary = summariseCullability([{ measured: { width: 250, height: 120 } }]);

		expect(summary.withMeasured).toBe(1);
		expect(summary.withHandleBounds).toBe(0);
		expect(summary.forceRendered).toBe(1);
	});

	it('counts a node with a width but no height as force-rendered', () => {
		// `area = width * height` is 0 without a height, and the visibility test is
		// `overlappingArea >= area`, so `0 >= 0` passes wherever the viewport is. Element cards
		// were built exactly this way — a literal width of 250 and no height at all.
		const summary = summariseCullability([
			{ measured: { width: 250 }, internals: { handleBounds: {} } }
		]);

		expect(summary.withHandleBounds).toBe(1);
		expect(summary.withMeasured).toBe(0);
		expect(summary.forceRendered).toBe(1);
	});

	it('ignores ids the lookup has no node for', () => {
		// `getInternalNode` returns undefined for a store node SvelteFlow has not adopted yet, and
		// a sample taken mid-update will contain some. They are absent, not force-rendered.
		const summary = summariseCullability([cullable(), undefined]);

		expect(summary.total).toBe(1);
		expect(summary.forceRendered).toBe(0);
	});
});
