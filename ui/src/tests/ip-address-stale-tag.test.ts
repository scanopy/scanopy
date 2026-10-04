import { describe, it, expect, vi, afterEach } from 'vitest';
import { IPAddressDisplay } from '$lib/shared/components/forms/selection/display/IPAddressDisplay.svelte';
import { displayTags } from '$lib/shared/components/forms/selection/display-tags';
import type { IPAddress } from '$lib/features/hosts/types/base';
import type { Network } from '$lib/features/networks/types';
import { common_stale } from '$lib/paraglide/messages';

const HOUR_MS = 60 * 60 * 1000;
const NOW = new Date('2026-10-04T12:00:00Z').getTime();

const network = { id: 'n1', effective_stale_after_hours: 24 * 28 } as Network;

function address(hoursAgo: number): IPAddress {
	return {
		id: 'a1',
		network_id: 'n1',
		subnet_id: 's1',
		host_id: 'h1',
		ip_address: '192.168.4.63',
		last_seen_at: new Date(NOW - hoursAgo * HOUR_MS).toISOString()
	} as IPAddress;
}

function staleTags(iface: IPAddress, compact = false) {
	return displayTags(IPAddressDisplay, iface, { subnets: [], networks: [network], compact }).filter(
		(t) => t.label === common_stale()
	);
}

afterEach(() => vi.useRealTimers());

/**
 * Discovery refreshes `last_seen_at` only on the addresses it reports, so an address a host
 * stopped answering on ages on its own while the host stays current. The address list is where
 * that shows.
 */
describe('IPAddressDisplay stale tag', () => {
	it('flags an address past its network window and leaves a current one alone', () => {
		vi.useFakeTimers();
		vi.setSystemTime(NOW);
		expect(staleTags(address(24 * 40))).toHaveLength(1);
		expect(staleTags(address(2))).toHaveLength(0);
	});

	// Compact mode drops the subnet tag, which topology shows as the container. The verdict has
	// nowhere else to appear in the inspector.
	it('keeps the verdict in compact mode', () => {
		vi.useFakeTimers();
		vi.setSystemTime(NOW);
		expect(staleTags(address(24 * 40), true)).toHaveLength(1);
	});
});
