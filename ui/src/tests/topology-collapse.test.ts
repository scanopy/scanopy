import { get } from 'svelte/store';
import { describe, it, expect } from 'vitest';
import {
	computeCollapsedForLevel,
	inferCurrentLevel,
	scaleCollapseCandidates,
	nextEffectiveLevel,
	stepExpand,
	collapseLevel,
	collapsedContainers
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

	it('collapses a collapsed_by_default root at every level except 4', () => {
		expect(computeCollapsedForLevel(1, nodes, containerTypes, null).has('ung')).toBe(true);
		expect(computeCollapsedForLevel(2, nodes, containerTypes, null).has('ung')).toBe(true);
		expect(computeCollapsedForLevel(3, nodes, containerTypes, null).has('ung')).toBe(true);
		expect(computeCollapsedForLevel(4, nodes, containerTypes, null).has('ung')).toBe(false);
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
	// genuinely distinguishable. Without the last one, levels 3 and 4 coincide
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

	it('infers level 1 for a scale-collapsed load, matching what is drawn', () => {
		// `applyAutoCollapse` closes every collapsible container at scale, which is
		// exactly level 1. Before, that matched no level and fell through to a
		// hardcoded 3 while the graph was drawn fully collapsed — so the indicator
		// disagreed with the diagram and the first Collapse press expanded it.
		const loaded = scaleCollapseCandidates(atScale, containerTypes);
		expect(loaded.size).toBeGreaterThan(0);
		expect(inferCurrentLevel(loaded, atScale, containerTypes, null)).toBe(1);
	});

	it('gives every level a distinct collapsed set, so no two render alike', () => {
		const sets = ([1, 2, 3, 4] as const).map(
			(l) => computeCollapsedForLevel(l, atScale, containerTypes, null).size
		);
		// Strictly decreasing: each step up genuinely expands something.
		for (let i = 1; i < sets.length; i++) {
			expect(sets[i], `level ${i + 1} collapsed the same count as level ${i}`).toBeLessThan(
				sets[i - 1]
			);
		}
	});

	it('still expands everything at level 4, so the user can always get out', () => {
		expect(computeCollapsedForLevel(4, atScale, containerTypes, null).size).toBe(0);
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
	// everything inside it, so levels 3, 2 and 1 all render the same diagram and only 4 differs.
	const applicationView = [
		container('ungrouped', 'ApplicationUngrouped'),
		container('stack', 'Stack', 'ungrouped'),
		element('svc', 'stack')
	];

	it('walks past levels whose diagram matches the current one', () => {
		// Rendered fully expanded at level 4.
		collapseLevel.set(4);
		collapsedContainers.set(new Set());

		// 3 collapses the auto-collapse root, which is a real change.
		expect(nextEffectiveLevel('collapse', applicationView, containerTypes, null)).toBe(3);

		// Now at 3, with the root collapsed. 2 and 1 only add containers already hidden inside
		// it, so neither changes anything and there is nowhere further to go.
		collapseLevel.set(3);
		collapsedContainers.set(computeCollapsedForLevel(3, applicationView, containerTypes, null));
		expect(nextEffectiveLevel('collapse', applicationView, containerTypes, null)).toBe(null);

		// Expanding back out still works.
		expect(nextEffectiveLevel('expand', applicationView, containerTypes, null)).toBe(4);
	});

	it('still steps one rung at a time where every level differs', () => {
		// A plain root plus a subcontainer and an auto-collapse root: all four levels distinct.
		const layered = [
			container('root', 'Subnet'),
			container('sub', 'NestedTag', 'root'),
			container('ungrouped', 'ApplicationUngrouped'),
			element('svc', 'sub')
		];

		collapseLevel.set(4);
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
