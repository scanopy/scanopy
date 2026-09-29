import { describe, expect, it } from 'vitest';
import { supportEmailAddress } from '$lib/features/support/support';
import billingPlans from '$lib/data/billing-plans-all.json';

describe('supportEmailAddress', () => {
	it('gives every plan an address matching its support flags', () => {
		for (const plan of billingPlans) {
			const { email_support, priority_support } = plan.metadata.features;
			const address = supportEmailAddress(plan.metadata.features);

			if (!email_support) {
				expect(address).toBeNull();
			} else if (priority_support) {
				expect(address).not.toBe(supportEmailAddress({ email_support, priority_support: false }));
			} else {
				expect(address).not.toBeNull();
			}
		}
	});
});
