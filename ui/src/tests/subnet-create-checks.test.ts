import { describe, it, expect } from 'vitest';
import { checkNewCidr } from '#lib/features/subnets/create-checks.ts';
import type { SubnetResponse } from '#lib/features/subnets/types/base.ts';
import type { components } from '#lib/api/schema';

type EntitySource = components['schemas']['EntitySource'];

const HQ = 'site-hq';
const DC = 'site-dc';

function subnet(
	id: string,
	cidr: string,
	site_id = HQ,
	source: EntitySource = { type: 'Discovery' }
): SubnetResponse {
	return {
		id,
		name: id,
		cidr,
		site_id,
		source,
		subnet_type: 'Lan',
		virtualization_service_id: null
	} as SubnetResponse;
}

const assumed: EntitySource = { type: 'Inferred', via: 'ReportedAddress' };

describe('checkNewCidr', () => {
	it('finds the subnet a range duplicates on the same site, and only there', () => {
		const lan = subnet('lan', '10.0.50.0/24');
		const dc = subnet('dc', '10.0.50.0/24', DC);

		const checks = checkNewCidr('10.0.50.0/24', HQ, [lan, dc]);

		expect(checks.sameHere?.id).toBe('lan');
		expect(checks.sameElsewhere.map((s) => s.id)).toEqual(['dc']);
	});

	it('treats an assumed range with the same CIDR as absorbed, not duplicated', () => {
		const guess = subnet('guess', '10.0.50.0/24', HQ, assumed);

		const checks = checkNewCidr('10.0.50.0/24', HQ, [guess]);

		expect(checks.sameHere).toBeNull();
		expect(checks.assumedInside.map((s) => s.id)).toEqual(['guess']);
	});

	it('splits the ranges inside the new one into assumed and settled, on this site only', () => {
		const subnets = [
			subnet('office', '10.0.10.0/24'),
			subnet('annex', '10.0.50.0/24', HQ, assumed),
			subnet('outside', '10.1.0.0/24'),
			subnet('other-site', '10.0.20.0/24', DC)
		];

		const checks = checkNewCidr('10.0.0.0/16', HQ, subnets);

		expect(checks.inside.map((s) => s.id)).toEqual(['office']);
		expect(checks.assumedInside.map((s) => s.id)).toEqual(['annex']);
	});

	it('ignores the organizational rows, which hold every address', () => {
		const internet = subnet('internet', '0.0.0.0/0', HQ, { type: 'System' });

		const checks = checkNewCidr('0.0.0.0/0', HQ, [internet]);

		expect(checks.sameHere).toBeNull();
	});
});
