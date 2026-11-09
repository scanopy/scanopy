import { describe, it, expect } from 'vitest';
import {
	isPlanUnchosen,
	isSelfHostedPlanLocked,
	type Organization
} from '$lib/features/organizations/types';

type Plan = NonNullable<Organization['plan']>;

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

function plan(type: string): Plan {
	return { type } as Plan;
}

// Each org state the Settings lock has to leave alone, and the one it adds.
const states: Record<string, Organization> = {
	free: org({ plan: plan('Free') }),
	licenseTrialNotStarted: org({ plan: plan('SelfHostedStandard'), plan_status: null }),
	trialing: org({ plan: plan('Pro'), plan_status: 'trialing' }),
	active: org({ plan: plan('Business'), plan_status: 'active' }),
	pastDue: org({ plan: plan('Pro'), plan_status: 'past_due' }),
	lapsed: org({ plan: plan('Pro'), plan_status: 'cancelled' }),
	licensed: org({ plan: plan('SelfHostedStandard'), plan_status: 'active' }),
	community: org({ plan: plan('Community') })
};

function lockedStates(isCloud: boolean): Record<string, string | null> {
	return Object.fromEntries(
		Object.entries(states).map(([name, o]) => [
			name,
			isPlanUnchosen(o, isCloud)
				? 'no-plan'
				: isSelfHostedPlanLocked(o, isCloud)
					? 'licensed'
					: null
		])
	);
}

describe('the Settings lock', () => {
	it('holds a license org whose trial never started, and a licensed one, and nothing else', () => {
		expect(lockedStates(true)).toEqual({
			free: null,
			licenseTrialNotStarted: 'no-plan',
			trialing: null,
			active: null,
			pastDue: null,
			lapsed: null,
			licensed: 'licensed',
			community: null
		});
	});

	it('never holds an org on a self-hosted server', () => {
		expect(Object.values(lockedStates(false)).every((lock) => lock === null)).toBe(true);
	});

	it('waits for the org to load', () => {
		expect(isPlanUnchosen(undefined, true)).toBe(false);
		expect(isPlanUnchosen(null, true)).toBe(false);
	});
});
