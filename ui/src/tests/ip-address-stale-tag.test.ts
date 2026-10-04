import { describe, it, expect, vi, afterEach } from 'vitest';
import { IPAddressDisplay } from '$lib/shared/components/forms/selection/display/IPAddressDisplay.svelte';
import { displayTags } from '$lib/shared/components/forms/selection/display-tags';
import type { IPAddress } from '$lib/features/hosts/types/base';
import type { Network } from '$lib/features/networks/types';
import type { Subnet } from '$lib/features/subnets/types/base';
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

	// A long IPv6 address can leave room for one tag or none. The status goes first, so it is the
	// tag that shows, and the first one the "+N more" tooltip names.
	it('comes before the subnet tag', () => {
		vi.useFakeTimers();
		vi.setSystemTime(NOW);
		const subnet = { id: 's1', cidr: '192.168.4.0/22', subnet_type: 'Lan' } as Subnet;
		const tags = displayTags(IPAddressDisplay, address(24 * 40), {
			subnets: [subnet],
			networks: [network]
		});
		expect(tags.map((t) => t.label)).toEqual([common_stale(), subnet.cidr]);
	});
});

describe('IPAddressDisplay row', () => {
	// The address takes the label line on its own, so it's the last thing a narrow row truncates;
	// the interface name moves down to the description.
	it('labels the row by the address and puts the name underneath', () => {
		const ipv6 = 'fd0b:d38d:98f6:1:1282:6e6a:66b1:3224';
		const iface = {
			...address(0),
			ip_address: ipv6,
			name: 'enp0s19',
			mac_address: 'BC:24:11:70:13:4A'
		} as IPAddress;
		const context = { subnets: [] };
		expect(IPAddressDisplay.getLabel(iface, context)).toBe(ipv6);
		const description = IPAddressDisplay.getDescription!(iface, context);
		expect(description).toContain('enp0s19');
		expect(description).toContain('BC:24:11:70:13:4A');
	});
});
