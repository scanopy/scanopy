import { describe, it, expect } from 'vitest';
import {
	buildRootGraph,
	detectRings,
	loopCycles,
	placeRings,
	ringNodes,
	rootGraphOf,
	type Point,
	type RootGraph,
	type Size
} from '#lib/features/topology/layout/ring-layout.ts';

/** A graph of the nodes the links name, in first-mention order. */
function graphOf(links: [string, string][]): RootGraph {
	const nodes = [...new Set(links.flat())];
	return buildRootGraph(nodes, links);
}

/** A path a-b-c-… as links. */
function path(...nodes: string[]): [string, string][] {
	return nodes.slice(1).map((n, i) => [nodes[i], n]);
}

function cycle(...nodes: string[]): [string, string][] {
	return [...path(...nodes), [nodes[nodes.length - 1], nodes[0]]];
}

/** Two rings of `n` stations each between cores A and B, which also link to each other. */
function twoRings(n: number): [string, string][] {
	const east = Array.from({ length: n }, (_, i) => `e${i}`);
	const west = Array.from({ length: n }, (_, i) => `w${i}`);
	return [['A', 'B'], ...path('A', ...east, 'B'), ...path('A', ...west, 'B')];
}

const CARD: Size = { width: 200, height: 120 };

function place(links: [string, string][]) {
	const graph = graphOf(links);
	const rings = detectRings(graph);
	const sizes = new Map(graph.nodes.map((n) => [n, CARD]));
	const positions = placeRings(graph, rings, sizes)!;
	return { graph, rings, positions };
}

function centre(p: Point): Point {
	return { x: p.x + CARD.width / 2, y: p.y + CARD.height / 2 };
}

/** Whether segments pq and rs cross at a point inside both. */
function segmentsCross(p: Point, q: Point, r: Point, s: Point): boolean {
	const orient = (a: Point, b: Point, c: Point) =>
		Math.sign((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x));
	return orient(p, q, r) * orient(p, q, s) < 0 && orient(r, s, p) * orient(r, s, q) < 0;
}

/** Crossings between a loop's own links, drawn centre to centre in loop order. */
function crossingsWithin(loop: string[], positions: Map<string, Point>): number {
	const segments = loop.map((n, i) => [
		centre(positions.get(n)!),
		centre(positions.get(loop[(i + 1) % loop.length])!)
	]);
	let crossings = 0;
	for (let i = 0; i < segments.length; i++) {
		for (let j = i + 2; j < segments.length; j++) {
			if (i === 0 && j === segments.length - 1) continue;
			if (segmentsCross(segments[i][0], segments[i][1], segments[j][0], segments[j][1])) {
				crossings++;
			}
		}
	}
	return crossings;
}

function overlaps(ids: string[], positions: Map<string, Point>): number {
	let count = 0;
	for (let i = 0; i < ids.length; i++) {
		for (let j = i + 1; j < ids.length; j++) {
			const a = positions.get(ids[i])!;
			const b = positions.get(ids[j])!;
			if (
				a.x < b.x + CARD.width &&
				b.x < a.x + CARD.width &&
				a.y < b.y + CARD.height &&
				b.y < a.y + CARD.height
			) {
				count++;
			}
		}
	}
	return count;
}

describe('detectRings', () => {
	it('finds one ring as a single loop in cycle order', () => {
		const rings = detectRings(graphOf(cycle('a', 'b', 'c', 'd', 'e', 'f')));
		expect(rings).toHaveLength(1);
		expect(rings[0].spine).toEqual([]);
		const [loop] = loopCycles(rings[0]);
		expect(new Set(loop)).toEqual(new Set(['a', 'b', 'c', 'd', 'e', 'f']));
		// Consecutive loop nodes are linked.
		const graph = graphOf(cycle('a', 'b', 'c', 'd', 'e', 'f'));
		loop.forEach((n, i) => {
			expect(graph.adjacency.get(n)!.has(loop[(i + 1) % loop.length])).toBe(true);
		});
	});

	it('finds two rings that share two nodes, joined at those nodes', () => {
		const rings = detectRings(graphOf(twoRings(6)));
		expect(rings).toHaveLength(1);
		expect(rings[0].spine).toEqual(['A', 'B']);
		expect(rings[0].loops.map((l) => [...l].sort())).toEqual([
			['e0', 'e1', 'e2', 'e3', 'e4', 'e5'],
			['w0', 'w1', 'w2', 'w3', 'w4', 'w5']
		]);
	});

	it('finds no ring in a tree', () => {
		const links: [string, string][] = [
			['core', 'd1'],
			['core', 'd2'],
			['d1', 'a1'],
			['d1', 'a2'],
			['d2', 'a3']
		];
		expect(detectRings(graphOf(links))).toEqual([]);
	});

	it('finds the ring and leaves its spurs off it', () => {
		const links: [string, string][] = [
			...cycle('a', 'b', 'c', 'd', 'e'),
			['a', 'host1'],
			['a', 'host2'],
			['c', 'sw'],
			['sw', 'host3']
		];
		const rings = detectRings(graphOf(links));
		expect(rings).toHaveLength(1);
		expect(new Set(ringNodes(rings[0]))).toEqual(new Set(['a', 'b', 'c', 'd', 'e']));
	});

	it('finds no ring in a dual-homed core and distribution mesh', () => {
		// Two cores, four distribution switches on both, access switches each on two of them.
		const links: [string, string][] = [['c1', 'c2']];
		for (const d of ['d1', 'd2', 'd3', 'd4']) links.push(['c1', d], ['c2', d]);
		for (let a = 0; a < 8; a++) {
			links.push([`a${a}`, `d${(a % 4) + 1}`], [`a${a}`, `d${((a + 2) % 4) + 1}`]);
		}
		expect(detectRings(graphOf(links))).toEqual([]);
	});

	it('finds no ring in a 4-cycle of redundant uplinks', () => {
		expect(detectRings(graphOf(cycle('c1', 'd1', 'c2', 'd2')))).toEqual([]);
	});
});

describe('rootGraphOf', () => {
	// Five hosts in a ring, each with two ports, linked port to port.
	const hosts = ['h0', 'h1', 'h2', 'h3', 'h4'];
	const ports = hosts.flatMap((h) => [`${h}/a`, `${h}/b`]);
	const links = hosts.map((h, i) => ({ source: `${h}/b`, target: `${hosts[(i + 1) % 5]}/a` }));
	const parentMap = new Map(ports.map((p) => [p, p.split('/')[0]]));

	it('finds the ring when every port is in the layout input', () => {
		const nodes = [
			...hosts.map((id) => ({ id, isContainer: true })),
			...ports.map((id) => ({ id, isContainer: false, parentId: id.split('/')[0] }))
		];
		expect(detectRings(rootGraphOf(nodes, links))).toHaveLength(1);
	});

	it('finds the ring when collapsed hosts leave their ports out of the input', () => {
		const nodes = hosts.map((id) => ({ id, isContainer: true }));
		expect(detectRings(rootGraphOf(nodes, links, parentMap))).toHaveLength(1);
	});
});

describe('placeRings', () => {
	it('keeps the caller layout when there is no ring', () => {
		const graph = graphOf(path('a', 'b', 'c'));
		expect(placeRings(graph, detectRings(graph), new Map())).toBeNull();
	});

	it('draws each loop without crossings in its own links', () => {
		for (const links of [cycle('a', 'b', 'c', 'd', 'e'), cycle(...'abcdefghijk'), twoRings(7)]) {
			const { rings, positions } = place(links);
			for (const loop of loopCycles(rings[0])) {
				expect(crossingsWithin(loop, positions)).toBe(0);
			}
		}
	});

	it('places ring nodes around the loop in loop order', () => {
		const { rings, positions } = place(twoRings(8));
		for (const loop of loopCycles(rings[0])) {
			// Walking the loop, the angle around its centre turns one way the whole time.
			const centres = loop.map((n) => centre(positions.get(n)!));
			const cx = centres.reduce((s, p) => s + p.x, 0) / centres.length;
			const cy = centres.reduce((s, p) => s + p.y, 0) / centres.length;
			const angles = centres.map((p) => Math.atan2(p.y - cy, p.x - cx));
			const turns = angles.map((a, i) => {
				let d = angles[(i + 1) % angles.length] - a;
				while (d <= -Math.PI) d += 2 * Math.PI;
				while (d > Math.PI) d -= 2 * Math.PI;
				return Math.sign(d);
			});
			expect(new Set(turns.filter((t) => t !== 0)).size).toBe(1);
		}
	});

	it('puts the two rings on opposite sides of the shared nodes', () => {
		const { positions } = place(twoRings(6));
		const a = positions.get('A')!.x;
		const eastX = Array.from({ length: 6 }, (_, i) => positions.get(`e${i}`)!.x);
		const westX = Array.from({ length: 6 }, (_, i) => positions.get(`w${i}`)!.x);
		const eastSide = Math.sign(eastX[0] - a);
		const westSide = Math.sign(westX[0] - a);
		expect(eastSide).not.toBe(0);
		expect(eastSide).toBe(-westSide);
		expect(eastX.every((x) => Math.sign(x - a) === eastSide)).toBe(true);
		expect(westX.every((x) => Math.sign(x - a) === westSide)).toBe(true);
	});

	it('places every node, ring and spur, without overlaps', () => {
		const links: [string, string][] = [...twoRings(6)];
		for (let i = 0; i < 6; i++) {
			for (let h = 0; h < 4; h++) links.push([`e${i}`, `e${i}-h${h}`], [`w${i}`, `w${i}-h${h}`]);
		}
		links.push(['A', 'server1'], ['A', 'server2'], ['B', 'server3']);
		const { graph, positions } = place(links);
		expect(positions.size).toBe(graph.nodes.length);
		expect(overlaps(graph.nodes, positions)).toBe(0);
	});

	it('draws two rings of a utility scale as one ellipse, with spurs inside it', () => {
		const links: [string, string][] = [...twoRings(49)];
		const spurs: string[] = [];
		for (let i = 0; i < 49; i++) {
			for (let h = 0; h < 3 + (i % 4); h++) {
				links.push([`e${i}`, `e${i}-h${h}`], [`w${i}`, `w${i}-h${h}`]);
				spurs.push(`e${i}-h${h}`, `w${i}-h${h}`);
			}
		}
		links.push(['A', 'server1'], ['A', 'server2'], ['B', 'server3'], ['B', 'server4']);
		const { graph, rings, positions } = place(links);

		const ring = ringNodes(rings[0]);
		const xs = ring.map((n) => positions.get(n)!.x);
		const ys = ring.map((n) => positions.get(n)!.y);
		const box = {
			left: Math.min(...xs),
			right: Math.max(...xs) + CARD.width,
			top: Math.min(...ys),
			bottom: Math.max(...ys) + CARD.height
		};
		const aspect = (box.right - box.left) / (box.bottom - box.top);
		expect(aspect).toBeGreaterThan(1.4);
		expect(aspect).toBeLessThan(1.8);

		// No bigger than it needs to be. The smallest ellipse whose half holds 50 gaps of a card
		// diagonal plus 75px is about 12,100px wide; two growth steps allow 1.32 times that.
		expect(box.right - box.left).toBeLessThan(12_100 * 1.32);

		for (const n of spurs) {
			const p = positions.get(n)!;
			expect(p.x).toBeGreaterThanOrEqual(box.left);
			expect(p.x + CARD.width).toBeLessThanOrEqual(box.right);
			expect(p.y).toBeGreaterThanOrEqual(box.top);
			expect(p.y + CARD.height).toBeLessThanOrEqual(box.bottom);
		}
		expect(overlaps(graph.nodes, positions)).toBe(0);
	});

	it('leaves components with no ring unplaced', () => {
		const links: [string, string][] = [...cycle('a', 'b', 'c', 'd', 'e'), ['x', 'y']];
		const graph = graphOf(links);
		const sizes = new Map(graph.nodes.map((n) => [n, CARD]));
		const positions = placeRings(graph, detectRings(graph), sizes)!;
		expect([...positions.keys()].sort()).toEqual(['a', 'b', 'c', 'd', 'e']);
	});
});
