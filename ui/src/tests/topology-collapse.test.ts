import { get } from 'svelte/store';
import { describe, it, expect } from 'vitest';
import {
	computeCollapsedForLevel,
	collapsedForEntryFromOtherScope,
	inferCurrentLevel,
	scaleCollapseCandidates,
	nextEffectiveLevel,
	stepExpand,
	collapseLevel,
	collapsedContainers,
	computeCollapsedEdges,
	stepCollapse,
	expandedInlineGroups,
	inlineGroupKey
} from '$lib/features/topology/collapse';
import { LayoutGraph } from '$lib/features/topology/layout/layout-graph';
import type { components } from '$lib/api/schema';

type TopologyNode = components['schemas']['Node'];
type ContainerTypeMetadata = import('$lib/shared/stores/metadata').ContainerTypeMetadata;

const container = (
	id: string,
	container_type: string,
	parent_container_id?: string
): TopologyNode =>
	({
		id,
		node_type: 'Container',
		container_type,
		...(parent_container_id ? { parent_container_id } : {})
	}) as unknown as TopologyNode;

const element = (id: string, container_id: string): TopologyNode =>
	({
		id,
		node_type: 'Element',
		container_id,
		host_id: 'h',
		element_type: 'Service'
	}) as unknown as TopologyNode;

// Mock the container-type metadata the collapse logic reads.
const META: Record<string, Partial<ContainerTypeMetadata>> = {
	Subnet: { is_subcontainer: false, collapsed_by_default: false, is_collapsible: true },
	NestedTag: { is_subcontainer: true, collapsed_by_default: false, is_collapsible: true },
	ApplicationUngrouped: { is_subcontainer: false, collapsed_by_default: true, is_collapsible: true }
};
const containerTypes = {
	getMetadata: (ct: string | null) => (META[ct ?? ''] ?? {}) as ContainerTypeMetadata
};

describe('computeCollapsedForLevel — collapsed_by_default root', () => {
	const nodes = [
		container('root', 'Subnet'),
		container('sub', 'NestedTag', 'root'),
		container('ung', 'ApplicationUngrouped')
	];

	it('collapses a collapsed_by_default root at every level except 5', () => {
		expect(computeCollapsedForLevel(1, nodes, containerTypes, null).has('ung')).toBe(true);
		expect(computeCollapsedForLevel(2, nodes, containerTypes, null).has('ung')).toBe(true);
		expect(computeCollapsedForLevel(3, nodes, containerTypes, null).has('ung')).toBe(true);
		expect(computeCollapsedForLevel(4, nodes, containerTypes, null).has('ung')).toBe(true);
		expect(computeCollapsedForLevel(5, nodes, containerTypes, null).has('ung')).toBe(false);
	});

	it('still expands a plain root at level 2 and collapses subcontainers', () => {
		const c2 = computeCollapsedForLevel(2, nodes, containerTypes, null);
		expect(c2.has('root')).toBe(false);
		expect(c2.has('sub')).toBe(true);
	});
});

describe('computeCollapsedForLevel — scale collapse', () => {
	// Above 300 elements the loader collapses every collapsible container. The
	// manual ladder has to agree, or the first Collapse press expands the graph
	// instead of densifying it and the level indicator stops describing what is
	// drawn. Two hosts' worth of containers, padded past the threshold.
	// Includes one of each type the ladder distinguishes — a plain root, a
	// subcontainer, and a collapsed-by-default root — so the four levels are
	// genuinely distinguishable. Without the last one, levels 3 and 5 coincide
	// for want of anything to auto-collapse, which is a property of the fixture
	// rather than of the ladder.
	const atScale = [
		container('root', 'Subnet'),
		container('sub', 'NestedTag', 'root'),
		container('ung', 'ApplicationUngrouped'),
		...Array.from({ length: 300 }, (_, i) => element(`e${i}`, 'sub'))
	];
	const belowScale = [
		container('root', 'Subnet'),
		container('sub', 'NestedTag', 'root'),
		...Array.from({ length: 10 }, (_, i) => element(`e${i}`, 'sub'))
	];

	it('opens a reload into another view collapsed, and leaves a reload of the same view alone', () => {
		// Stored ids belong to the view they were saved in. A reload into another view strips them
		// all, and the empty set that remains would otherwise read as "expanded by hand".
		const other = collapsedForEntryFromOtherScope(
			't|L2Physical',
			't|Workloads',
			atScale,
			containerTypes
		);
		expect(other).toEqual(computeCollapsedForLevel(1, atScale, containerTypes, null));
		expect(inferCurrentLevel(other!, atScale, containerTypes, null)).toBe(1);

		expect(
			collapsedForEntryFromOtherScope('t|Workloads', 't|Workloads', atScale, containerTypes)
		).toBeNull();
		expect(
			collapsedForEntryFromOtherScope('t|L2Physical', 't|Workloads', belowScale, containerTypes)
		).toBeNull();
	});

	it('infers level 1 for a scale-collapsed load, matching what is drawn', () => {
		// `applyAutoCollapse` closes every collapsible container at scale, which is
		// exactly level 1. Before, that matched no level and fell through to a
		// hardcoded 3 while the graph was drawn fully collapsed — so the indicator
		// disagreed with the diagram and the first Collapse press expanded it.
		const loaded = scaleCollapseCandidates(atScale, containerTypes);
		expect(loaded.size).toBeGreaterThan(0);
		expect(inferCurrentLevel(loaded, atScale, containerTypes, null)).toBe(1);
	});

	it('gives every container level a distinct collapsed set, so no two render alike', () => {
		// Level 4 shares level 3's containers; it differs in the manager boxes on cards.
		const sets = ([1, 2, 3, 5] as const).map(
			(l) => computeCollapsedForLevel(l, atScale, containerTypes, null).size
		);
		// Strictly decreasing: each step up genuinely expands something.
		for (let i = 1; i < sets.length; i++) {
			expect(sets[i], `level ${i + 1} collapsed the same count as level ${i}`).toBeLessThan(
				sets[i - 1]
			);
		}
	});

	it('still expands everything at level 5, so the user can always get out', () => {
		expect(computeCollapsedForLevel(5, atScale, containerTypes, null).size).toBe(0);
	});

	it('leaves sub-threshold graphs exactly as they were', () => {
		expect(computeCollapsedForLevel(3, belowScale, containerTypes, null).has('root')).toBe(false);
		expect(computeCollapsedForLevel(2, belowScale, containerTypes, null).has('root')).toBe(false);
		// Scale collapse is inert below the threshold.
		expect(scaleCollapseCandidates(belowScale, containerTypes).size).toBe(0);
	});

	it('level 1 means the same thing at any size', () => {
		// Level 1 is "everything collapsible", independent of the size threshold —
		// otherwise a small graph could never reach a fully collapsed state.
		expect(computeCollapsedForLevel(1, belowScale, containerTypes, null).has('root')).toBe(true);
	});
});

describe('getVisibleNodes — transitive ancestor collapse', () => {
	// root → subcontainer → element. Only the root is in the collapsed set
	// (the level-3 / auto-collapse case where the subcontainer is left expanded).
	const nodes = [
		container('ung', 'ApplicationUngrouped'),
		container('sub', 'NestedTag', 'ung'),
		element('svc', 'sub')
	];

	it('hides a grandchild when only its root ancestor is collapsed', () => {
		const graph = LayoutGraph.fromTopology(nodes);
		graph.containers.get('ung')!.collapsed = true; // root only; sub stays expanded
		const visible = graph.getVisibleNodes(nodes).map((n) => n.id);
		expect(visible).toContain('ung'); // collapsed root itself still renders
		expect(visible).not.toContain('sub'); // direct child hidden
		expect(visible).not.toContain('svc'); // grandchild hidden (the bug)
	});

	it('shows everything when nothing is collapsed', () => {
		const graph = LayoutGraph.fromTopology(nodes);
		const visible = graph.getVisibleNodes(nodes).map((n) => n.id);
		expect(visible).toEqual(expect.arrayContaining(['ung', 'sub', 'svc']));
	});
});

describe('nextEffectiveLevel — skipping rungs that change nothing', () => {
	// The Application view as reported: with no application tags every service lands in one
	// `ApplicationUngrouped` root, which is collapsed_by_default. Collapsing a root hides
	// everything inside it, so levels 4 down to 1 all render the same diagram and only 5 differs.
	const applicationView = [
		container('ungrouped', 'ApplicationUngrouped'),
		container('stack', 'Stack', 'ungrouped'),
		element('svc', 'stack')
	];

	it('walks past levels whose diagram matches the current one', () => {
		// Rendered fully expanded at level 5.
		collapseLevel.set(5);
		collapsedContainers.set(new Set());

		// 3 collapses the auto-collapse root, which is a real change. 4 draws the same with no
		// manager boxes in the view, so the step lands on 3.
		expect(nextEffectiveLevel('collapse', applicationView, containerTypes, null)).toBe(3);

		// Now at 3, with the root collapsed. 2 and 1 only add containers already hidden inside
		// it, so neither changes anything and there is nowhere further to go.
		collapseLevel.set(3);
		collapsedContainers.set(computeCollapsedForLevel(3, applicationView, containerTypes, null));
		expect(nextEffectiveLevel('collapse', applicationView, containerTypes, null)).toBe(null);

		// Expanding back out still works.
		expect(nextEffectiveLevel('expand', applicationView, containerTypes, null)).toBe(5);
	});

	it('still steps one rung at a time where every level differs', () => {
		// A plain root plus a subcontainer and an auto-collapse root: every container level distinct.
		const layered = [
			container('root', 'Subnet'),
			container('sub', 'NestedTag', 'root'),
			container('ungrouped', 'ApplicationUngrouped'),
			element('svc', 'sub')
		];

		collapseLevel.set(5);
		collapsedContainers.set(new Set());
		expect(nextEffectiveLevel('collapse', layered, containerTypes, null)).toBe(3);

		collapseLevel.set(3);
		collapsedContainers.set(computeCollapsedForLevel(3, layered, containerTypes, null));
		expect(nextEffectiveLevel('collapse', layered, containerTypes, null)).toBe(2);

		collapseLevel.set(2);
		collapsedContainers.set(computeCollapsedForLevel(2, layered, containerTypes, null));
		expect(nextEffectiveLevel('collapse', layered, containerTypes, null)).toBe(1);

		collapseLevel.set(1);
		collapsedContainers.set(computeCollapsedForLevel(1, layered, containerTypes, null));
		expect(nextEffectiveLevel('collapse', layered, containerTypes, null)).toBe(null);
	});
});

describe('stepExpand ordering', () => {
	// Writing `collapsedContainers` notifies subscribers synchronously, and the viewer's
	// subscriber runs the pipeline as far as `prepare` before returning. So anything the caller
	// does *after* stepExpand returns is too late to affect the run that stepExpand caused —
	// which is how the auto-collapse containers got re-collapsed by the very run that was meant
	// to expand them, leaving the level advanced and the diagram unchanged.
	it('hands over auto-collapse ids before writing the collapse stores', () => {
		const nodes = [
			container('root', 'Subnet'),
			container('auto', 'ApplicationUngrouped', 'root'),
			element('e1', 'auto')
		];

		collapseLevel.set(3);
		collapsedContainers.set(new Set(['auto']));

		let storeWhenCalled: Set<string> | null = null;
		stepExpand(nodes, containerTypes, null, () => {
			// Snapshot what the store still holds at the moment the caller is handed the ids.
			storeWhenCalled = new Set(get(collapsedContainers));
		});

		expect(storeWhenCalled, 'callback never ran').not.toBeNull();
		expect(
			storeWhenCalled,
			'the collapse stores were written before the caller could mark ids seen'
		).toEqual(new Set(['auto']));
	});
});

describe('stepping the ladder protects what the level leaves open', () => {
	// Auto-collapse is one-shot per container via `seenAutoCollapseIds`, which lives in memory
	// while `collapsedContainers` is persisted. After a reload the graph is collapsed and that
	// record is empty, so without this the next pipeline run re-collapsed everything the ladder
	// had just expanded — the ladder then took two presses per rung, one moving the number and
	// the next moving the graph.
	it('reports the collapsible containers the new level leaves expanded', () => {
		const nodes = [
			container('root', 'Subnet'),
			container('sub', 'NestedTag', 'root'),
			element('e1', 'sub')
		];

		collapseLevel.set(1);
		collapsedContainers.set(new Set(['root', 'sub']));

		let handed: string[] = [];
		stepExpand(nodes, containerTypes, null, (ids) => {
			handed = ids;
		});

		// Level 2 opens root containers and keeps subcontainers closed, so `root` must be
		// exempted from auto-collapse and `sub` must not.
		expect(handed).toContain('root');
		expect(handed).not.toContain('sub');
	});
});

describe('level 4 opens manager boxes; level 5 opens auto-collapse containers last', () => {
	// One host with a guest card whose Network Identities box is drawn inline.
	const hostCard = {
		id: 'guest',
		node_type: 'Element',
		container_id: 'host',
		host_id: 'guest',
		element_type: 'Host',
		inline_groups: [
			{ entity_id: 'mv-1', entity_type: 'Host', group_id: 'identities', role: 'Member' }
		]
	} as unknown as TopologyNode;
	// The collapsed-by-default root keeps levels 4 and 5 apart.
	const nodes = [
		container('host', 'Subnet'),
		container('sub', 'NestedTag', 'host'),
		container('ung', 'ApplicationUngrouped'),
		hostCard
	];
	const box = inlineGroupKey('guest', 'identities');
	const level = () => inferCurrentLevel(get(collapsedContainers), nodes, containerTypes, null);

	const at = (level: 2 | 3, expanded: string[] = []) => {
		collapsedContainers.set(computeCollapsedForLevel(level, nodes, containerTypes, null));
		collapseLevel.set(level);
		expandedInlineGroups.set(new Set(expanded));
	};

	it('opens the boxes at 4, the auto-collapse root at 5, and closes them in reverse', () => {
		at(3);
		expect(level()).toBe(3);

		expect(stepExpand(nodes, containerTypes, null).newLevel).toBe(4);
		expect(get(expandedInlineGroups).has(box)).toBe(true);
		expect(get(collapsedContainers).has('ung')).toBe(true);
		expect(level()).toBe(4);

		expect(stepExpand(nodes, containerTypes, null).newLevel).toBe(5);
		expect(get(collapsedContainers).has('ung')).toBe(false);
		expect(level()).toBe(5);

		expect(stepCollapse(nodes, containerTypes, null).newLevel).toBe(4);
		expect(get(collapsedContainers).has('ung')).toBe(true);
		expect(get(expandedInlineGroups).has(box)).toBe(true);

		expect(stepCollapse(nodes, containerTypes, null).newLevel).toBe(3);
		expect(get(expandedInlineGroups).has(box)).toBe(false);
	});

	it('expanding from 2 with a box opened by hand keeps it open, which is level 4', () => {
		at(2, [box]);

		// With the box already open, levels 3 and 4 draw the same diagram, so the step lands on
		// the level inference reads back.
		expect(stepExpand(nodes, containerTypes, null).newLevel).toBe(4);
		expect(get(expandedInlineGroups).has(box)).toBe(true);
		expect(level()).toBe(4);
	});

	it('leaves boxes in other views alone', () => {
		at(3, ['other-view-card|docker']);

		stepExpand(nodes, containerTypes, null);
		stepCollapse(nodes, containerTypes, null);
		expect(get(expandedInlineGroups)).toEqual(new Set(['other-view-card|docker']));
	});

	it('steps from 3 straight to 5 in a view with no boxes', () => {
		const noBoxes = [container('host', 'Subnet'), container('ung', 'ApplicationUngrouped')];
		collapsedContainers.set(computeCollapsedForLevel(3, noBoxes, containerTypes, null));
		expandedInlineGroups.set(new Set());

		expect(nextEffectiveLevel('expand', noBoxes, containerTypes, null)).toBe(5);
	});

	it('has nowhere to expand to once everything is open', () => {
		const plain = [container('host', 'Subnet'), container('sub', 'NestedTag', 'host')];
		collapsedContainers.set(new Set());
		expandedInlineGroups.set(new Set());

		expect(nextEffectiveLevel('expand', plain, containerTypes, null)).toBe(null);
	});
});

describe('computeCollapsedEdges — edges the view disables', () => {
	type TopologyEdge = components['schemas']['Edge'];
	const edge = (id: string, edge_type: string, view_config: { type: string }): TopologyEdge =>
		({
			id,
			source: 'inside',
			target: 'outside',
			edge_type,
			view_config
		}) as unknown as TopologyEdge;
	const nodes = [container('subnet', 'Subnet'), element('inside', 'subnet')];

	// The L3 view disables PhysicalLink; collapsing a subnet merged them into visible edges.
	it('leaves a disabled edge out of the merged edge, as the expanded view does', () => {
		const result = computeCollapsedEdges(
			[
				edge('physical', 'PhysicalLink', { type: 'disabled' }),
				edge('same-host', 'SameHost', { type: 'active' })
			],
			new Set(['subnet']),
			nodes,
			[]
		);
		expect(result.map((agg) => agg.originalEdges.map((e) => e.id))).toEqual([['same-host']]);
	});

	it('draws no merged edge when every crossing edge is disabled', () => {
		const result = computeCollapsedEdges(
			[edge('physical', 'PhysicalLink', { type: 'disabled' })],
			new Set(['subnet']),
			nodes,
			[]
		);
		expect(result).toEqual([]);
	});
});
