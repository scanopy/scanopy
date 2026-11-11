import { describe, it, expect, vi, beforeEach } from 'vitest';

/**
 * The two savings that cut layout allocation, locked as behaviour.
 *
 * Captures showed 8 pipeline runs for roughly 4 collapse presses, each rebuilding the layout graph
 * and running full ELK at 250-340MB and 1.8-3.7s. Both regressions would be silent — the view would
 * still render correctly, just at several times the memory — so they are worth asserting rather
 * than leaving to inspection.
 */

// Hoisted, because `vi.mock` is lifted above ordinary declarations and the factory below closes
// over this.
const { computeSpy } = vi.hoisted(() => ({
	computeSpy: vi.fn(async () => ({
		nodePositions: new Map<string, { x: number; y: number }>(),
		containerSizes: new Map<string, { width: number; height: number }>(),
		elementNodeSizes: new Map<string, { x: number; y: number }>()
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
		getMetadata: () => ({ collapsed_size: { width: 200, height: 60 } })
	} as unknown as Awaited<ReturnType<typeof importOriginal>>['containerTypes']
}));

import { executeLayout } from '#lib/features/topology/pipeline/execute-layout.ts';
import { createInitialState } from '#lib/features/topology/pipeline/types.ts';
import type { PrepareResult } from '#lib/features/topology/pipeline/types.ts';
import { LayoutGraph } from '#lib/features/topology/layout/layout-graph.ts';
import type { RenderableTopology } from '#lib/features/topology/types/base.ts';

const CONTAINER = {
	id: 'c1',
	node_type: 'Container' as const,
	label: 'Switch',
	parent_container_id: null
};
const ELEMENT = {
	id: 'e1',
	node_type: 'Element' as const,
	label: 'Port',
	container_id: 'c1'
};

const NODES = [CONTAINER, ELEMENT] as unknown as PrepareResult['layoutNodes'];

const TOPOLOGY = { id: 't1', nodes: NODES, edges: [] } as unknown as RenderableTopology;

function prep(overrides: Partial<PrepareResult> = {}): PrepareResult {
	return {
		layoutNodes: NODES,
		collapsed: new Set<string>(),
		elevatedEdges: [],
		elementToContainer: new Map(),
		parentIndex: { childrenOf: new Map(), parentOf: new Map() },
		topoKey: 'topo',
		structureKey: 'base:',
		baseKey: 'base',
		isNewStructure: true,
		isNewBaseStructure: false,
		viewChanged: false,
		topologyChanged: false,
		deferCollapse: false,
		needsElkForExpand: false,
		collapseChanged: false,
		visibleNodes: NODES,
		aggregatedEdges: [],
		hiddenEdgeTypes: [],
		prevExpandedSizes: undefined,
		prevChildPositions: undefined,
		currentView: 'L2Physical',
		topologyId: 't1',
		needsElk: true,
		isViewTransition: false,
		...overrides
	} as unknown as PrepareResult;
}

describe('layout execution reuse and cancellation', () => {
	beforeEach(() => computeSpy.mockClear());

	it('skips ELK entirely when the run is already superseded', async () => {
		const state = createInitialState();
		state.layoutGraph = LayoutGraph.fromTopology(NODES);

		const result = await executeLayout(TOPOLOGY, state, prep(), new Map(), () => true);

		// The point of the change: a press that overtakes a run must cost no layout, not a layout
		// whose result is thrown away. Checking staleness after compute() still paid for it.
		expect(computeSpy).not.toHaveBeenCalled();
		expect(result).toBeNull();
	});

	it('reuses the layout graph when only collapse changed', async () => {
		const state = createInitialState();
		const original = LayoutGraph.fromTopology(NODES);
		state.layoutGraph = original;

		await executeLayout(
			TOPOLOGY,
			state,
			prep({ collapsed: new Set(['c1']), collapseChanged: true }),
			new Map(),
			() => false
		);

		// Identity, not equality: a fresh graph would lay out the same but resets every
		// expandedSize to zero on the way, which is what produced 0x0 containers.
		expect(state.layoutGraph).toBe(original);
		expect(computeSpy).toHaveBeenCalledTimes(1);
	});

	it('rebuilds the layout graph when the node set changed', async () => {
		const state = createInitialState();
		const original = LayoutGraph.fromTopology(NODES);
		state.layoutGraph = original;

		await executeLayout(
			TOPOLOGY,
			state,
			prep({ isNewBaseStructure: true }),
			new Map(),
			() => false
		);

		expect(state.layoutGraph).not.toBe(original);
	});
});
