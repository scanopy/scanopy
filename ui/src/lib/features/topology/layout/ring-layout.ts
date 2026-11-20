/**
 * Ring layout: finds rings (closed loops of links) among root containers and places each as a
 * loop, with everything else hanging off the ring node it is attached to.
 *
 * Works on graph structure alone. A ring is a biconnected block that is either one simple cycle,
 * or several chains between the same two nodes (rings that share those two nodes). Every loop has
 * at least `MIN_LOOP` nodes, so the short cycles redundant uplink pairs form are left alone. A
 * block with more than two branch nodes is a mesh, which this leaves to the layered layout.
 *
 * A ring group is drawn as an ellipse at the aspect the other views pack to, so it fills a
 * landscape screen and reads as a ring. A single loop runs all the way round. Rings that share
 * nodes have those nodes on the vertical axis, with the loops as half ellipses alternating right
 * and left of it. Spurs stack inwards, filling the ellipse rather than widening it.
 */

export interface RootGraph {
	/** Node ids in input order, which breaks every tie so the layout is deterministic. */
	nodes: string[];
	adjacency: Map<string, Set<string>>;
}

/** A detected ring group: loops that share `spine`, or a single loop when `spine` is empty. */
export interface RingGroup {
	/**
	 * The shared nodes, top to bottom: the first branch node, the interior of the shortest chain,
	 * the second branch node. Empty for a simple cycle.
	 */
	spine: string[];
	/**
	 * Each loop's own nodes in loop order. With a spine, a loop runs from the node after the
	 * spine's first to the node before its last; without one, it is the whole cycle.
	 */
	loops: string[][];
}

export interface Size {
	width: number;
	height: number;
}

export interface Point {
	x: number;
	y: number;
}

/** Fewest nodes on a loop drawn as a ring. A 4-cycle is a redundant uplink pair, not a ring. */
export const MIN_LOOP = 5;

const RUN_GAP = 75;
// Between stacked cards, as the layered L2 layout keeps them (`elk.spacing.nodeNode`): a card's name
// tag sits above its box, in this gap.
const SPUR_GAP = 50;
const GROUP_GAP = 200;
const PADDING = 25;
const SNAP = 25;

/** A node as the root graph reads it: its id, and its parent when it sits in a container. */
export interface GraphNode {
	id: string;
	isContainer: boolean;
	parentId?: string;
}

/**
 * The root graph of a layout input: root containers, linked wherever an edge joins two of them.
 * An edge end resolves to its root container through `parentMap` first, which covers every node
 * of the view, then through the nodes' own parents. Collapsed containers leave their elements out
 * of `nodes` while edges still name them, so `nodes` alone cannot resolve those ends.
 */
export function rootGraphOf(
	nodes: GraphNode[],
	edges: { source: string; target: string }[],
	parentMap: ReadonlyMap<string, string> = new Map()
): RootGraph {
	const parentOf = new Map(parentMap);
	const roots: string[] = [];
	for (const node of nodes) {
		if (node.parentId) parentOf.set(node.id, node.parentId);
		else if (node.isContainer) roots.push(node.id);
	}
	const rootOf = (id: string): string => {
		let current = id;
		while (parentOf.has(current)) current = parentOf.get(current)!;
		return current;
	};
	return buildRootGraph(
		roots,
		edges.map((edge) => [rootOf(edge.source), rootOf(edge.target)])
	);
}

/** An undirected graph of `nodes`, one edge per linked pair however many links join them. */
export function buildRootGraph(nodes: string[], links: Iterable<[string, string]>): RootGraph {
	const adjacency = new Map<string, Set<string>>(nodes.map((n) => [n, new Set()]));
	for (const [a, b] of links) {
		if (a === b) continue;
		const aSet = adjacency.get(a);
		const bSet = adjacency.get(b);
		if (!aSet || !bSet) continue;
		aSet.add(b);
		bSet.add(a);
	}
	return { nodes, adjacency };
}

/** Biconnected blocks with at least three nodes, as node sets (Tarjan, iterative). */
function biconnectedBlocks(graph: RootGraph): Set<string>[] {
	const index = new Map<string, number>();
	const low = new Map<string, number>();
	const blocks: Set<string>[] = [];
	let counter = 0;

	for (const root of graph.nodes) {
		if (index.has(root)) continue;
		index.set(root, counter);
		low.set(root, counter);
		counter++;
		const edgeStack: [string, string][] = [];
		const stack: { node: string; parent: string | null; neighbours: string[]; next: number }[] = [
			{ node: root, parent: null, neighbours: [...graph.adjacency.get(root)!], next: 0 }
		];
		while (stack.length > 0) {
			const frame = stack[stack.length - 1];
			if (frame.next < frame.neighbours.length) {
				const w = frame.neighbours[frame.next++];
				if (w === frame.parent) continue;
				if (!index.has(w)) {
					edgeStack.push([frame.node, w]);
					index.set(w, counter);
					low.set(w, counter);
					counter++;
					stack.push({
						node: w,
						parent: frame.node,
						neighbours: [...graph.adjacency.get(w)!],
						next: 0
					});
				} else if (index.get(w)! < index.get(frame.node)!) {
					edgeStack.push([frame.node, w]);
					low.set(frame.node, Math.min(low.get(frame.node)!, index.get(w)!));
				}
				continue;
			}
			stack.pop();
			const parent = frame.parent;
			if (parent === null) continue;
			low.set(parent, Math.min(low.get(parent)!, low.get(frame.node)!));
			if (low.get(frame.node)! >= index.get(parent)!) {
				const block = new Set<string>();
				while (edgeStack.length > 0) {
					const [u, v] = edgeStack.pop()!;
					block.add(u);
					block.add(v);
					if (u === parent && v === frame.node) break;
				}
				if (block.size >= 3) blocks.push(block);
			}
		}
	}
	return blocks;
}

/** Walks from `start` through `first` along nodes of block degree 2 until a branch node. */
function walkChain(
	start: string,
	first: string,
	blockAdj: Map<string, string[]>
): { interior: string[]; end: string } {
	const interior: string[] = [];
	let prev = start;
	let current = first;
	while (blockAdj.get(current)!.length === 2 && current !== start) {
		interior.push(current);
		const [x, y] = blockAdj.get(current)!;
		const next = x === prev ? y : x;
		prev = current;
		current = next;
	}
	return { interior, end: current };
}

/** The ring a block forms, or null when it is a mesh or a loop is too short. */
function ringOfBlock(block: Set<string>, graph: RootGraph): RingGroup | null {
	const order = graph.nodes.filter((n) => block.has(n));
	const blockAdj = new Map<string, string[]>(
		order.map((n) => [n, order.filter((m) => graph.adjacency.get(n)!.has(m))])
	);
	const branches = order.filter((n) => blockAdj.get(n)!.length >= 3);

	if (branches.length === 0) {
		if (order.length < MIN_LOOP) return null;
		const start = order[0];
		const { interior } = walkChain(start, blockAdj.get(start)![0], blockAdj);
		return { spine: [], loops: [[start, ...interior]] };
	}
	if (branches.length !== 2) return null;

	const [a, b] = branches;
	const chains: string[][] = [];
	for (const first of blockAdj.get(a)!) {
		if (first === b) {
			chains.push([]);
			continue;
		}
		const { interior, end } = walkChain(a, first, blockAdj);
		if (end !== b) return null;
		chains.push(interior);
	}
	// Shortest chain first, ties in input order (stable sort).
	chains.sort((x, y) => x.length - y.length);
	const [spineInterior, ...loops] = chains;
	if (loops.some((loop) => loop.length + spineInterior.length + 2 < MIN_LOOP)) return null;
	return { spine: [a, ...spineInterior, b], loops };
}

/**
 * Rings in `graph`. Blocks are taken largest first, and a block sharing a node with one already
 * taken is skipped, so no node is on two ring groups.
 */
export function detectRings(graph: RootGraph): RingGroup[] {
	const blocks = biconnectedBlocks(graph).sort((x, y) => y.size - x.size);
	const taken = new Set<string>();
	const rings: RingGroup[] = [];
	for (const block of blocks) {
		if ([...block].some((n) => taken.has(n))) continue;
		const ring = ringOfBlock(block, graph);
		if (!ring) continue;
		rings.push(ring);
		for (const n of block) taken.add(n);
	}
	return rings;
}

/** Every node of a ring group, spine first. */
export function ringNodes(ring: RingGroup): string[] {
	return [...ring.spine, ...ring.loops.flat()];
}

/** The nodes of each loop in the order they are drawn around it, spine included. */
export function loopCycles(ring: RingGroup): string[][] {
	if (ring.spine.length === 0) return ring.loops;
	const reversedSpine = [...ring.spine].reverse();
	return ring.loops.map((loop) => [ring.spine[0], ...loop, ...reversedSpine.slice(0, -1)]);
}

/** Width over height of a drawn ring, the aspect the other views pack their roots to. */
export const RING_ASPECT = 1.6;
const ARC_SAMPLES = 512;

/** A point on a sampled arc and the arc length up to it. */
interface ArcPoint extends Point {
	at: number;
}

/**
 * Samples the ellipse with semi-axes `a` and `b` centred on the origin, clockwise from its top
 * over `sweep` radians: half the ellipse runs from the top through the right to the bottom.
 */
function sampleArc(a: number, b: number, sweep: number): ArcPoint[] {
	const points: ArcPoint[] = [];
	let at = 0;
	for (let i = 0; i <= ARC_SAMPLES; i++) {
		const t = -Math.PI / 2 + (sweep * i) / ARC_SAMPLES;
		const x = a * Math.cos(t);
		const y = b * Math.sin(t);
		if (i > 0) at += Math.hypot(x - points[i - 1].x, y - points[i - 1].y);
		points.push({ x, y, at });
	}
	return points;
}

const arcLength = (arc: ArcPoint[]) => arc[arc.length - 1].at;

/** The points at `fractions` (ascending, 0 to 1) of the way along `arc` by arc length. */
function pointsAt(arc: ArcPoint[], fractions: number[]): Point[] {
	const total = arcLength(arc);
	let j = 0;
	return fractions.map((fraction) => {
		const s = total * fraction;
		while (j < arc.length - 2 && arc[j + 1].at < s) j++;
		const p = arc[j];
		const q = arc[j + 1];
		const f = q.at > p.at ? Math.min(1, Math.max(0, (s - p.at) / (q.at - p.at))) : 0;
		return { x: p.x + (q.x - p.x) * f, y: p.y + (q.y - p.y) * f };
	});
}

/**
 * `count` points evenly spaced by arc length along `arc`, leaving a gap at each end for the nodes
 * it runs between.
 */
function spreadAlong(arc: ArcPoint[], count: number, open: boolean): Point[] {
	const gaps = open ? count + 1 : count;
	return pointsAt(
		arc,
		Array.from({ length: count }, (_, i) => (open ? i + 1 : i) / gaps)
	);
}

/**
 * Places the components of `graph` that hold a ring: each ring group as an ellipse of
 * `RING_ASPECT`, and every other node of those components in a spur stacked from its nearest ring
 * node towards the ellipse's centre. Ring groups sit side by side. Nodes of components with no
 * ring get no position. Returns null when the graph has no ring, so the caller keeps its layout.
 */
export function placeRings(
	graph: RootGraph,
	rings: RingGroup[],
	sizes: Map<string, Size>
): Map<string, Point> | null {
	if (rings.length === 0) return null;
	const size = (id: string): Size => sizes.get(id) ?? { width: 0, height: 0 };

	// Spur trees: a multi-source BFS from every ring node, in ring then input order.
	const owner = new Set<string>();
	const children = new Map<string, string[]>();
	const queue: string[] = [];
	for (const ring of rings) {
		for (const n of ringNodes(ring)) {
			owner.add(n);
			queue.push(n);
		}
	}
	for (let i = 0; i < queue.length; i++) {
		const node = queue[i];
		const neighbours = graph.nodes.filter((n) => graph.adjacency.get(node)!.has(n));
		for (const next of neighbours) {
			if (owner.has(next)) continue;
			owner.add(next);
			if (!children.has(node)) children.set(node, []);
			children.get(node)!.push(next);
			queue.push(next);
		}
	}

	// A node's spur: its subtree, depth first, as one stack of cards.
	const spurOf = (node: string): string[] => {
		const out: string[] = [];
		const visit = (n: string) => {
			for (const c of children.get(n) ?? []) {
				out.push(c);
				visit(c);
			}
		};
		visit(node);
		return out;
	};
	/** How far a spur reaches from its ring node, whichever way it points. */
	const reachOf = (node: string) =>
		spurOf(node).reduce(
			(r, n) => r + Math.max(size(n).width, size(n).height) + SPUR_GAP,
			Math.max(size(node).width, size(node).height) / 2
		);

	/**
	 * Distance between the centres of cards `a` and `b` stacked along unit direction `u`, so that
	 * `gap` separates them on at least one axis. Each axis is judged for the pair: a tall
	 * card cleared on x beside a flat one cleared on y would otherwise be cleared on neither.
	 */
	const separation = (a: string, b: string, u: Point, gap = SPUR_GAP) => {
		const sa = size(a);
		const sb = size(b);
		const onX = (sa.width + sb.width) / 2 + gap;
		const onY = (sa.height + sb.height) / 2 + gap;
		const byX = Math.abs(u.x) > 1e-9 ? onX / Math.abs(u.x) : Infinity;
		const byY = Math.abs(u.y) > 1e-9 ? onY / Math.abs(u.y) : Infinity;
		return Math.min(byX, byY);
	};

	const centres = new Map<string, Point>();
	// The ring node each spur card stacks from, so the stack moves with it when it snaps to the grid.
	const anchorOf = new Map<string, string>();
	/** Centres `node` on `c` and stacks its spur from it along `u`. */
	const placeNode = (node: string, c: Point, u: Point) => {
		centres.set(node, c);
		let previous = node;
		let distance = 0;
		for (const n of spurOf(node)) {
			distance += separation(previous, n, u);
			centres.set(n, { x: c.x + u.x * distance, y: c.y + u.y * distance });
			anchorOf.set(n, node);
			previous = n;
		}
	};
	const towardCentre = (c: Point): Point => {
		const d = Math.hypot(c.x, c.y);
		return d > 0 ? { x: -c.x / d, y: -c.y / d } : { x: 0, y: 1 };
	};

	/**
	 * Whether any two placed cards overlap or come closer than the grid snap could close. Sweeps
	 * the cards by their left edge, so only cards that share a vertical band are compared.
	 */
	const cardsCollide = () => {
		const margin = SNAP / 2 - 1;
		const boxes = [...centres].map(([n, c]) => ({
			left: c.x - size(n).width / 2 - margin,
			right: c.x + size(n).width / 2 + margin,
			top: c.y - size(n).height / 2 - margin,
			bottom: c.y + size(n).height / 2 + margin
		}));
		boxes.sort((p, q) => p.left - q.left);
		for (let i = 0; i < boxes.length; i++) {
			for (let j = i + 1; j < boxes.length && boxes[j].left < boxes[i].right; j++) {
				if (boxes[i].top < boxes[j].bottom && boxes[j].top < boxes[i].bottom) return true;
			}
		}
		return false;
	};

	const positions = new Map<string, Point>();
	let groupX = 0;
	for (const ring of rings) {
		const all = ringNodes(ring);
		// A further loop on one side of the spine is spaced by the largest card diagonal, which
		// clears two cards whatever direction the ellipse runs between them.
		const step = Math.max(...all.map((n) => Math.hypot(size(n).width, size(n).height))) + RUN_GAP;

		/**
		 * Where along `unit` (an arc at scale 1) each node of `chain` sits, as fractions of its
		 * length, and how long the arc must be. Each pair of neighbours gets the room it needs in
		 * the direction the arc runs between them: side by side at the top, stacked at the sides.
		 * The direction depends on where the pair sits, which depends on the spacing, so it is
		 * refined a few times. A closed chain wraps from its last node to its first.
		 */
		const fitChain = (unit: ArcPoint[], chain: string[], closed: boolean) => {
			const gaps = closed ? chain.length : chain.length - 1;
			let fractions = chain.map((_, i) => i / gaps);
			let length = 0;
			for (let pass = 0; pass < 4; pass++) {
				const points = pointsAt(unit, fractions);
				const needs = Array.from({ length: gaps }, (_, k) => {
					const p = points[k];
					const q = points[(k + 1) % chain.length];
					const d = Math.hypot(q.x - p.x, q.y - p.y) || 1;
					const u = { x: (q.x - p.x) / d, y: (q.y - p.y) / d };
					// A chord is a little shorter than the arc it spans.
					return separation(chain[k], chain[(k + 1) % chain.length], u, RUN_GAP) * 1.02;
				});
				length = needs.reduce((s, g) => s + g, 0);
				let at = 0;
				fractions = chain.map((_, i) => {
					const f = at / length;
					if (i < needs.length) at += needs[i];
					return f;
				});
			}
			return { fractions, length };
		};

		// Drawn at the smallest size that fits the ring nodes, then larger until the spurs inside it
		// clear each other: a small ring has little room in the middle for long spurs.
		const draw = (scale: number) => {
			centres.clear();
			if (ring.spine.length === 0) {
				const [loop] = ring.loops;
				const unit = sampleArc(RING_ASPECT, 1, 2 * Math.PI);
				const { fractions, length } = fitChain(unit, loop, true);
				const b = (scale * length) / arcLength(unit);
				pointsAt(sampleArc(RING_ASPECT * b, b, 2 * Math.PI), fractions).forEach((c, i) =>
					placeNode(loop[i], c, towardCentre(c))
				);
			} else {
				// The spine is the vertical axis; loops alternate right and left of it, each a half
				// ellipse from the spine's first node at the top to its last at the bottom.
				const spine = ring.spine;
				const sides = [0, 1].map((side) => ring.loops.filter((_, i) => i % 2 === side));
				const unit = sampleArc(RING_ASPECT, 1, Math.PI);
				const fitted = sides.map((loops) =>
					loops[0] ? fitChain(unit, [spine[0], ...loops[0], spine[spine.length - 1]], false) : null
				);
				const down = { x: 0, y: 1 };
				const spineNeeds = spine.slice(1).map((n, i) => separation(spine[i], n, down, RUN_GAP));
				const spineLength = spineNeeds.reduce((s, g) => s + g, 0);
				const b =
					scale *
					Math.max(
						...fitted.map((fit) => (fit ? fit.length / arcLength(unit) : 0)),
						spineLength / 2
					);

				let y = -b;
				spine.forEach((n, i) => {
					if (i > 0) y += (2 * b * spineNeeds[i - 1]) / spineLength;
					const c = { x: 0, y: i === spine.length - 1 ? b : y };
					const u =
						i === 0 ? { x: 0, y: -1 } : i === spine.length - 1 ? { x: 0, y: 1 } : { x: 1, y: 0 };
					placeNode(n, c, u);
				});

				sides.forEach((loops, side) => {
					const mirror = side === 0 ? 1 : -1;
					let a = 0;
					loops.forEach((loop, k) => {
						let points: Point[];
						if (k === 0) {
							a = RING_ASPECT * b;
							// The fit includes the two spine ends, which are already placed.
							points = pointsAt(sampleArc(a, b, Math.PI), fitted[side]!.fractions).slice(1, -1);
						} else {
							// A further loop on a side runs outside the previous one, far enough out
							// for its own spurs to fit in between.
							a += step + Math.max(0, ...loop.map((n) => reachOf(n)));
							let arc = sampleArc(a, b, Math.PI);
							while (arcLength(arc) < (loop.length + 1) * step) {
								a *= 1.1;
								arc = sampleArc(a, b, Math.PI);
							}
							points = spreadAlong(arc, loop.length, true);
						}
						points.forEach((p, i) => {
							const c = { x: p.x * mirror, y: p.y };
							placeNode(loop[i], c, towardCentre(c));
						});
					});
				});
			}
		};
		let scale = 1;
		draw(scale);
		// Twelve tries is five times the size: a ring that still collides there has a spur too
		// long for any ellipse, and growing it further only shrinks the whole map out of view.
		for (let tries = 0; tries < 12 && cardsCollide(); tries++) {
			scale *= 1.15;
			draw(scale);
		}

		// Cards are placed by their top-left corner, and the group starts right of the last one.
		let minX = Infinity;
		let maxX = -Infinity;
		for (const [n, c] of centres) {
			minX = Math.min(minX, c.x - size(n).width / 2);
			maxX = Math.max(maxX, c.x + size(n).width / 2);
		}
		for (const [n, c] of centres) {
			positions.set(n, {
				x: c.x - size(n).width / 2 - minX + groupX,
				y: c.y - size(n).height / 2
			});
		}
		groupX += maxX - minX + GROUP_GAP;
	}

	// Normalize to the padding and snap each ring node to the grid. A spur card moves by the same
	// amount as the ring node it stacks from, so the gaps within a stack stay as placed.
	const allMinX = Math.min(...[...positions.values()].map((p) => p.x));
	const allMinY = Math.min(...[...positions.values()].map((p) => p.y));
	const shift = (p: Point): Point => ({ x: p.x - allMinX + PADDING, y: p.y - allMinY + PADDING });
	const snapped = new Map<string, Point>();
	for (const [n, p] of positions) {
		if (anchorOf.has(n)) continue;
		const q = shift(p);
		snapped.set(n, { x: Math.round(q.x / SNAP) * SNAP, y: Math.round(q.y / SNAP) * SNAP });
	}
	for (const [n, p] of positions) {
		const anchor = anchorOf.get(n);
		if (anchor === undefined) continue;
		const q = shift(p);
		const a = shift(positions.get(anchor)!);
		const s = snapped.get(anchor)!;
		snapped.set(n, { x: Math.round(q.x + s.x - a.x), y: Math.round(q.y + s.y - a.y) });
	}
	return snapped;
}
