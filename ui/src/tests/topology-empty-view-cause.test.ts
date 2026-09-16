import { describe, it, expect } from 'vitest';
import { activeViewFilters } from '$lib/features/topology/interactions';
import { clearedHideSetFor } from '$lib/features/topology/queries';
import type { RenderableTopology } from '$lib/features/topology/types/base';

/**
 * An L2 view whose ports are all linked — the state the live database is in, where hiding
 * `Unlinked` removes 103 of 171 ports and the remaining 68 render.
 *
 * `filtered_out` stands for what the server dropped before the response was built: those entities
 * are absent from `interfaces`, so it is the only evidence they exist.
 */
function l2Topology(filteredOut: Record<string, Record<string, number>>): RenderableTopology {
	return {
		id: 'topo-1',
		network_id: 'net-1',
		nodes: [],
		edges: [],
		hosts: [],
		services: [],
		subnets: [],
		ip_addresses: [],
		ports: [],
		bindings: [],
		interfaces: [{ id: 'if-linked', host_id: 'h1', network_id: 'net-1' }],
		neighbours: [
			{ id: 'row-1', interface_id: 'if-linked', neighbor: { type: 'Interface', id: 'if-far' } }
		],
		candidates: [],
		filtered_out: filteredOut,
		dependencies: [],
		vlans: [],
		entity_tags: [],
		name: 'My Network'
	} as unknown as RenderableTopology;
}

const NO_TAGS = {
	hidden_host_tag_ids: [],
	hidden_service_tag_ids: [],
	hidden_subnet_tag_ids: []
};

describe('what emptied a topology view', () => {
	it('names the filter and counts the entities the server never sent', () => {
		const causes = activeViewFilters(
			'L2Physical',
			l2Topology({ Interface: { LinkState: 103 } }),
			{ Interface: { LinkState: ['Unlinked'] } },
			[],
			NO_TAGS
		);

		expect(causes).toHaveLength(1);
		// The panel's own label for the control, so the message points at something the user can
		// find rather than at an internal filter id.
		expect(causes[0].label).toBe('By link');
		expect(causes[0].values).toEqual(['Unlinked']);
		expect(causes[0].count).toBe(103);
	});

	it('adds the entities the browser is hiding to the ones the server dropped', () => {
		// `if-linked` has a resolved neighbour of its own, so it classifies Linked and this
		// hide-set catches it client-side on top of the 103 already gone.
		const causes = activeViewFilters(
			'L2Physical',
			l2Topology({ Interface: { LinkState: 103 } }),
			{ Interface: { LinkState: ['Unlinked', 'Linked'] } },
			[],
			NO_TAGS
		);

		expect(causes[0].count).toBe(104);
	});

	it('says nothing when no filter is hiding anything', () => {
		expect(
			activeViewFilters('L2Physical', l2Topology({}), { Interface: { LinkState: [] } }, [], NO_TAGS)
		).toEqual([]);
	});

	/**
	 * A view with no data and no filters must fall through to its own setup prompt. Reporting a
	 * cause here is what let the L2 prompt claim discovery had found nothing while a filter was
	 * responsible — and reporting one when nothing is set would invert the same mistake.
	 */
	it('says nothing when the hide-set is absent entirely', () => {
		expect(activeViewFilters('L2Physical', l2Topology({}), undefined, [], NO_TAGS)).toEqual([]);
	});

	/** A stored entry for a filter the view no longer declares matches nothing, so it hides nothing. */
	it('ignores a hide entry the view does not declare', () => {
		const causes = activeViewFilters(
			'L2Physical',
			l2Topology({}),
			{ Interface: { Staleness: ['Stale'] } },
			[],
			NO_TAGS
		);

		expect(causes).toEqual([]);
	});

	it('reports an entity type hidden by the eye toggle', () => {
		const causes = activeViewFilters('L2Physical', l2Topology({}), {}, ['Interface'], NO_TAGS);

		expect(causes).toHaveLength(1);
		expect(causes[0].count).toBe(1);
	});
});

describe('clearing a view’s filters', () => {
	/**
	 * The reported bug: Clear all restored `Interface/LinkState = [Unlinked]` from the view's
	 * defaults, so the one button offered for undoing a filter re-applied it every time.
	 */
	it('empties a product default rather than restoring it', () => {
		const cleared = clearedHideSetFor('L2Physical', 'Interface', { LinkState: ['Unlinked'] });

		expect(cleared).toEqual({ LinkState: [] });
	});

	/**
	 * An explicit empty list is what the server reads as "show everything"; an absent key is "no
	 * opinion" and gets refilled from the defaults on the next read
	 * (`TopologyRequestOptions::merge_missing_hide_defaults`). So a filter with nothing stored
	 * still has to come back as `[]`, or clearing it survives only until the page reloads.
	 */
	it('writes an empty list for a declared filter that had no stored entry', () => {
		expect(clearedHideSetFor('L2Physical', 'Interface', undefined)).toEqual({ LinkState: [] });
	});

	/** A stored filter the view no longer declares still has to stop hiding things. */
	it('empties a stored filter the view no longer declares', () => {
		const cleared = clearedHideSetFor('L2Physical', 'Interface', { Staleness: ['Stale'] });

		expect(cleared).toEqual({ LinkState: [], Staleness: [] });
	});
});
