/**
 * Moving the camera onto a node: search matches, Focus buttons and the F shortcut.
 *
 * These used `fitView({ nodes })`, which only queues a fit until every node is measured; with
 * off-screen culling that never happens, so the camera never moved. They now compute the node's
 * bounds and call `fitBounds`, which moves at once, so the bounds have to be right.
 */
import { describe, it, expect } from 'vitest';
import { boundsOfAdoptedNodes, type AdoptedNode } from '$lib/features/topology/viewport-fit';

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
