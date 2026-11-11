import { describe, it, expect } from 'vitest';
import { credentialTypeUpgradeFeature } from '#lib/features/credentials/types/base.ts';
import credentialTypes from '../lib/data/credential-types.json';
import billingPlans from '../lib/data/billing-plans-all.json';

type Features = Record<string, boolean | number | null>;

const gatedTypes = credentialTypes.filter((t) => t.metadata.required_feature);
const ungatedTypes = credentialTypes.filter((t) => !t.metadata.required_feature);

/** Plans picked by what they include rather than by name, so a plan edit doesn't break the cases. */
function plansWhere(feature: string, included: boolean): string[] {
	return billingPlans
		.filter((p) => Boolean((p.metadata.features as Features)[feature]) === included)
		.map((p) => p.id);
}

describe('credentialTypeUpgradeFeature', () => {
	it('gates a type exactly on the plans that lack its feature', () => {
		expect(gatedTypes.length).toBeGreaterThan(0);
		for (const type of gatedTypes) {
			const feature = type.metadata.required_feature as string;
			for (const plan of plansWhere(feature, false)) {
				expect(credentialTypeUpgradeFeature(type.id, plan)).toBe(feature);
			}
			for (const plan of plansWhere(feature, true)) {
				expect(credentialTypeUpgradeFeature(type.id, plan)).toBeNull();
			}
		}
	});

	it('never gates a type that needs no feature', () => {
		for (const type of ungatedTypes) {
			for (const plan of billingPlans) {
				expect(credentialTypeUpgradeFeature(type.id, plan.id)).toBeNull();
			}
		}
	});

	// The organization query may not have loaded; the server still refuses the create.
	it('gates nothing without a plan', () => {
		for (const type of gatedTypes) {
			expect(credentialTypeUpgradeFeature(type.id, null)).toBeNull();
		}
	});
});
