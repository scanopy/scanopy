import { describe, expect, it } from 'vitest';
import type { Organization } from '#lib/features/organizations/types.ts';
import { trialChargeDate } from '#lib/shared/utils/trial.ts';

const now = Date.UTC(2026, 9, 25);
const day = 24 * 60 * 60 * 1000;

function org(fields: Partial<Organization>): Organization {
	return { plan_status: null, trial_end_date: null, ...fields } as Organization;
}

function plan(trial_days: number): Organization['plan'] {
	return { trial_days } as Organization['plan'];
}

describe('trialChargeDate', () => {
	it('keeps the current trial end for a trialing org, whatever plan is picked', () => {
		const trialing = org({ plan_status: 'trialing', trial_end_date: '2026-11-01T00:00:00Z' });
		expect(trialChargeDate(trialing, plan(14), now)).toEqual(new Date('2026-11-01T00:00:00Z'));
		expect(trialChargeDate(trialing, null, now)).toEqual(new Date('2026-11-01T00:00:00Z'));
	});

	it('keeps the trial end when a cancellation is scheduled mid-trial', () => {
		const cancelling = org({
			plan_status: 'pending_cancellation',
			trial_end_date: '2026-11-09T03:47:44Z'
		});
		expect(trialChargeDate(cancelling, plan(14), now)).toEqual(new Date('2026-11-09T03:47:44Z'));
	});

	it("counts the picked plan's trial from now for an org that never trialed", () => {
		expect(trialChargeDate(org({}), plan(14), now)).toEqual(new Date(now + 14 * day));
	});

	it('charges straight away once the trial is used or the plan has none', () => {
		const used = org({ plan_status: null, trial_end_date: '2026-09-01T00:00:00Z' });
		expect(trialChargeDate(used, plan(14), now)).toBeNull();
		expect(trialChargeDate(org({}), plan(0), now)).toBeNull();
		expect(trialChargeDate(org({ plan_status: 'active' }), null, now)).toBeNull();
		const paidCancelling = org({
			plan_status: 'pending_cancellation',
			trial_end_date: '2026-09-01T00:00:00Z'
		});
		expect(trialChargeDate(paidCancelling, null, now)).toBeNull();
	});
});
