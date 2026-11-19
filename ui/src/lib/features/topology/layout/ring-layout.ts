/**
 * Ring layout: finds rings (closed loops of links) among root containers and places each as a
 * loop, with everything else hanging off the ring node it is attached to.
 *
 * Works on graph structure alone. A ring is a biconnected block that is either one simple cycle,
 * or several chains between the same two nodes (rings that share those two nodes). Every loop has
 * at least `MIN_LOOP` nodes, so the short cycles redundant uplink pairs form are left alone. A
 * block with more than two branch nodes is a mesh, which this leaves to the layered layout.
 *
 * Each loop is drawn as a racetrack: a top run and a bottom run joined at the far end, so cards
 * stay axis-aligned and the links along a run read left to right. Rings that share two nodes have
 * those nodes stacked in a central column, with the loops alternating right and left of it.
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
const ROW_GAP = 150;
const SPUR_GAP = 25;
const SPUR_INDENT = 50;
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

type Direction = 'up' | 'down' | 'right';

/**
 * Places `graph` with each ring group as loops and every other node of a ring group's component
 * hanging off its nearest ring node. Nodes in components with no ring keep `fallback`, moved below
 * the rings. Returns null when the graph has no ring, so the caller keeps its own layout untouched.
 */
export function placeRings(
	graph: RootGraph,
	rings: RingGroup[],
	sizes: Map<string, Size>,
	fallback: Map<string, Point>
): Map<string, Point> | null {
	if (rings.length === 0) return null;
	const size = (id: string): Size => sizes.get(id) ?? { width: 0, height: 0 };

	// Spur trees: a multi-source BFS from every ring node, in ring then input order.
	const owner = new Map<string, number>();
	const children = new Map<string, string[]>();
	const queue: string[] = [];
	rings.forEach((ring, r) => {
		for (const n of ringNodes(ring)) {
			owner.set(n, r);
			queue.push(n);
		}
	});
	for (let i = 0; i < queue.length; i++) {
		const node = queue[i];
		const neighbours = graph.nodes.filter((n) => graph.adjacency.get(node)!.has(n));
		for (const next of neighbours) {
			if (owner.has(next)) continue;
			owner.set(next, owner.get(node)!);
			if (!children.has(node)) children.set(node, []);
			children.get(node)!.push(next);
			queue.push(next);
		}
	}

	// A node's spur: its subtree, depth first, as one column of cards.
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
	const spurExtent = (spur: string[]): Size => ({
		width: spur.length ? Math.max(...spur.map((n) => size(n).width)) + SPUR_INDENT : 0,
		height: spur.reduce((h, n) => h + size(n).height + SPUR_GAP, 0)
	});

	const positions = new Map<string, Point>();

	/** Places `node` at (x, y) and its spur outward in `direction`. */
	const placeWithSpur = (node: string, x: number, y: number, direction: Direction) => {
		positions.set(node, { x, y });
		const own = size(node);
		const spur = spurOf(node);
		let cursor =
			direction === 'up' ? y - SPUR_GAP : direction === 'down' ? y + own.height + SPUR_GAP : y;
		for (const n of spur) {
			const s = size(n);
			if (direction === 'up') {
				cursor -= s.height;
				positions.set(n, { x: x + SPUR_INDENT, y: cursor });
				cursor -= SPUR_GAP;
			} else if (direction === 'down') {
				positions.set(n, { x: x + SPUR_INDENT, y: cursor });
				cursor += s.height + SPUR_GAP;
			} else {
				positions.set(n, { x: x + own.width + SPUR_GAP + SPUR_INDENT, y: cursor });
				cursor += s.height + SPUR_GAP;
			}
		}
	};
	/** Horizontal room a ring node and its spur take on a run. */
	const slotWidth = (node: string) =>
		Math.max(size(node).width, spurExtent(spurOf(node)).width) + RUN_GAP;

	let groupX = 0;
	let ringBottom = -Infinity;
	for (const ring of rings) {
		const spine = ring.spine;
		const loops = ring.loops.map((loop) => {
			const topCount = Math.ceil(loop.length / 2);
			return { top: loop.slice(0, topCount), bottom: loop.slice(topCount).reverse() };
		});
		const rowHeight = (row: string[]) => Math.max(0, ...row.map((n) => size(n).height));
		const runWidth = (row: string[]) => row.reduce((w, n) => w + slotWidth(n), 0);

		// The spine column: its ends stack their spurs above and below, its interior nodes to the
		// right, so it is as wide as the widest node with its spur.
		const spineWidth =
			spine.length === 0
				? 0
				: Math.max(
						...spine.map((n, i) => {
							const spur = spurExtent(spurOf(n)).width;
							return i > 0 && i < spine.length - 1
								? size(n).width + spur + SPUR_GAP
								: Math.max(size(n).width, spur);
						})
					) + RUN_GAP;

		// Rows: every loop's top run shares the top row, its bottom run the bottom row. The bottom
		// row sits below the top row and below the spine's own stack.
		const topY = 0;
		const topHeight = Math.max(
			spine.length ? size(spine[0]).height : 0,
			...loops.map((l) => rowHeight(l.top))
		);
		const spineStack = spine.slice(0, -1).reduce((h, n) => h + size(n).height + ROW_GAP / 2, 0);
		const bottomY = topY + Math.max(topHeight + ROW_GAP, spineStack);
		const bottomHeight = Math.max(
			spine.length ? size(spine[spine.length - 1]).height : 0,
			...loops.map((l) => rowHeight(l.bottom))
		);

		// Loops alternate right and left of the spine; loops on one side nest outward by sitting
		// beyond the previous loop's far end.
		const rightLoops = loops.filter((_, i) => i % 2 === 0);
		const leftLoops = loops.filter((_, i) => i % 2 === 1);
		const sideWidth = (side: typeof loops) =>
			side.reduce((w, l) => w + Math.max(runWidth(l.top), runWidth(l.bottom)), 0);
		const leftWidth = sideWidth(leftLoops);
		const spineX = groupX + leftWidth;

		spine.forEach((n, i) => {
			if (i === 0) placeWithSpur(n, spineX, topY, 'up');
			else if (i === spine.length - 1) placeWithSpur(n, spineX, bottomY, 'down');
			else {
				const y = topY + spine.slice(0, i).reduce((h, m) => h + size(m).height + ROW_GAP / 2, 0);
				placeWithSpur(n, spineX, y, 'right');
			}
		});

		const placeRun = (row: string[], y: number, x0: number, rightward: boolean, dir: Direction) => {
			let x = x0;
			for (const n of row) {
				const w = slotWidth(n);
				placeWithSpur(n, rightward ? x : x - w + RUN_GAP, y, dir);
				x += rightward ? w : -w;
			}
		};
		let rightX = spineX + spineWidth;
		for (const loop of rightLoops) {
			placeRun(loop.top, topY, rightX, true, 'up');
			placeRun(loop.bottom, bottomY, rightX, true, 'down');
			rightX += Math.max(runWidth(loop.top), runWidth(loop.bottom));
		}
		let leftX = spineX;
		for (const loop of leftLoops) {
			placeRun(loop.top, topY, leftX - RUN_GAP, false, 'up');
			placeRun(loop.bottom, bottomY, leftX - RUN_GAP, false, 'down');
			leftX -= Math.max(runWidth(loop.top), runWidth(loop.bottom));
		}

		groupX = rightX + GROUP_GAP;
		for (const n of ringNodes(ring)) {
			const spur = spurExtent(spurOf(n));
			ringBottom = Math.max(ringBottom, positions.get(n)!.y + size(n).height + spur.height);
		}
		ringBottom = Math.max(ringBottom, bottomY + bottomHeight);
	}

	// Components with no ring keep their own layout, moved below the rings.
	const rest = graph.nodes.filter((n) => !positions.has(n));
	if (rest.length > 0) {
		const minX = Math.min(...rest.map((n) => fallback.get(n)?.x ?? 0));
		const minY = Math.min(...rest.map((n) => fallback.get(n)?.y ?? 0));
		const ringMinX = Math.min(...[...positions.values()].map((p) => p.x));
		for (const n of rest) {
			const p = fallback.get(n) ?? { x: 0, y: 0 };
			positions.set(n, { x: p.x - minX + ringMinX, y: p.y - minY + ringBottom + GROUP_GAP });
		}
	}

	// Normalize to the padding and snap to the grid.
	const allMinX = Math.min(...[...positions.values()].map((p) => p.x));
	const allMinY = Math.min(...[...positions.values()].map((p) => p.y));
	for (const [n, p] of positions) {
		positions.set(n, {
			x: Math.round((p.x - allMinX + PADDING) / SNAP) * SNAP,
			y: Math.round((p.y - allMinY + PADDING) / SNAP) * SNAP
		});
	}
	return positions;
}
