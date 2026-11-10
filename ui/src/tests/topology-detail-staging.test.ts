import { describe, it, expect } from 'vitest';
import { grantWithinBudget } from '$lib/features/topology/detail-staging.svelte';
import {
	DETAIL_ZOOM,
	LOD_MIN_NODES,
	PREWARM_RELEASE_ZOOM,
	PREWARM_ZOOM,
	detailIsStaged
} from '$lib/features/topology/pipeline/render-mode';

/**
 * Card detail is mounted a few cards a frame ahead of the zoom where it shows. The frame's share
 * is a time budget, and these pin what a frame may and may not do with it.
 */
describe('grantWithinBudget', () => {
	/** A clock that advances by `cost` each time a card is granted. */
	const mounting = (cost: number) => {
		let time = 0;
		const granted: string[] = [];
		return {
			granted,
			grant: (id: string) => {
				granted.push(id);
				time += cost;
			},
			now: () => time
		};
	};

	it('grants in the order nodes asked, until the budget is spent', () => {
		const queue = new Set(['a', 'b', 'c', 'd', 'e']);
		const frame = mounting(2);

		const count = grantWithinBudget(queue, frame.grant, frame.now, 5);

		// Three cards at 2 ms each pass the 5 ms mark; the rest wait for the next frame.
		expect(count).toBe(3);
		expect(frame.granted).toEqual(['a', 'b', 'c']);
		expect([...queue]).toEqual(['d', 'e']);
	});

	it('grants one card even when it alone exceeds the budget', () => {
		// Otherwise a heavy card at the head of the queue would hold up every card behind it.
		const queue = new Set(['heavy', 'next']);
		const frame = mounting(40);

		expect(grantWithinBudget(queue, frame.grant, frame.now, 5)).toBe(1);
		expect([...queue]).toEqual(['next']);
	});

	it('empties a short queue within one frame', () => {
		const queue = new Set(['a', 'b']);
		const frame = mounting(1);

		expect(grantWithinBudget(queue, frame.grant, frame.now, 5)).toBe(2);
		expect(queue.size).toBe(0);
	});

	it('does nothing with nothing waiting', () => {
		const frame = mounting(1);
		expect(grantWithinBudget(new Set(), frame.grant, frame.now, 5)).toBe(0);
	});
});

describe('detailIsStaged', () => {
	const large = { nodeCount: LOD_MIN_NODES, measuring: false, exporting: false };

	it('stages a graph that is simplified by zoom', () => {
		expect(detailIsStaged(large)).toBe(true);
	});

	it('leaves a small graph drawing its cards at once', () => {
		expect(detailIsStaged({ ...large, nodeCount: LOD_MIN_NODES - 1 })).toBe(false);
	});

	it('gives a measure pass and an export every card in full at once', () => {
		// A staged card still waiting is a box, and a box measures and exports as one.
		expect(detailIsStaged({ ...large, measuring: true })).toBe(false);
		expect(detailIsStaged({ ...large, exporting: true })).toBe(false);
	});
});

describe('the prewarm zooms', () => {
	it('start preparing detail below the zoom where it shows, and drop it lower still', () => {
		expect(PREWARM_ZOOM).toBeLessThan(DETAIL_ZOOM);
		expect(PREWARM_RELEASE_ZOOM).toBeLessThan(PREWARM_ZOOM);
	});
});
