import { describe, it, expect } from 'vitest';
import { canTrialTopology, topologyTrialDays } from '#lib/shared/utils/trial.ts';
import type { Organization } from '#lib/features/organizations/types.ts';

function org(overrides: Partial<Organization> = {}): Organization {
	return {
		id: '00000000-0000-4000-8000-000000000000',
		created_at: '2026-01-01T00:00:00Z',
		updated_at: '2026-01-01T00:00:00Z',
		name: 'Acme',
		setup: [],
		daemon_connected: false,
		plan: { type: 'Free' } as Organization['plan'],
		plan_status: null,
		...overrides
	};
}

const plan = (type: string) => ({ type }) as Organization['plan'];

// The topology gate offers a trial only when checkout would grant one (`checkout.rs`): never to an
// org that has had a paid subscription or a trial.
describe('topology gate trial offer', () => {
	it('is offered to a Free org that has never trialed', () => {
		expect(topologyTrialDays()).toBeGreaterThan(0);
		expect(canTrialTopology(org())).toBe(true);
	});

	it('is withheld once the org has trialed', () => {
		expect(canTrialTopology(org({ trial_end_date: '2026-03-01T00:00:00Z' }))).toBe(false);
	});

	it('is withheld from an org that has had a paid subscription', () => {
		expect(canTrialTopology(org({ plan: plan('Pro'), plan_status: 'cancelled' }))).toBe(false);
	});

	it('is offered on a license signup whose trial never started', () => {
		expect(canTrialTopology(org({ plan: plan('SelfHostedStandard'), plan_status: null }))).toBe(
			true
		);
	});
});
