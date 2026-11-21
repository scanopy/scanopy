/**
 * Moving the camera onto a node: search matches, Focus buttons and the F shortcut.
 *
 * These used `fitView({ nodes })`, which only queues a fit until every node is measured; with
 * off-screen culling that never happens, so the camera never moved. They now compute the node's
 * bounds and call `fitBounds`, which moves at once, so the bounds have to be right.
 */
import { describe, it, expect } from 'vitest';
import {
	boundsOfAdoptedNodes,
	focusEdge,
	focusEdgeEnd,
	isAtFit,
	type AdoptedNode,
	type FocusableFlow
} from '#lib/features/topology/viewport-fit.ts';
import type { Rect } from '@xyflow/system';

const node = (x: number, y: number, width: number, height: number): AdoptedNode => ({
	internals: { positionAbsolute: { x, y } },
	measured: { width, height }
});

describe('boundsOfAdoptedNodes', () => {
	it('places a node nested in a container at its absolute position', () => {
		const nodes: Record<string, AdoptedNode> = { ip: node(1950, 824, 250, 162) };
		expect(boundsOfAdoptedNodes(['ip'], (id) => nodes[id])).toEqual({
			x: 1950,
			y: 824,
			width: 250,
			height: 162
		});
	});

	it('spans several nodes, skipping ids the flow has not adopted', () => {
		const nodes: Record<string, AdoptedNode> = {
			a: node(0, 0, 100, 50),
			b: node(300, 200, 100, 50)
		};
		expect(boundsOfAdoptedNodes(['a', 'gone', 'b'], (id) => nodes[id])).toEqual({
			x: 0,
			y: 0,
			width: 400,
			height: 250
		});
	});

	it('is null when no id is adopted, so the camera stays put', () => {
		expect(boundsOfAdoptedNodes(['gone'], () => undefined)).toBeNull();
	});
});

describe('isAtFit', () => {
	const fit = { x: -812.25, y: 140.5, zoom: 0.0491273 };

	it('holds after the float round trip through d3', () => {
		expect(isAtFit({ x: -812.2500001, y: 140.4999, zoom: 0.04912731 }, fit)).toBe(true);
	});

	it('is false once the user pans or zooms, so selecting keeps their viewport', () => {
		expect(isAtFit({ ...fit, x: fit.x + 3 }, fit)).toBe(false);
		expect(isAtFit({ ...fit, zoom: fit.zoom * 1.2 }, fit)).toBe(false);
	});

	it('is false before any fit', () => {
		expect(isAtFit(fit, null)).toBe(false);
	});
});

describe('focusEdge', () => {
	/** A flow over the given adopted nodes that records the bounds it was asked to fit. */
	function flowOver(nodes: Record<string, AdoptedNode>) {
		const fitted: Rect[] = [];
		const flow: FocusableFlow = {
			fitBounds: async (bounds) => {
				fitted.push(bounds);
				return true;
			},
			getInternalNode: (id) => nodes[id]
		};
		return { flow, fitted };
	}

	it('frames both ends of a plain edge, not their containers', () => {
		const { flow, fitted } = flowOver({
			subnet: node(0, 0, 5000, 5000),
			a: node(100, 100, 200, 100),
			b: node(900, 500, 200, 100)
		});
		focusEdge(flow, { source: 'a', target: 'b' });
		expect(fitted).toEqual([{ x: 100, y: 100, width: 1000, height: 500 }]);
	});

	it('frames an end that is culled off screen, since it is still adopted', () => {
		const { flow, fitted } = flowOver({
			a: node(0, 0, 100, 100),
			offscreen: node(20000, 15000, 100, 100)
		});
		focusEdge(flow, { source: 'a', target: 'offscreen' });
		expect(fitted).toEqual([{ x: 0, y: 0, width: 20100, height: 15100 }]);
	});

	it('frames the collapsed container that stands in for an aggregated edge’s hidden ends', () => {
		// The aggregated flow edge points at the collapsed container; the ends inside it are not
		// adopted.
		const { flow, fitted } = flowOver({
			collapsed: node(0, 0, 300, 80),
			far: node(1000, 0, 200, 100)
		});
		focusEdge(flow, { source: 'collapsed', target: 'far' }, [
			{ source: 'hidden-1', target: 'far' },
			{ source: 'hidden-2', target: 'far' }
		]);
		expect(fitted).toEqual([{ x: 0, y: 0, width: 1200, height: 100 }]);
	});

	it('frames every end an aggregated edge stands for when they are drawn', () => {
		const { flow, fitted } = flowOver({
			a: node(0, 0, 100, 100),
			b: node(500, 0, 100, 100),
			c: node(0, 800, 100, 100)
		});
		focusEdge(flow, { source: 'a', target: 'b' }, [
			{ source: 'a', target: 'b' },
			{ source: 'c', target: 'b' }
		]);
		expect(fitted).toEqual([{ x: 0, y: 0, width: 600, height: 900 }]);
	});

	it('frames every member of a bundle, not only the edge drawn for it', () => {
		const { flow, fitted } = flowOver({
			a: node(0, 0, 100, 100),
			b: node(500, 0, 100, 100),
			c: node(0, 800, 100, 100),
			d: node(1500, 800, 100, 100)
		});
		focusEdge(flow, {
			source: 'a',
			target: 'b',
			data: {
				bundleEdges: [
					{ source: 'a', target: 'b' },
					{ source: 'c', target: 'd' }
				]
			}
		});
		expect(fitted).toEqual([{ x: 0, y: 0, width: 1600, height: 900 }]);
	});

	it('does not move the camera when no end is drawn', () => {
		const { flow, fitted } = flowOver({});
		focusEdge(flow, { source: 'gone-a', target: 'gone-b' });
		expect(fitted).toEqual([]);
	});

	describe('focusEdgeEnd', () => {
		const nodes = {
			card: node(100, 100, 200, 100),
			container: node(2000, 0, 800, 600),
			offscreen: node(30000, 20000, 200, 100)
		};

		it('frames only the chosen end', () => {
			const { flow, fitted } = flowOver(nodes);
			focusEdgeEnd(flow, { source: 'card', target: 'container' }, 'source');
			focusEdgeEnd(flow, { source: 'card', target: 'container' }, 'target');
			expect(fitted).toEqual([
				{ x: 100, y: 100, width: 200, height: 100 },
				{ x: 2000, y: 0, width: 800, height: 600 }
			]);
		});

		it('frames an end that is culled off screen', () => {
			const { flow, fitted } = flowOver(nodes);
			focusEdgeEnd(flow, { source: 'card', target: 'offscreen' }, 'target');
			expect(fitted).toEqual([{ x: 30000, y: 20000, width: 200, height: 100 }]);
		});

		it('does not move the camera when the end is not drawn', () => {
			const { flow, fitted } = flowOver(nodes);
			focusEdgeEnd(flow, { source: 'gone', target: 'card' }, 'source');
			expect(fitted).toEqual([]);
		});
	});
});
