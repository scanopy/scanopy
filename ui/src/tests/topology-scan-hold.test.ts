import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { get } from 'svelte/store';
import {
	heldTopologySites,
	heldTopologyUpdates,
	invalidateLiveTopology,
	refreshHeldTopology,
	setTopologyHeld
} from '#lib/features/topology/queries.ts';
import { queryClient, queryKeys } from '#lib/api/query-client.ts';

/**
 * While a scan runs on a large site, the layout is held: the change stream must not refetch that
 * site's live bundle (each refetch is a full layout run), but must record that a change arrived so
 * the viewer can offer a refresh, and releasing the hold must fetch once.
 */

const heldLive = queryKeys.topology.data('held', undefined, 'L3Logical');
const heldOtherView = queryKeys.topology.data('held', undefined, 'L2Physical');
const heldSnapshot = queryKeys.topology.data('held', 'snap-1', 'L3Logical');
const otherLive = queryKeys.topology.data('other', undefined, 'L3Logical');

function invalidated(key: readonly unknown[]): boolean {
	return queryClient.getQueryState(key)?.isInvalidated ?? false;
}

describe('holding a site during a scan', () => {
	beforeEach(() => {
		queryClient.clear();
		heldTopologySites.set(new Set());
		heldTopologyUpdates.set(new Set());
		for (const key of [heldLive, heldOtherView, heldSnapshot, otherLive]) {
			queryClient.setQueryData(key, { hosts: [] });
		}
	});

	afterEach(() => {
		queryClient.clear();
	});

	it('does not refetch a held site on a change, and records the change', () => {
		setTopologyHeld('held', true);

		invalidateLiveTopology(new Set(['held', 'other']));

		expect(invalidated(heldLive)).toBe(false);
		expect(invalidated(heldOtherView)).toBe(false);
		expect(get(heldTopologyUpdates).has('held')).toBe(true);
		// Sites that are not held follow the scan as before.
		expect(invalidated(otherLive)).toBe(true);
		// Snapshots never change.
		expect(invalidated(heldSnapshot)).toBe(false);
	});

	it('fetches a held site once on release when it changed', () => {
		setTopologyHeld('held', true);
		invalidateLiveTopology(new Set(['held']));

		setTopologyHeld('held', false);

		expect(invalidated(heldLive)).toBe(true);
		expect(invalidated(heldOtherView)).toBe(true);
		expect(get(heldTopologyUpdates).has('held')).toBe(false);
		expect(invalidated(heldSnapshot)).toBe(false);
	});

	it('leaves the bundle alone on release when nothing changed', () => {
		setTopologyHeld('held', true);
		setTopologyHeld('held', false);

		expect(invalidated(heldLive)).toBe(false);
	});

	it('refreshes on request and keeps holding', () => {
		setTopologyHeld('held', true);
		invalidateLiveTopology(new Set(['held']));

		refreshHeldTopology('held');

		expect(invalidated(heldLive)).toBe(true);
		expect(get(heldTopologyUpdates).has('held')).toBe(false);
		expect(get(heldTopologySites).has('held')).toBe(true);
	});
});
