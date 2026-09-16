/**
 * The payload summaries have to stay affordable on the pan path.
 *
 * `sampleViewerState` runs on every viewport move, throttled to twice a second, and these two
 * walks are O(edges + interfaces) with no cap — at the scale the diagnostic exists to explain that
 * is thousands of edges and ~15k interfaces. `CULLABILITY_SAMPLE_LIMIT` caps the neighbouring walk
 * for exactly this reason; this one is memoised instead, because a payload cannot change between
 * refetches, so the exact figures cost nothing to keep.
 */
import { describe, it, expect, beforeEach } from 'vitest';
import {
	summarisePayload,
	resetDiagnostics,
	type DiagnosablePayload
} from '$lib/features/topology/diagnostics';

/**
 * A payload that reports how many times its collections were walked.
 *
 * GH #701: an interface's resolution is a `Vec` of rows on `neighbours` rather than a scalar
 * `neighbor` field, so `interfaces` here carries only ids and the resolved kind lives on the
 * separate `neighbours` array, matched by `interface_id`.
 */
function countingPayload(): { payload: DiagnosablePayload; reads: () => number } {
	let reads = 0;
	const edges = [
		{ edge_type: 'PhysicalLink' },
		{ edge_type: 'PhysicalLink' },
		{ edge_type: 'NeighborLink' }
	];
	const interfaces = [{ id: 'a' }, { id: 'b' }, { id: 'c' }];
	const neighbours = [
		{ interface_id: 'a', neighbor: { type: 'Interface' as const, id: 'x' } },
		{ interface_id: 'b', neighbor: { type: 'Host' as const, id: 'y' } }
	];
	const payload = {
		get edges() {
			reads += 1;
			return edges;
		},
		get interfaces() {
			return interfaces;
		},
		get neighbours() {
			return neighbours;
		}
	} as DiagnosablePayload;
	return { payload, reads: () => reads };
}

describe('summarisePayload', () => {
	beforeEach(resetDiagnostics);

	it('counts edges by type and interfaces by how far their neighbour resolved', () => {
		const { payload } = countingPayload();

		const summary = summarisePayload(payload);

		expect(summary.edgesByType).toEqual({ PhysicalLink: 2, NeighborLink: 1 });
		expect(summary.interfaceNeighborKinds).toEqual({ Interface: 1, Host: 1, none: 1 });
	});

	it('counts a port with several resolved rows in every bucket its rows fall into', () => {
		// GH #701: a port can now carry more than one adjacency row (e.g. an LLDP entry and a CDP
		// entry resolving to different neighbours), so it is no longer a partition of the interface
		// set the way a single `neighbor` field was — one interface can land in both buckets.
		const payload = {
			edges: [],
			interfaces: [{ id: 'a' }],
			neighbours: [
				{ interface_id: 'a', neighbor: { type: 'Interface' as const, id: 'x' } },
				{ interface_id: 'a', neighbor: { type: 'Host' as const, id: 'y' } }
			]
		} as DiagnosablePayload;

		const summary = summarisePayload(payload);

		expect(summary.interfaceNeighborKinds).toEqual({ Interface: 1, Host: 1, none: 0 });
	});

	it('walks a payload once however many samples are taken from it', () => {
		const { payload, reads } = countingPayload();

		for (let i = 0; i < 20; i++) summarisePayload(payload);

		expect(reads()).toBe(1);
	});

	it('re-reads when the payload is replaced, so a refetch is never reported stale', () => {
		const first = countingPayload();
		const second = countingPayload();

		summarisePayload(first.payload);
		summarisePayload(second.payload);

		expect(second.reads()).toBe(1);
	});

	it('reports empty counts rather than throwing when there is no payload', () => {
		const summary = summarisePayload(null);

		expect(summary.edgesByType).toEqual({});
		expect(summary.interfaceNeighborKinds).toEqual({ Interface: 0, Host: 0, none: 0 });
	});
});
