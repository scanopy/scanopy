import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { topologySSEManager } from '$lib/features/topology/queries';
import { queryClient } from '$lib/api/query-client';

/**
 * The topology SSE stream pings ~5×/sec/site during discovery, and each ping
 * would otherwise refetch the uncached, full-rebuild `/topology/data`. The
 * manager coalesces a burst of pings into at most one invalidation pass per
 * ~1s window (trailing edge). These tests assert the coalescing *ratio* and
 * that the final ping is never dropped — behavior, not the throttle constant.
 */

// The manager wires its ping handler inside createConfig(); reach it to drive pings.
function ping(siteId: string) {
	const cfg = (
		topologySSEManager as unknown as {
			createConfig(): { onMessage(u: { site_id: string }): void };
		}
	).createConfig();
	cfg.onMessage({ site_id: siteId });
}

describe('TopologySSEManager ping coalescing', () => {
	let invalidateSpy: ReturnType<typeof vi.spyOn>;

	beforeEach(() => {
		vi.useFakeTimers();
		invalidateSpy = vi
			.spyOn(queryClient, 'invalidateQueries')
			.mockImplementation(() => Promise.resolve());
	});

	afterEach(() => {
		topologySSEManager.disconnect();
		vi.clearAllTimers();
		vi.useRealTimers();
		vi.restoreAllMocks();
	});

	it('collapses a burst of pings for one site into a single flush', () => {
		for (let i = 0; i < 20; i++) ping('net-a');
		// Trailing throttle: nothing fires until the window elapses.
		expect(invalidateSpy).not.toHaveBeenCalled();

		vi.advanceTimersByTime(1000);

		// One flush for one site = one topology-data predicate invalidation
		// + one snapshots invalidation (org is absent from cache, so skipped).
		// The point: 20 pings did NOT produce 20 (or 40) invalidations.
		expect(invalidateSpy.mock.calls.length).toBe(2);
	});

	it('bounds refetches under a sustained ping stream (~1 flush per window)', () => {
		// Three windows of continuous pinging → ~3 flushes, not one-per-ping.
		for (let w = 0; w < 3; w++) {
			for (let i = 0; i < 5; i++) ping('net-a');
			vi.advanceTimersByTime(1000);
		}
		// 15 pings, 3 windows → 3 flushes × 2 invalidations each = 6.
		expect(invalidateSpy.mock.calls.length).toBe(6);
	});

	it('always fires a trailing flush for a lone ping (final state converges)', () => {
		ping('net-a');
		expect(invalidateSpy).not.toHaveBeenCalled();
		vi.advanceTimersByTime(1000);
		expect(invalidateSpy.mock.calls.length).toBe(2);
	});

	it('drains every pinged site in one flush', () => {
		ping('net-a');
		ping('net-b');
		ping('net-a');
		vi.advanceTimersByTime(1000);

		// One predicate pass (covers both sites' data + the list) plus one
		// snapshots invalidation per distinct site (a, b) = 1 + 2 = 3.
		expect(invalidateSpy.mock.calls.length).toBe(3);

		// The snapshot invalidations target both distinct sites.
		const snapshotCalls = invalidateSpy.mock.calls.filter((c: unknown[]) => {
			const key = (c[0] as { queryKey?: readonly unknown[] })?.queryKey;
			return Array.isArray(key) && key.includes('snapshots');
		});
		expect(snapshotCalls.length).toBe(2);
	});
});
