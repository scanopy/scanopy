import { describe, it, expect } from 'vitest';
import {
	computeOptimalHandles,
	ObstacleGrid,
	type ObstacleRect
} from '#lib/features/topology/layout/handle-picking.ts';

/** Deterministic PRNG so a failure reproduces. */
function rng(seed: number) {
	let s = seed >>> 0;
	return () => {
		s = (s * 1664525 + 1013904223) >>> 0;
		return s / 0x100000000;
	};
}

type Rect = ObstacleRect & { id: string };

/** Cards of mixed sizes plus a few large containers, the mix `getAllNodeRects` returns. */
function randomLayout(seed: number, count: number): Rect[] {
	const r = rng(seed);
	const rects: Rect[] = [];
	for (let i = 0; i < count; i++) {
		rects.push({
			id: `n${i}`,
			x: Math.round(r() * 5000),
			y: Math.round(r() * 3000),
			w: 50 + Math.round(r() * 250),
			h: 30 + Math.round(r() * 150)
		});
	}
	for (let i = 0; i < 5; i++) {
		rects.push({
			id: `c${i}`,
			x: Math.round(r() * 3000),
			y: Math.round(r() * 2000),
			w: 800 + Math.round(r() * 1500),
			h: 500 + Math.round(r() * 1000)
		});
	}
	return rects;
}

describe('computeOptimalHandles', () => {
	it('leaves a card from a side facing the other end, not one that doubles back across it', () => {
		// An L3 ContainerRuntime edge from a workstation address in a LAN device type group down to a
		// Docker bridge subnet below and to the left, with the cards and groups between them. Its
		// Top→Right chord ran back through its own card, which the endpoint-excluded obstacle count
		// left free, so it beat the shorter Bottom chord that clipped one more IoT card.
		const card = (id: string, x: number, y: number): Rect => ({ id, x, y, w: 120, h: 95 });
		const group = (id: string, x: number, y: number, w: number, h: number): Rect => ({
			id,
			x,
			y,
			w,
			h
		});
		const obstacles: Rect[] = [
			group('printer', 375, 50, 170, 170),
			group('router', 575, 50, 170, 170),
			group('access-point', 750, 50, 170, 170),
			group('camera', 950, 50, 170, 170),
			group('media-player', 1150, 50, 315, 170),
			group('tablet', 225, 250, 315, 170),
			group('mobile', 575, 250, 460, 170),
			group('speaker', 225, 450, 605, 170),
			group('iot', 225, 625, 895, 290),
			group('authentik', 50, 1175, 460, 170),
			card('.34', 225, 50),
			card('.48', 400, 100),
			card('.1', 600, 100),
			card('.30', 775, 100),
			card('.21', 975, 100),
			card('.29', 1175, 100),
			card('.27', 1320, 100),
			card('.31', 250, 300),
			card('.37', 395, 300),
			card('.36', 600, 300),
			card('.23', 745, 300),
			card('.33', 890, 300),
			card('.25', 1220, 300),
			card('.22', 1365, 300),
			card('.206', 250, 500),
			card('.41', 395, 500),
			card('.20', 540, 500),
			card('.32', 685, 500),
			card('.43', 250, 675),
			card('.46', 395, 675),
			card('.45', 540, 675),
			card('.35', 685, 675),
			card('.44', 830, 675),
			card('.250', 975, 675),
			card('.149', 250, 795),
			card('.42', 395, 795),
			card('.246', 540, 795),
			card('.38', 685, 795),
			card('.24', 830, 795),
			card('172.17.0.2', 50, 1050),
			card('172.18.0.4', 75, 1225),
			card('172.18.0.2', 220, 1225),
			card('172.18.0.3', 365, 1225)
		];

		const handles = computeOptimalHandles(
			{ x: 1075, y: 300 },
			{ w: 120, h: 95 },
			{ x: 25, y: 1025 },
			{ w: 510, h: 340 },
			obstacles
		);

		expect(handles.sourceHandle).not.toBe('Top');
	});
});

describe('ObstacleGrid', () => {
	it('picks the same handles as testing every rect, across random layouts', () => {
		for (let seed = 1; seed <= 20; seed++) {
			const rects = randomLayout(seed, 300);
			const grid = new ObstacleGrid(rects);
			const r = rng(seed * 7919);
			for (let e = 0; e < 50; e++) {
				const src = rects[Math.floor(r() * rects.length)];
				const tgt = rects[Math.floor(r() * rects.length)];
				if (src === tgt) continue;
				// Some edges also exclude an unrelated rect, standing in for an ancestor container.
				const exclude = new Set([src.id, tgt.id]);
				if (r() < 0.5) exclude.add(rects[Math.floor(r() * rects.length)].id);

				const srcPos = { x: src.x, y: src.y };
				const srcSize = { w: src.w, h: src.h };
				const tgtPos = { x: tgt.x, y: tgt.y };
				const tgtSize = { w: tgt.w, h: tgt.h };

				const brute = computeOptimalHandles(
					srcPos,
					srcSize,
					tgtPos,
					tgtSize,
					rects.filter((rect) => !exclude.has(rect.id))
				);
				const indexed = computeOptimalHandles(srcPos, srcSize, tgtPos, tgtSize, (x1, y1, x2, y2) =>
					grid.countCrossings(x1, y1, x2, y2, exclude)
				);
				expect(indexed, `seed ${seed}, ${src.id} -> ${tgt.id}`).toEqual(brute);
			}
		}
	});

	it('counts crossings by segments that run along cell boundaries', () => {
		// A grid of identical cards, so cell edges and card edges line up.
		const rects: Rect[] = [];
		for (let i = 0; i < 10; i++) {
			for (let j = 0; j < 10; j++) {
				rects.push({ id: `${i}-${j}`, x: i * 200, y: j * 200, w: 100, h: 100 });
			}
		}
		const grid = new ObstacleGrid(rects);
		const none = new Set<string>();
		// Horizontal along the top edge of row 3: touches all ten cards in that row.
		expect(grid.countCrossings(0, 600, 1900, 600, none)).toBe(10);
		// Vertical along the left edge of column 2.
		expect(grid.countCrossings(400, 0, 400, 1900, none)).toBe(10);
		// Through the gap between rows: touches nothing.
		expect(grid.countCrossings(0, 150, 1900, 150, none)).toBe(0);
	});
});
