import { describe, it, expect } from 'vitest';
import {
	adoptStoredSizes,
	containerSizesFor,
	deleteStoredLayout,
	layoutStoreKey,
	readStoredLayout,
	writeStoredLayout,
	type LayoutKeyInputs,
	type StoredLayout
} from '$lib/features/topology/pipeline/layout-store';
import { createInitialState } from '$lib/features/topology/pipeline/types';
import type { TopologyEdge, TopologyNode } from '$lib/features/topology/types/base';

const node = (id: string, header = id): TopologyNode =>
	({ id, node_type: 'Container', header }) as unknown as TopologyNode;

const edge = (source: string, target: string, affects_layout: boolean): TopologyEdge =>
	({
		id: `${source}-${target}-${Math.random()}`,
		source,
		target,
		edge_type: 'PhysicalLink',
		view_config: { type: 'active', affects_layout }
	}) as unknown as TopologyEdge;

const inputs = (overrides: Partial<LayoutKeyInputs> = {}): LayoutKeyInputs => ({
	structureKey: 'L2Physical:2:1:a@,b@||:b',
	deferCollapse: false,
	hiddenEdgeTypes: [],
	edges: [edge('a', 'b', true)],
	nodes: [node('a'), node('b')],
	...overrides
});

const layout = (): StoredLayout => ({
	sizes: new Map([
		['a', { x: 250, y: 80 }],
		['b', { x: 250, y: 120 }]
	]),
	containerSizes: new Map([['a', { collapsed: { x: 250, y: 80 } }]]),
	result: {
		nodePositions: new Map([
			['a', { x: 0, y: 0 }],
			['b', { x: 300, y: 0 }]
		]),
		containerSizes: new Map([['a', { width: 250, height: 80 }]]),
		elementNodeSizes: new Map([['b', { x: 250, y: 120 }]])
	}
});

/**
 * The key decides whether a stored layout is reused, so both directions matter: an input that
 * changes the layout must change the key, and the per-request noise in what the backend sends
 * must not.
 */
describe('layoutStoreKey', () => {
	it('is the same for the same inputs', () => {
		expect(layoutStoreKey(inputs())).toBe(layoutStoreKey(inputs()));
	});

	it('ignores the order nodes and edges arrive in, and per-request edge ids', () => {
		// The builders iterate hash maps and mint edge ids per request.
		const reordered = inputs({
			nodes: [node('b'), node('a')],
			edges: [edge('a', 'b', true)]
		});
		expect(layoutStoreKey(reordered)).toBe(layoutStoreKey(inputs()));
	});

	it('changes with the collapsed set, which the structure key carries', () => {
		const collapsedOther = inputs({ structureKey: 'L2Physical:2:1:a@,b@||:a' });
		expect(layoutStoreKey(collapsedOther)).not.toBe(layoutStoreKey(inputs()));
	});

	it('changes when a node row changes, such as a renamed header', () => {
		const renamed = inputs({ nodes: [node('a', 'core-switch-renamed'), node('b')] });
		expect(layoutStoreKey(renamed)).not.toBe(layoutStoreKey(inputs()));
	});

	it('changes when a layout edge moves to other endpoints', () => {
		// The structure key only counts edges, so a moved link leaves it unchanged.
		const moved = inputs({ edges: [edge('b', 'a', true)] });
		expect(layoutStoreKey(moved)).not.toBe(layoutStoreKey(inputs()));
	});

	it('ignores edges that do not steer layout', () => {
		const withDrawnOnlyEdge = inputs({ edges: [edge('a', 'b', true), edge('b', 'a', false)] });
		expect(layoutStoreKey(withDrawnOnlyEdge)).toBe(layoutStoreKey(inputs()));
	});

	it('changes with deferred collapse and with the hidden edge types', () => {
		expect(layoutStoreKey(inputs({ deferCollapse: true }))).not.toBe(layoutStoreKey(inputs()));
		expect(layoutStoreKey(inputs({ hiddenEdgeTypes: ['PhysicalLink'] }))).not.toBe(
			layoutStoreKey(inputs())
		);
	});
});

describe('stored layouts', () => {
	it('returns what was written, and nothing once it is deleted', async () => {
		const stored = layout();
		writeStoredLayout('key-roundtrip', stored);
		expect(await readStoredLayout('key-roundtrip')).toBe(stored);

		deleteStoredLayout('key-roundtrip');
		expect(await readStoredLayout('key-roundtrip')).toBeUndefined();
	});

	it('drops the least recently used entry past the limit', async () => {
		writeStoredLayout('key-oldest', layout());
		for (let i = 0; i < 8; i++) writeStoredLayout(`key-fill-${i}`, layout());
		expect(await readStoredLayout('key-oldest')).toBeUndefined();
		expect(await readStoredLayout('key-fill-7')).toBeDefined();
	});
});

describe('adoptStoredSizes', () => {
	it('fills the size caches so the run needs no measure pass', () => {
		const state = createInitialState();
		const sizes = adoptStoredSizes(state, 'L2Physical:topology-1', layout());

		expect(sizes.get('b')).toEqual({ x: 250, y: 120 });
		expect(state.viewSizeCache.get('L2Physical:topology-1')?.get('a')).toEqual({ x: 250, y: 80 });
		expect(state.containerSizeCache.get('a')?.collapsed).toEqual({ x: 250, y: 80 });
	});

	it('keeps a container size the session already knows and the entry lacks', () => {
		const state = createInitialState();
		state.containerSizeCache.set('a', { expanded: { x: 900, y: 600 } });
		adoptStoredSizes(state, 'L2Physical:topology-1', layout());

		expect(state.containerSizeCache.get('a')).toEqual({
			expanded: { x: 900, y: 600 },
			collapsed: { x: 250, y: 80 }
		});
	});
});

describe('containerSizesFor', () => {
	it('keeps the container sizes of the nodes the layout was computed for', () => {
		const state = createInitialState();
		state.containerSizeCache.set('a', { collapsed: { x: 250, y: 80 } });
		state.containerSizeCache.set('elsewhere', { collapsed: { x: 1, y: 1 } });

		const kept = containerSizesFor(state, layout().sizes);
		expect([...kept.keys()]).toEqual(['a']);
	});
});
