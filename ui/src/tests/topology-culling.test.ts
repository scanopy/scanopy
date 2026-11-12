/**
 * Culling has to actually reduce what gets mounted.
 *
 * A customer's L2 view held 17,236 nodes and mounted every one of them while the culling gate
 * reported itself on, which exhausted browser memory. The gate was fine; the nodes were not. Two
 * properties of a freshly built node set each defeat `getNodesInside` on their own:
 *
 *  - a node with no known height has `area === 0`, and the test is `overlappingArea >= area`, so
 *    `0 >= 0` passes for every node wherever the viewport is;
 *  - a node whose object carries no `measured` loses its handle bounds in `adoptUserNodes`, which
 *    sets `forceInitialRender` and short-circuits the viewport test entirely.
 *
 * Neither is visible from `shouldCull`, which is why the existing gate test passed throughout. So
 * these drive the *real* `adoptUserNodes` and `getNodesInside` from `@xyflow/system` over nodes
 * from the *real* `buildFlowNodes`, and assert on how many survive. That is the symptom the bug
 * produced, and the only thing that would have caught it.
 */
import { describe, it, expect } from 'vitest';
import {
	adoptUserNodes,
	getNodesInside,
	type InternalNodeBase,
	type NodeLookup,
	type ParentLookup
} from '@xyflow/system';
import type { Node } from '@xyflow/svelte';
import {
	buildFlowNodes,
	sortFlowNodes,
	stripSizeSeed
} from '#lib/features/topology/pipeline/build-flow-nodes.ts';
import type { LayoutGraph } from '#lib/features/topology/layout/layout-graph.ts';
import type { RenderableTopology, TopologyNode } from '#lib/features/topology/types/base.ts';
import type { XY } from '#lib/features/topology/pipeline/types.ts';

const ELEMENT_WIDTH = 250;
const ELEMENT_HEIGHT = 120;
const COLUMN_SPACING = 400;
const ROW_SPACING = 200;

/** Elements per column, so the fixture is tall enough for most of it to be off screen. */
const ROWS = 40;
const COLUMNS = 20;
const ELEMENT_COUNT = ROWS * COLUMNS;

const container = (id: string): TopologyNode =>
	({ id, node_type: 'Container', container_type: 'Host' }) as unknown as TopologyNode;

const element = (id: string, container_id: string): TopologyNode =>
	({
		id,
		node_type: 'Element',
		container_id,
		host_id: 'h',
		element_type: 'Interface',
		entity_id: id
	}) as unknown as TopologyNode;

/**
 * A wide, tall grid of element cards inside one container — the shape an expanded L2 host produces,
 * scaled down enough to keep the test quick.
 */
function buildFixture(): {
	visibleNodes: TopologyNode[];
	positions: Map<string, { x: number; y: number }>;
	sizeHints: Map<string, XY>;
} {
	const visibleNodes: TopologyNode[] = [container('c1')];
	const positions = new Map<string, { x: number; y: number }>([['c1', { x: 0, y: 0 }]]);
	const sizeHints = new Map<string, XY>([
		['c1', { x: COLUMNS * COLUMN_SPACING, y: ROWS * ROW_SPACING }]
	]);

	for (let i = 0; i < ELEMENT_COUNT; i++) {
		const id = `e${i}`;
		visibleNodes.push(element(id, 'c1'));
		positions.set(id, {
			x: (i % COLUMNS) * COLUMN_SPACING,
			y: Math.floor(i / COLUMNS) * ROW_SPACING
		});
		sizeHints.set(id, { x: ELEMENT_WIDTH, y: ELEMENT_HEIGHT });
	}

	return { visibleNodes, positions, sizeHints };
}

const { visibleNodes, positions, sizeHints } = buildFixture();

/** Only the four accessors `buildFlowNodes` reaches for. */
const layoutGraph = {
	getPosition: (id: string) => positions.get(id),
	getContainerSize: (id: string) => {
		const hint = sizeHints.get(id);
		return hint ? { width: hint.x, height: hint.y } : undefined;
	},
	getChildCount: () => ELEMENT_COUNT,
	getSubgroupSummaries: () => []
} as unknown as LayoutGraph;

const topology = { nodes: visibleNodes, edges: [] } as unknown as RenderableTopology;

function build(
	overrides: {
		sizeHints?: Map<string, XY> | null;
		isNewStructure?: boolean;
		useGraph?: boolean;
	} = {}
) {
	return sortFlowNodes(
		buildFlowNodes({
			visibleNodes,
			collapsed: new Set<string>(),
			topology,
			useGraph: overrides.useGraph ?? true,
			layoutGraph,
			isNewStructure: overrides.isNewStructure ?? false,
			liveNodes: [],
			infraRuleId: null,
			editMode: false,
			sizeHints: 'sizeHints' in overrides ? overrides.sizeHints : sizeHints
		})
	);
}

type Lookup = NodeLookup<InternalNodeBase<Node>>;

/**
 * Mount `flowNodes` into a lookup and return how many SvelteFlow would render.
 *
 * `lookup` is passed in so a test can adopt twice into the same one — which is what a pipeline
 * rebuild does, and where the handle-bounds interaction lives.
 */
function visibleCount(
	flowNodes: Node[],
	viewport: { width: number; height: number },
	lookup: Lookup = new Map()
): number {
	adoptUserNodes(flowNodes, lookup, new Map() as ParentLookup<InternalNodeBase<Node>>, {});
	return getNodesInside(
		lookup,
		{ x: 0, y: 0, width: viewport.width, height: viewport.height },
		[0, 0, 1],
		true
	).length;
}

// A viewport over the top-left corner of the grid: 2 columns by 4 rows of a 20 × 40 grid, so
// well under a tenth of the graph is on screen and the rest must be culled.
const VIEWPORT = { width: 2 * COLUMN_SPACING, height: 4 * ROW_SPACING };

describe('viewport culling over built flow nodes', () => {
	it('mounts a small fraction of a graph that is mostly off screen', () => {
		const mounted = visibleCount(build(), VIEWPORT);

		// The container spans the whole grid so it is always on screen; everything else has to
		// earn its place. Generous bound: the point is that `mounted` stops tracking node count,
		// not that it hits an exact figure a layout tweak would invalidate.
		expect(mounted).toBeLessThan(ELEMENT_COUNT / 4);
	});

	it('keeps culling across a rebuild', () => {
		// The pipeline rebuilds node objects on every run. Object identity changes, so
		// `adoptUserNodes` re-derives internals — and `parseHandles` only carries handle bounds
		// forward for a node that has already mounted. Rebuilding must not silently restore
		// `forceInitialRender` for the whole graph.
		const lookup: Lookup = new Map();
		const first = visibleCount(build(), VIEWPORT, lookup);
		const second = visibleCount(build(), VIEWPORT, lookup);

		expect(second).toBe(first);
	});

	it('culls nodes that have never mounted', () => {
		// The out-of-memory path: expanding a collapsed container introduces thousands of nodes
		// with no previous mount to inherit handle bounds from. An empty lookup is that case.
		// This passes only because the built nodes carry synthesized `handles`.
		const mounted = visibleCount(build(), VIEWPORT, new Map());

		expect(mounted).toBeLessThan(ELEMENT_COUNT / 4);
	});

	it('mounts everything when no size is known', () => {
		// Without hints a node has neither a height nor handle bounds, so it must render once to
		// be measured. Asserted so the degradation stays deliberate: it is the reason a first
		// paint is still a full mount, and the reason the measure pass has to suspend culling.
		const mounted = visibleCount(build({ sizeHints: null }), VIEWPORT);

		expect(mounted).toBe(ELEMENT_COUNT + 1);
	});
});

describe('the measurement pass is never culled', () => {
	it('mounts every node when building for measurement', () => {
		// `resolveNodeSizes` mounts the graph and reads heights out of the DOM. A culled node
		// never mounts, so it would measure as absent and ELK would lay out against a fallback.
		// Both the measurement placement branch and `stripSizeSeed` exist to guarantee this; if a
		// future change seeds sizes into either, this fails rather than quietly degrading layout.
		//
		// The measurement branch is reached by `!useGraph && isNewStructure`. This previously passed
		// `isNewStructure` alone against a layout graph, which never reaches that branch — it was
		// passing on the old gate, which dropped the size hint for *any* new structure rather than
		// for a measurement, and so asserted a proxy rather than the behaviour it names.
		const measurementPass = visibleCount(
			build({ useGraph: false, isNewStructure: true }),
			VIEWPORT
		);
		expect(measurementPass).toBe(ELEMENT_COUNT + 1);

		const stripped = visibleCount(stripSizeSeed(build()), VIEWPORT);
		expect(stripped).toBe(ELEMENT_COUNT + 1);
	});

	it('keeps culling a collapse change, which is not a measurement', () => {
		// The counterpart to the above, and the regression that mattered: a collapse press sets
		// `isNewStructure` while placing from the layout graph. Dropping the size hint there left
		// every element without `measured`, which means `forceInitialRender` — so each press
		// rebuilt the whole graph unculled and mounted all of it.
		const mounted = visibleCount(build({ isNewStructure: true }), VIEWPORT);

		expect(mounted).toBeLessThan(ELEMENT_COUNT / 4);
	});
});
