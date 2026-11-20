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
	isAtFit,
	type AdoptedNode
} from '#lib/features/topology/viewport-fit.ts';

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
