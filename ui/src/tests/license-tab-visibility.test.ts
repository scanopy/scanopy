import { describe, it, expect } from 'vitest';
import { licenseTabVisible } from '$lib/features/settings/license-tab';

type Flags = Parameters<typeof licenseTabVisible>[0];

const FLAGS = [
	'isOwner',
	'isDemoOrg',
	'billingEnabled',
	'signingAvailable',
	'hasLicensedPlan',
	'licensedPlanPending'
] as const satisfies readonly (keyof Flags)[];

/** Every combination of the six flags that also matches `fixed`. */
function combinations(fixed: Partial<Flags>): Flags[] {
	const all: Flags[] = [];
	for (let bits = 0; bits < 1 << FLAGS.length; bits++) {
		const flags = Object.fromEntries(FLAGS.map((name, i) => [name, (bits & (1 << i)) !== 0]));
		all.push(flags as Flags);
	}
	return all.filter((flags) =>
		FLAGS.every((name) => !(name in fixed) || flags[name] === fixed[name])
	);
}

// An owner of a real org on a server that sells and signs licenses.
const eligible = {
	isOwner: true,
	isDemoOrg: false,
	billingEnabled: true,
	signingAvailable: true
};

describe('License tab visibility', () => {
	it('never shows to a non-owner', () => {
		const cases = combinations({ isOwner: false });
		expect(cases).toHaveLength(32);
		expect(cases.filter(licenseTabVisible)).toEqual([]);
	});

	it('never shows on a demo org', () => {
		const cases = combinations({ isDemoOrg: true });
		expect(cases).toHaveLength(32);
		expect(cases.filter(licenseTabVisible)).toEqual([]);
	});

	// The org's plan only flips after the Stripe webhook, and the forced Settings
	// modal opens on this tab before that.
	it('shows while a licensed plan is pending and the org has not caught up', () => {
		expect(
			licenseTabVisible({ ...eligible, hasLicensedPlan: false, licensedPlanPending: true })
		).toBe(true);
	});

	it('stays hidden on a licensed plan when the server cannot sign or does not bill', () => {
		const licensed = { ...eligible, hasLicensedPlan: true };
		for (const licensedPlanPending of [false, true]) {
			expect(licenseTabVisible({ ...licensed, licensedPlanPending, signingAvailable: false })).toBe(
				false
			);
			expect(licenseTabVisible({ ...licensed, licensedPlanPending, billingEnabled: false })).toBe(
				false
			);
		}
	});
});
