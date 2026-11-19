import { describe, it, expect } from 'vitest';
import type { LayoutGraph } from '#lib/features/topology/layout/layout-graph.ts';
import { reconcileMeasuredSizes } from '#lib/features/topology/pipeline/post-render.ts';
import type { XY } from '#lib/features/topology/pipeline/types.ts';

/** A mounted SvelteFlow node as `reconcileMeasuredSizes` reads it. */
const flowNode = (id: string, offsetHeight: number, drawnAsBox: boolean) => ({
	dataset: { id },
	offsetHeight,
	querySelector: (selector: string) =>
		selector === '[data-lod-simplified]' && drawnAsBox ? {} : null
});

const canvas = (nodes: ReturnType<typeof flowNode>[]) =>
	({ querySelectorAll: () => nodes }) as unknown as HTMLDivElement;

const graph = { containers: new Map() } as unknown as LayoutGraph;

describe('reconcileMeasuredSizes', () => {
	it('keeps the cached size of a card drawn as a box', () => {
		// A box-drawn card has no content: before SvelteFlow measures it, it is 2px of border. On
		// the Water Utility L2 view that height reached the stored layout, and a VLAN container was
		// laid out around a 2px card that renders 99px tall.
		const cache = new Map<string, XY>([['plc', { x: 120, y: 99 }]]);
		const drifted = reconcileMeasuredSizes(
			canvas([flowNode('plc', 2, true)]),
			cache,
			graph,
			new Set()
		);
		expect(drifted).toBe(0);
		expect(cache.get('plc')).toEqual({ x: 120, y: 99 });
	});

	it('corrects a card drawn in full whose height differs from the cache', () => {
		const cache = new Map<string, XY>([['plc', { x: 120, y: 2 }]]);
		const drifted = reconcileMeasuredSizes(
			canvas([flowNode('plc', 99, false)]),
			cache,
			graph,
			new Set()
		);
		expect(drifted).toBe(1);
		expect(cache.get('plc')).toEqual({ x: 120, y: 99 });
	});
});
