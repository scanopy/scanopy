import { describe, it, expect, vi, beforeEach } from 'vitest';

/**
 * Opening one container on a large site: the inside is laid out alone and room is made around it,
 * in place of a full layout run.
 */

const { computeSpy } = vi.hoisted(() => ({
	computeSpy: vi.fn(async (input: { nodes: { id: string }[] }) => ({
		// The engine places its one root container wherever it likes and sizes it from its inside.
		nodePositions: new Map(input.nodes.map((node, index) => [node.id, { x: 20, y: 40 + index }])),
		containerSizes: new Map([['open', { width: 620, height: 410 }]]),
		elementNodeSizes: new Map([['card', { x: 250, y: 90 }]])
	}))
}));

vi.mock('#lib/features/topology/layout/engine.ts', () => ({
	ElkLayoutEngine: class {
		compute = computeSpy;
	}
}));

vi.mock(import('#lib/shared/stores/metadata.ts'), async (importOriginal) => ({
	...(await importOriginal()),
	containerTypes: {
		getMetadata: () => ({
			collapsed_size: { width: 200, height: 60 },
			padding: { top: 40, left: 10, bottom: 10, right: 10 },
			is_subcontainer: false
		})
	} as unknown as Awaited<ReturnType<typeof importOriginal>>['containerTypes']
}));

import {
	executeLayout,
	localExpandTarget
} from '#lib/features/topology/pipeline/execute-layout.ts';
import { createInitialState } from '#lib/features/topology/pipeline/types.ts';
import type { PrepareResult } from '#lib/features/topology/pipeline/types.ts';
import { LayoutGraph } from '#lib/features/topology/layout/layout-graph.ts';
import type { RenderableTopology, TopologyNode } from '#lib/features/topology/types/base.ts';

type Box = { id: string; x: number; y: number; w: number; h: number };

const containerNode = (id: string, parent: string | null = null): TopologyNode =>
	({ id, node_type: 'Container', parent_container_id: parent }) as unknown as TopologyNode;
const elementNode = (id: string, container: string): TopologyNode =>
	({ id, node_type: 'Element', container_id: container }) as unknown as TopologyNode;

/** A graph of collapsed root containers at the given boxes. */
function graphOf(boxes: Box[]): LayoutGraph {
	const graph = LayoutGraph.fromTopology(boxes.map((box) => containerNode(box.id)));
	for (const box of boxes) {
		const container = graph.containers.get(box.id)!;
		container.collapsed = true;
		container.position = { x: box.x, y: box.y };
		container.measuredCollapsedSize = { width: box.w, height: box.h };
	}
	return graph;
}

function boxesOf(graph: LayoutGraph, ids: string[]): Box[] {
	return ids.map((id) => {
		const container = graph.containers.get(id)!;
		return {
			id,
			x: container.position.x,
			y: container.position.y,
			w: container.size.width,
			h: container.size.height
		};
	});
}

const overlap = (a: Box, b: Box) =>
	a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;

/** Deterministic pseudo-random numbers, so a failure reproduces. */
function seeded(seed: number): () => number {
	let state = seed;
	return () => {
		state = (state * 1664525 + 1013904223) % 4294967296;
		return state / 4294967296;
	};
}

describe('LayoutGraph.makeRoomFor', () => {
	it('moves what is right of and below the old box, and nothing else', () => {
		const graph = graphOf([
			{ id: 'above', x: 300, y: 0, w: 200, h: 60 },
			{ id: 'left', x: 0, y: 100, w: 200, h: 60 },
			{ id: 'open', x: 300, y: 100, w: 200, h: 60 },
			{ id: 'right', x: 600, y: 100, w: 200, h: 60 },
			{ id: 'below', x: 300, y: 200, w: 200, h: 60 },
			{ id: 'diagonal', x: 600, y: 200, w: 200, h: 60 }
		]);
		const open = graph.containers.get('open')!;
		open.collapsed = false;
		open.expandedSize = { width: 500, height: 360 };

		graph.makeRoomFor('open', { width: 200, height: 60 });

		const at = (id: string) => graph.containers.get(id)!.position;
		expect(at('open')).toEqual({ x: 300, y: 100 });
		expect(at('above')).toEqual({ x: 300, y: 0 });
		expect(at('left')).toEqual({ x: 0, y: 100 });
		expect(at('right')).toEqual({ x: 900, y: 100 });
		expect(at('below')).toEqual({ x: 300, y: 500 });
		expect(at('diagonal')).toEqual({ x: 900, y: 500 });
	});

	it('creates no overlap on random packings', () => {
		const random = seeded(7);
		for (let round = 0; round < 200; round++) {
			// Boxes of random size in the cells of a grid with random gaps: no two overlap.
			const boxes: Box[] = [];
			const columns = 2 + Math.floor(random() * 6);
			const rows = 2 + Math.floor(random() * 6);
			for (let row = 0; row < rows; row++) {
				for (let column = 0; column < columns; column++) {
					if (random() < 0.15) continue;
					boxes.push({
						id: `${row}-${column}`,
						x: column * 400 + Math.floor(random() * 100),
						y: row * 300 + Math.floor(random() * 100),
						w: 100 + Math.floor(random() * 200),
						h: 50 + Math.floor(random() * 150)
					});
				}
			}
			if (boxes.length < 2) continue;
			const graph = graphOf(boxes);
			const opened = boxes[Math.floor(random() * boxes.length)];
			const container = graph.containers.get(opened.id)!;
			container.collapsed = false;
			container.expandedSize = {
				width: opened.w + Math.floor(random() * 3000),
				height: opened.h + Math.floor(random() * 3000)
			};

			graph.makeRoomFor(opened.id, { width: opened.w, height: opened.h });

			const after = boxesOf(
				graph,
				boxes.map((box) => box.id)
			);
			for (let i = 0; i < after.length; i++) {
				for (let j = i + 1; j < after.length; j++) {
					expect(
						overlap(after[i], after[j]),
						`round ${round}: ${after[i].id} / ${after[j].id}`
					).toBe(false);
				}
			}
		}
	});

	it('grows the parent of an opened subcontainer and makes room for the parent too', () => {
		const graph = LayoutGraph.fromTopology([
			containerNode('parent'),
			containerNode('sub', 'parent'),
			elementNode('beside', 'parent'),
			elementNode('under', 'parent'),
			containerNode('neighbour')
		]);
		const parent = graph.containers.get('parent')!;
		parent.position = { x: 0, y: 0 };
		parent.expandedSize = { width: 480, height: 220 };
		const sub = graph.containers.get('sub')!;
		sub.position = { x: 10, y: 40 };
		sub.expandedSize = { width: 400, height: 300 };
		graph.elements.get('beside')!.position = { x: 220, y: 40 };
		graph.elements.get('beside')!.size = { x: 250, y: 60 };
		graph.elements.get('under')!.position = { x: 10, y: 110 };
		graph.elements.get('under')!.size = { x: 250, y: 100 };
		const neighbour = graph.containers.get('neighbour')!;
		neighbour.collapsed = true;
		neighbour.position = { x: 500, y: 0 };

		graph.makeRoomFor('sub', { width: 200, height: 60 });

		expect(sub.position).toEqual({ x: 10, y: 40 });
		expect(graph.elements.get('beside')!.position).toEqual({ x: 420, y: 40 });
		expect(graph.elements.get('under')!.position).toEqual({ x: 10, y: 350 });
		// Widest child ends at 670, lowest at 450, plus the parent's padding.
		expect(parent.expandedSize).toEqual({ width: 680, height: 460 });
		// The parent grew by 200, so the root to its right moves by that.
		expect(neighbour.position).toEqual({ x: 700, y: 0 });
	});
});

describe('localExpandTarget', () => {
	const graph = () => {
		const built = LayoutGraph.fromTopology([
			containerNode('a'),
			containerNode('b'),
			containerNode('sub', 'a')
		]);
		for (const container of built.containers.values()) container.collapsed = true;
		return built;
	};
	const all = () => new Set(['a', 'b', 'sub']);

	it('names the one container that opened', () => {
		expect(localExpandTarget(graph(), all(), new Set(['a', 'sub']))).toBe('b');
	});

	it('names the outer container when its subcontainers open with it', () => {
		expect(localExpandTarget(graph(), all(), new Set(['b']))).toBe('a');
	});

	it('is undefined before any layout, and when nothing or two separate containers changed', () => {
		expect(localExpandTarget(graph(), null, new Set(['a', 'sub']))).toBeUndefined();
		expect(localExpandTarget(graph(), all(), all())).toBeUndefined();
		expect(localExpandTarget(graph(), all(), new Set(['sub']))).toBeUndefined();
	});

	it('is undefined for a collapse', () => {
		const built = graph();
		built.containers.get('b')!.collapsed = false;
		expect(localExpandTarget(built, new Set(['a', 'sub']), all())).toBeUndefined();
	});

	it('is undefined for a container hidden inside a collapsed parent', () => {
		expect(localExpandTarget(graph(), all(), new Set(['a', 'b']))).toBeUndefined();
	});

	it('is undefined when a superseded run left the graph in another collapse state', () => {
		const built = graph();
		built.containers.get('a')!.collapsed = false;
		expect(localExpandTarget(built, all(), new Set(['a', 'sub']))).toBeUndefined();
	});
});

describe('executeLayout with a local expand', () => {
	const NODES = [
		containerNode('open'),
		elementNode('card', 'open'),
		containerNode('right'),
		elementNode('hidden', 'right')
	];
	const TOPOLOGY = { id: 't1', nodes: NODES, edges: [] } as unknown as RenderableTopology;
	const VISIBLE = NODES.filter((node) => node.id !== 'hidden');

	const prep = (overrides: Partial<PrepareResult> = {}): PrepareResult =>
		({
			layoutNodes: NODES,
			collapsed: new Set(['right']),
			elevatedEdges: [
				{ id: 'cross', source: 'card', target: 'hidden' },
				{ id: 'inner', source: 'card', target: 'open' }
			],
			elementToContainer: new Map(),
			parentIndex: { childrenOf: new Map(), parentOf: new Map() },
			topoKey: 'topo',
			structureKey: 'base:right',
			baseKey: 'base',
			isNewStructure: true,
			isNewBaseStructure: false,
			viewChanged: false,
			topologyChanged: false,
			deferCollapse: false,
			needsElkForExpand: true,
			collapseChanged: true,
			visibleNodes: VISIBLE,
			aggregatedEdges: [],
			hiddenEdgeTypes: [],
			prevExpandedSizes: undefined,
			prevChildPositions: undefined,
			currentView: 'L2Physical',
			topologyId: 't1',
			needsElk: true,
			isViewTransition: false,
			localExpand: { containerId: 'open', previousSize: { width: 200, height: 60 } },
			...overrides
		}) as unknown as PrepareResult;

	const laidOutState = () => {
		const state = createInitialState();
		const graph = LayoutGraph.fromTopology(NODES);
		const open = graph.containers.get('open')!;
		open.position = { x: 100, y: 50 };
		open.measuredCollapsedSize = { width: 200, height: 60 };
		const right = graph.containers.get('right')!;
		right.collapsed = true;
		right.position = { x: 350, y: 50 };
		right.measuredCollapsedSize = { width: 200, height: 60 };
		state.layoutGraph = graph;
		state.sessionCollapsed = new Set(['open', 'right']);
		return state;
	};

	// Braces: vitest calls whatever a hook returns as its cleanup, and `mockClear` returns the spy.
	beforeEach(() => {
		computeSpy.mockClear();
	});

	it('runs the engine on the opened container alone and keeps it in place', async () => {
		const state = laidOutState();

		const result = await executeLayout(TOPOLOGY, state, prep(), new Map(), () => false);

		expect(computeSpy).toHaveBeenCalledTimes(1);
		const input = computeSpy.mock.calls[0][0] as unknown as {
			nodes: { id: string }[];
			edges: { id: string }[];
		};
		expect(input.nodes.map((node) => node.id)).toEqual(['open', 'card']);
		expect(input.edges.map((edge) => edge.id)).toEqual(['inner']);

		const graph = state.layoutGraph!;
		expect(graph.containers.get('open')!.position).toEqual({ x: 100, y: 50 });
		expect(graph.containers.get('open')!.expandedSize).toEqual({ width: 620, height: 410 });
		// 420 wider, moved on the 25px grid.
		expect(graph.containers.get('right')!.position).toEqual({ x: 775, y: 50 });
		expect(state.sessionCollapsed).toEqual(new Set(['right']));
		// Nothing for the layout store: a reload of this state gets the full layout.
		expect(result?.engineResult).toBeUndefined();
	});

	it('uses a stored layout for the same state instead', async () => {
		const state = laidOutState();
		const stored = {
			nodePositions: new Map([
				['open', { x: 0, y: 0 }],
				['right', { x: 900, y: 0 }]
			]),
			containerSizes: new Map([['open', { width: 620, height: 410 }]]),
			elementNodeSizes: new Map()
		};

		await executeLayout(TOPOLOGY, state, prep(), new Map(), () => false, stored);

		expect(computeSpy).not.toHaveBeenCalled();
		expect(state.layoutGraph!.containers.get('right')!.position).toEqual({ x: 900, y: 0 });
	});

	it('runs the full layout when the run has no local expand', async () => {
		const state = laidOutState();

		await executeLayout(TOPOLOGY, state, prep({ localExpand: undefined }), new Map(), () => false);

		const input = computeSpy.mock.calls[0][0] as unknown as { nodes: { id: string }[] };
		expect(input.nodes.map((node) => node.id)).toEqual(['open', 'card', 'right']);
	});
});
