import { describe, it, expect } from 'vitest';
import { packBoxes, packComponents, type Box } from '#lib/features/topology/layout/box-packing.ts';
import { buildRootGraph } from '#lib/features/topology/layout/ring-layout.ts';

const CARD = { width: 250, height: 100 };

function overlapping(boxes: Box[]): number {
	let count = 0;
	for (let i = 0; i < boxes.length; i++) {
		for (let j = i + 1; j < boxes.length; j++) {
			const [a, b] = [boxes[i], boxes[j]];
			if (
				a.x < b.x + b.width &&
				b.x < a.x + a.width &&
				a.y < b.y + b.height &&
				b.y < a.y + a.height
			) {
				count++;
			}
		}
	}
	return count;
}

function aspectOf(boxes: Box[]): number {
	const left = Math.min(...boxes.map((b) => b.x));
	const right = Math.max(...boxes.map((b) => b.x + b.width));
	const top = Math.min(...boxes.map((b) => b.y));
	const bottom = Math.max(...boxes.map((b) => b.y + b.height));
	return (right - left) / (bottom - top);
}

describe('packBoxes', () => {
	it('tiles equal boxes into a landscape rectangle without overlaps', () => {
		const placed = new Map<string, Box>();
		packBoxes(
			placed,
			Array.from({ length: 30 }, (_, i) => ({ id: `h${i}`, ...CARD })),
			1.6,
			75
		);
		const boxes = [...placed.values()];
		expect(boxes).toHaveLength(30);
		expect(overlapping(boxes)).toBe(0);
		expect(aspectOf(boxes)).toBeGreaterThan(1.2);
		expect(aspectOf(boxes)).toBeLessThan(2.0);
	});

	it('never moves a box already placed', () => {
		const fixed = { x: 400, y: 300, width: 1000, height: 600 };
		const placed = new Map<string, Box>([['fixed', { ...fixed }]]);
		packBoxes(
			placed,
			Array.from({ length: 10 }, (_, i) => ({ id: `h${i}`, ...CARD })),
			1.6,
			75
		);
		expect(placed.get('fixed')).toEqual(fixed);
		expect(overlapping([...placed.values()])).toBe(0);
	});
});

describe('packComponents', () => {
	it('tiles unlinked hosts beside a linked group, which keeps its own layout', () => {
		// A chain of five switches laid out in one column, and 30 hosts with no link, all stacked
		// in a second column the way a layered layout leaves them.
		const chain = ['s0', 's1', 's2', 's3', 's4'];
		const hosts = Array.from({ length: 30 }, (_, i) => `h${i}`);
		const graph = buildRootGraph(
			[...chain, ...hosts],
			chain.slice(1).map((s, i): [string, string] => [chain[i], s])
		);
		const positions = new Map([
			...chain.map((id, i): [string, { x: number; y: number }] => [id, { x: 0, y: i * 150 }]),
			...hosts.map((id, i): [string, { x: number; y: number }] => [id, { x: 400, y: i * 150 }])
		]);
		const sizes = new Map([...chain, ...hosts].map((id) => [id, CARD]));

		const moved = packComponents(graph, positions, sizes, 1.6, 75);
		const boxes = [...moved].map(([id, p]) => ({ ...p, ...sizes.get(id)! }));
		expect(boxes).toHaveLength(35);
		expect(overlapping(boxes)).toBe(0);
		expect(aspectOf(boxes)).toBeGreaterThan(1.2);
		expect(aspectOf(boxes)).toBeLessThan(2.0);

		const dx = moved.get('s0')!.x - positions.get('s0')!.x;
		const dy = moved.get('s0')!.y - positions.get('s0')!.y;
		for (const id of chain) {
			expect(moved.get(id)).toEqual({ x: positions.get(id)!.x + dx, y: positions.get(id)!.y + dy });
		}
	});

	it('only moves a single connected group as a whole', () => {
		const ids = ['a', 'b', 'c'];
		const graph = buildRootGraph(ids, [
			['a', 'b'],
			['b', 'c']
		]);
		const positions = new Map([
			['a', { x: 25, y: 25 }],
			['b', { x: 400, y: 200 }],
			['c', { x: 25, y: 400 }]
		]);
		const sizes = new Map(ids.map((id) => [id, CARD]));
		const moved = packComponents(graph, positions, sizes, 1.6, 75);
		const dx = moved.get('a')!.x - 25;
		const dy = moved.get('a')!.y - 25;
		for (const id of ids) {
			expect(moved.get(id)).toEqual({ x: positions.get(id)!.x + dx, y: positions.get(id)!.y + dy });
		}
	});
});
