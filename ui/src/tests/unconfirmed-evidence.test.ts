import { describe, expect, it } from 'vitest';
import { toHostPrimitive } from '#lib/features/hosts/queries.ts';
import { unconfirmedEvidence } from '#lib/features/hosts/unconfirmed-evidence.ts';
import type { HostResponse, Interface } from '#lib/features/hosts/types/base.ts';

const SWITCH = 'switch-id';
const SCAN = 'scan-id';

/**
 * A host as `GET /api/v1/hosts` returns it, children included. The Hosts list keeps the host as a
 * primitive (`toHostPrimitive`) and puts its interfaces in a cache of their own, which is the
 * shape the resolve modal is handed.
 */
function listed(fields: Partial<HostResponse>, interfaces: Partial<Interface>[] = []) {
	const response = {
		id: 'host-id',
		site_id: 'site-id',
		source: { type: 'Inferred', via: 'ForwardingTable' },
		reported_by_host_ids: [SWITCH],
		found_by_discovery_id: SCAN,
		ip_addresses: [],
		ports: [],
		services: [],
		interfaces: interfaces.map((i) => ({ host_id: 'host-id', ...i })),
		...fields
	} as unknown as HostResponse;
	return {
		host: toHostPrimitive(response),
		interfaceCache: response.interfaces as Interface[]
	};
}

describe('what the resolve modal shows for an unconfirmed host, through the Hosts list', () => {
	// The MAC rides only on an interface, which the list strips from the host itself: the case
	// the modal lost when it read `host.interfaces`.
	it('a forwarding-table host carries its reporter, its scan and the MAC on its interface', () => {
		const { host, interfaceCache } = listed({}, [{ mac_address: '00:80:63:2A:51:07' }]);
		expect(unconfirmedEvidence(host, interfaceCache)).toEqual({
			reportedByHostIds: [SWITCH],
			sourceDiscoveryId: SCAN,
			chassisId: null,
			sysName: null,
			macs: ['00:80:63:2A:51:07']
		});
	});

	it('a neighbour-advertised host carries the chassis ID it advertised', () => {
		const { host, interfaceCache } = listed(
			{
				source: { type: 'Inferred', via: 'Neighbour' },
				chassis_id: 'hq-annex-uplink'
			} as Partial<HostResponse>,
			[{}]
		);
		const evidence = unconfirmedEvidence(host, interfaceCache);
		expect(evidence.chassisId).toBe('hq-annex-uplink');
		expect(evidence.reportedByHostIds).toEqual([SWITCH]);
		expect(evidence.sourceDiscoveryId).toBe(SCAN);
		expect(evidence.macs).toEqual([]);
	});

	it("leaves out other hosts' interfaces in the cache", () => {
		const { host, interfaceCache } = listed({}, [{ mac_address: '00:80:63:2A:51:07' }]);
		const other = { host_id: 'other-host', mac_address: 'aa:bb:cc:dd:ee:ff' } as Interface;
		expect(unconfirmedEvidence(host, [...interfaceCache, other]).macs).toEqual([
			'00:80:63:2A:51:07'
		]);
	});
});
