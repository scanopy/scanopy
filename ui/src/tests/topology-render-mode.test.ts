import { describe, it, expect } from 'vitest';
import {
	shouldCull,
	shouldSimplify,
	nodeDetail,
	BOX_MIN_PX,
	LOD_MIN_NODES,
	DETAIL_ZOOM
} from '$lib/features/topology/pipeline/render-mode';

/**
 * Culling has two hard suspensions. Both exist because something else needs
 * every node present in the DOM, and in both cases the failure is silent
 * rather than loud — a truncated measurement or a cropped export, not an error.
 */

const LARGE = 500;
const SMALL = 20;

describe('shouldCull', () => {
	it('culls a large graph when nothing needs the full DOM', () => {
		expect(shouldCull({ renderedCount: LARGE, measuring: false, exporting: false })).toBe(true);
	});

	it('leaves small graphs alone', () => {
		// The guardrail: normal-scale topologies keep today's behaviour exactly.
		expect(shouldCull({ renderedCount: SMALL, measuring: false, exporting: false })).toBe(false);
	});

	it('suspends while measuring, even at scale', () => {
		// The measure pass mounts every node to read its height; culled nodes
		// never mount, so ELK would receive fallback sizes.
		expect(shouldCull({ renderedCount: LARGE, measuring: true, exporting: false })).toBe(false);
	});

	it('suspends while exporting, even at scale', () => {
		// Export rasterises the whole flow element; culling crops the image.
		expect(shouldCull({ renderedCount: LARGE, measuring: false, exporting: true })).toBe(false);
	});

	it('stays suspended when measuring and exporting overlap', () => {
		expect(shouldCull({ renderedCount: LARGE, measuring: true, exporting: true })).toBe(false);
	});
});

/**
 * The level-of-detail tier.
 *
 * These pin the asymmetry a first attempt got wrong by treating every node alike: at the zoom a
 * large L2 graph fits at, a host container is 3.8px on screen and one global boolean gave it the
 * same treatment as a container two orders of magnitude larger.
 */
describe('nodeDetail', () => {
	const base = { detail: false, screenWidth: 200, isSubcontainer: false };

	it('renders everything in full above the detail threshold', () => {
		// Size is irrelevant here: full detail means the card's own text is legible.
		expect(nodeDetail({ ...base, detail: true, screenWidth: 2 })).toBe('full');
	});

	it('drops a grouping container that is only a few pixels across', () => {
		// Its children draw at their own absolute positions, so nothing is lost but the noise.
		expect(nodeDetail({ ...base, isSubcontainer: true, screenWidth: BOX_MIN_PX - 1 })).toBe(
			'hidden'
		);
	});

	it('keeps a grouping container that is big enough to read as a region', () => {
		expect(nodeDetail({ ...base, isSubcontainer: true, screenWidth: BOX_MIN_PX })).toBe('boxed');
	});

	it('keeps an element card at any size', () => {
		// Elements are the graph's texture and their state colour is the one thing that survives at
		// this scale, so they keep a box where a grouping container would be dropped.
		expect(nodeDetail({ ...base, screenWidth: 1 })).toBe('boxed');
	});
});

/**
 * The size gate.
 *
 * Simplifying costs the operator something real — the cards stop saying what they are — so it has
 * to buy something back. On a few hundred nodes there is nothing to reclaim and hiding detail is
 * purely worse, which is why this is gated on the graph and not on the zoom alone.
 */
describe('shouldSimplify', () => {
	// The widest node is a 5,000px root container, 50px on screen at this zoom, so the size gate
	// is what decides these cases.
	const big = {
		zoom: 0.01,
		nodeCount: LOD_MIN_NODES,
		widestNodeWidth: 5_000,
		measuring: false,
		exporting: false
	};

	it('simplifies a large graph once it is zoomed out past the detail threshold', () => {
		expect(shouldSimplify(big)).toBe(true);
	});

	it('leaves a small graph alone however far it is zoomed out', () => {
		// The whole point of the gate: a 200-node graph costs nothing to draw in full, so taking its
		// labels away at low zoom would be a loss with no compensating gain.
		expect(shouldSimplify({ ...big, nodeCount: LOD_MIN_NODES - 1 })).toBe(false);
	});

	it('leaves a large graph alone while it is zoomed in', () => {
		expect(shouldSimplify({ ...big, zoom: DETAIL_ZOOM })).toBe(false);
	});

	it('suspends for the measure pass and for export', () => {
		// Both would otherwise corrupt something silently: the measure pass would hand ELK the
		// pinned heights, and an export would rasterise a page of empty boxes.
		expect(shouldSimplify({ ...big, measuring: true })).toBe(false);
		expect(shouldSimplify({ ...big, exporting: true })).toBe(false);
	});

	describe('when every node is too small to read', () => {
		// 252 collapsed switch cards, the widest 353px: below the node-count gate, and 5.3px wide
		// at the zoom the site fits at.
		const collapsed = { ...big, nodeCount: 252, widestNodeWidth: 353, zoom: 0.015 };

		it('simplifies a graph below the size gate', () => {
			expect(shouldSimplify(collapsed)).toBe(true);
		});

		it('draws in full once the widest node reaches the minimum box width', () => {
			expect(shouldSimplify({ ...collapsed, zoom: BOX_MIN_PX / 353 })).toBe(false);
		});

		it('still suspends for the measure pass and for export', () => {
			expect(shouldSimplify({ ...collapsed, measuring: true })).toBe(false);
			expect(shouldSimplify({ ...collapsed, exporting: true })).toBe(false);
		});

		it('does not simplify before any node has a width', () => {
			expect(shouldSimplify({ ...collapsed, widestNodeWidth: 0 })).toBe(false);
		});
	});
});
