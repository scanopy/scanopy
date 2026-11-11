import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
	activeTagFilter,
	activeView,
	hiddenEdgeTypes,
	hydrateStoresFromTopology,
	resetTopologyOptions,
	topologyOptions,
	updateActiveTagFilter,
	updateHiddenEdgeTypes,
	updateTopologyOptions
} from '#lib/features/topology/queries.ts';
import type { Topology, TopologyOptions } from '#lib/features/topology/types/base.ts';

/**
 * Local options (minimap, edge bundling, fading) are saved as one value for every view; hidden
 * edge types and the tag filter are saved per view. Switching views or reloading on a non-default view
 * must keep what the user saved.
 */

const TAG_ID = '6f1c2a9e-3b7d-4c55-9a0e-1d2f3e4a5b6c';

function savedOptions(): TopologyOptions {
	return {
		request: {
			hide_entities: {},
			hide_metadata_values: {},
			container_rules: {},
			element_rules: [],
			element_sort: {}
		},
		local: {
			show_minimap: false,
			bundle_edges: false,
			no_fade_edges: true,
			tag_filter: { L3Logical: { hidden_tag_ids: { Host: [TAG_ID] } } },
			hide_edge_types: {
				L3Logical: ['Hypervisor'],
				L2Physical: ['SameHost'],
				Workloads: [],
				Application: ['ContainerRuntime']
			}
		}
	};
}

// Hydration reads only `options`; the rest of the row plays no part here.
function topologyWith(options: TopologyOptions): Topology {
	return { options } as Topology;
}

function expectSavedSharedLocal() {
	const local = get(topologyOptions).local;
	expect(local.show_minimap).toBe(false);
	expect(local.bundle_edges).toBe(false);
	expect(local.no_fade_edges).toBe(true);
	expect(local.tag_filter.L3Logical?.hidden_tag_ids).toEqual({ Host: [TAG_ID] });
}

describe('topology local options across views', () => {
	beforeEach(() => {
		resetTopologyOptions();
		activeView.set('L3Logical');
	});

	it('keeps saved local options when the active view changes', () => {
		hydrateStoresFromTopology(topologyWith(savedOptions()));
		activeView.set('L2Physical');
		expectSavedSharedLocal();
		activeView.set('Application');
		expectSavedSharedLocal();
	});

	it('keeps saved local options when the URL view is applied after hydration', () => {
		// TopologyTab hydrates first, then sets the view from `?view=` on reload.
		hydrateStoresFromTopology(topologyWith(savedOptions()));
		activeView.set('Workloads');
		expectSavedSharedLocal();
		expect(get(hiddenEdgeTypes)).toEqual([]);
	});

	it('keeps a change made in one view after switching to another', () => {
		hydrateStoresFromTopology(topologyWith(savedOptions()));
		activeView.set('L2Physical');
		updateTopologyOptions((opts) => ({ ...opts, local: { ...opts.local, show_minimap: true } }));
		activeView.set('L3Logical');
		expect(get(topologyOptions).local.show_minimap).toBe(true);
		expect(get(topologyOptions).local.bundle_edges).toBe(false);
	});

	it('keeps hidden edge types per view', () => {
		hydrateStoresFromTopology(topologyWith(savedOptions()));
		expect(get(hiddenEdgeTypes)).toEqual(['Hypervisor']);

		updateHiddenEdgeTypes((hidden) => [...hidden, 'RequestPath']);

		activeView.set('L2Physical');
		expect(get(hiddenEdgeTypes)).toEqual(['SameHost']);

		activeView.set('L3Logical');
		expect(get(hiddenEdgeTypes)).toEqual(['Hypervisor', 'RequestPath']);
		expect(get(topologyOptions).local.hide_edge_types.Application).toEqual(['ContainerRuntime']);
	});

	it('keeps the tag filter per view', () => {
		hydrateStoresFromTopology(topologyWith(savedOptions()));
		expect(get(activeTagFilter).hidden_tag_ids).toEqual({ Host: [TAG_ID] });

		activeView.set('Workloads');
		expect(get(activeTagFilter).hidden_tag_ids).toEqual({});
		updateActiveTagFilter(() => ({ hidden_tag_ids: { Service: [TAG_ID] } }));

		activeView.set('L3Logical');
		expect(get(activeTagFilter).hidden_tag_ids).toEqual({ Host: [TAG_ID] });
		expect(get(topologyOptions).local.tag_filter.Workloads?.hidden_tag_ids).toEqual({
			Service: [TAG_ID]
		});
	});

	it('share viewers get defaults, not the saved options', () => {
		hydrateStoresFromTopology(topologyWith(savedOptions()), true, true);
		const local = get(topologyOptions).local;
		expect(local.show_minimap).toBe(true);
		expect(local.tag_filter).toEqual({});
	});
});
