import { describe, it, expect } from 'vitest';
import {
	liveUnifiedScans,
	scansTargeting,
	withRoutedTarget
} from '#lib/features/discovery/utils/targets.ts';
import type { Discovery } from '#lib/features/discovery/types/base.ts';

const HQ = 'site-hq';

function scan(
	id: string,
	subnet_ids: string[] | null,
	{
		site_id = HQ,
		historical = false,
		type = 'Unified'
	}: { site_id?: string; historical?: boolean; type?: 'Unified' | 'Network' } = {}
): Discovery {
	return {
		id,
		name: id,
		site_id,
		daemon_id: 'daemon',
		run_type: historical ? { type: 'Historical', results: {} } : { type: 'AdHoc', last_run: null },
		discovery_type: { type, subnet_ids, host_id: 'host' }
	} as unknown as Discovery;
}

describe('scan targets', () => {
	it('names the live Unified scans that sweep a subnet as routed', () => {
		const scans = [
			scan('routes-it', ['branch']),
			scan('interfaced-only', null),
			scan('past-run', ['branch'], { historical: true }),
			scan('legacy', ['branch'], { type: 'Network' })
		];

		expect(scansTargeting('branch', scans).map((d) => d.id)).toEqual(['routes-it']);
	});

	it('lists the live Unified scans on a site', () => {
		const scans = [scan('hq', null), scan('dc', null, { site_id: 'site-dc' })];

		expect(liveUnifiedScans(scans, HQ).map((d) => d.id)).toEqual(['hq']);
		expect(liveUnifiedScans(scans)).toHaveLength(2);
	});

	it('adds a routed network to a scan once, keeping the ones it had', () => {
		const interfacedOnly = withRoutedTarget(scan('s', null), 'branch');
		const again = withRoutedTarget(interfacedOnly, 'branch');
		const second = withRoutedTarget(again, 'annex');

		expect(again.discovery_type).toMatchObject({ subnet_ids: ['branch'] });
		expect(second.discovery_type).toMatchObject({ subnet_ids: ['branch', 'annex'] });
	});
});
