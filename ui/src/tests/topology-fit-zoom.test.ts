/**
 * The zoom floor, and the bounds it is derived from.
 *
 * A fixed floor of 0.1 is what made a large estate unfittable: `getViewportForBounds` clamps to it,
 * so a graph needing 0.03 was centred and shown at a third of its width with the rest off-screen,
 * and `F` recomputed the same clamped transform every time. A capture had all 31 samples at exactly
 * `scale(0.1)` across two views and four collapse levels — eight separate fits, all clamping.
 */
import { describe, it, expect } from 'vitest';
import { getViewportForBounds } from '@xyflow/system';
import {
	ABSOLUTE_MIN_ZOOM,
	DEFAULT_MIN_ZOOM,
	boundsOfNodes,
	zoomFloorFor,
	type BoundableNode
} from '#lib/features/topology/viewport-fit.ts';

/** The reporting customer's pane. */
const PANE = { width: 2028, height: 1210 };

const container = (x: number, y: number, width = 250, height = 120): BoundableNode => ({
	position: { x, y },
	measured: { width, height }
});

describe('zoomFloorFor', () => {
	it('leaves the floor alone for a graph that already fits', () => {
		// The guardrail: nothing changes at the scale nobody has complained about.
		expect(zoomFloorFor(0.8)).toBe(DEFAULT_MIN_ZOOM);
		expect(zoomFloorFor(DEFAULT_MIN_ZOOM)).toBe(DEFAULT_MIN_ZOOM);
	});

	it('drops below the old floor for a graph that needs it', () => {
		const floor = zoomFloorFor(0.03);

		expect(floor).toBeLessThan(DEFAULT_MIN_ZOOM);
		// Low enough that the fit is not clamped by it — the whole point.
		expect(floor).toBeLessThan(0.03);
	});

	it('holds at the absolute floor for a graph past the point of fitting', () => {
		// At 0.0001 an element card is a fraction of a pixel; fitting stops being a service. The
		// clamp still happens, but now at a documented limit rather than one chosen for a much
		// smaller estate — and `fitZoom.clampedAtFloor` says so in the export.
		expect(zoomFloorFor(0.0001)).toBe(ABSOLUTE_MIN_ZOOM);
	});

	it('falls back to the old floor for a graph with no size', () => {
		// `getViewportForBounds` divides by the bounds, so a degenerate graph yields Infinity or a
		// zero. Neither is a floor, and propagating one into a transform blanks the canvas for real.
		expect(zoomFloorFor(Infinity)).toBe(DEFAULT_MIN_ZOOM);
		expect(zoomFloorFor(0)).toBe(DEFAULT_MIN_ZOOM);
		expect(zoomFloorFor(Number.NaN)).toBe(DEFAULT_MIN_ZOOM);
	});

	it('lets a fit reach a graph the old floor could not show', () => {
		// End to end against the function that actually applies the clamp: a layered graph laid out
		// as a wide strip, the shape an edge-less L3 Logical view produces.
		const strip = { x: 0, y: 0, width: 60000, height: 4000 };
		const clampedAtOldFloor = getViewportForBounds(
			strip,
			PANE.width,
			PANE.height,
			DEFAULT_MIN_ZOOM,
			2,
			0.2
		);
		expect(clampedAtOldFloor.zoom).toBe(DEFAULT_MIN_ZOOM);

		const required = getViewportForBounds(
			strip,
			PANE.width,
			PANE.height,
			ABSOLUTE_MIN_ZOOM,
			2,
			0.2
		).zoom;
		const withDerivedFloor = getViewportForBounds(
			strip,
			PANE.width,
			PANE.height,
			zoomFloorFor(required),
			2,
			0.2
		);

		expect(withDerivedFloor.zoom).toBeCloseTo(required, 10);
	});
});

describe('boundsOfNodes', () => {
	it('spans the top-level nodes', () => {
		expect(boundsOfNodes([container(0, 0), container(1000, 500)])).toEqual({
			x: 0,
			y: 0,
			width: 1250,
			height: 620
		});
	});

	it('ignores children, whose positions are relative to their parent', () => {
		// The subtlety worth a test. A child at `position: {x: 20, y: 40}` inside a container at
		// x=1000 is not at x=20, and folding it in drags the box back to the origin — inflating the
		// width, lowering the required zoom, and pushing the derived floor down for no reason.
		// Containers carry `expandParent`, so a root already encloses its descendants.
		const withChild: BoundableNode[] = [
			container(1000, 1000, 400, 300),
			{ position: { x: 20, y: 40 }, measured: { width: 250, height: 120 }, parentId: 'root' }
		];

		expect(boundsOfNodes(withChild)).toEqual({ x: 1000, y: 1000, width: 400, height: 300 });
	});

	it('returns null when there is no graph', () => {
		// Distinguishable from a graph at the origin, which is a real thing to fit.
		expect(boundsOfNodes([])).toBeNull();
		expect(boundsOfNodes([{ position: { x: 0, y: 0 }, parentId: 'orphan-parent' }])).toBeNull();
		expect(boundsOfNodes([container(0, 0, 0, 0)])).toEqual({
			x: 0,
			y: 0,
			width: 0,
			height: 0
		});
	});

	it('falls back to declared size when a node has not been measured', () => {
		// The pipeline emits `measured` only once a real size is known; `width`/`height` are what
		// the layout assigned. A fit taken before measurement should still frame the right region.
		expect(boundsOfNodes([{ position: { x: 10, y: 10 }, width: 90, height: 40 }])).toEqual({
			x: 10,
			y: 10,
			width: 90,
			height: 40
		});
	});
});
