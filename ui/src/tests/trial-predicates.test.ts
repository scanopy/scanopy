import { describe, it, expect } from 'vitest';
import { isTrialingWithoutPayment } from '$lib/shared/utils/trial';
import type { Organization } from '$lib/features/organizations/types';

function org(overrides: Partial<Organization> = {}): Organization {
	return {
		id: '00000000-0000-4000-8000-000000000000',
		created_at: '2026-01-01T00:00:00Z',
		updated_at: '2026-01-01T00:00:00Z',
		name: 'Acme',
		onboarding: [],
		plan: null,
		plan_status: null,
		...overrides
	};
}

describe('trial nags follow the deployment, not the org row', () => {
	// A server that once ran with Stripe configured keeps plan_status and
	// has_payment_method on its org rows forever — only Stripe webhooks write
	// them, so nothing clears them once Stripe is switched off. Acting on that
	// stale state nagged self-hosted users about a subscription they can't have.
	const trialingWithoutCard = org({ plan_status: 'trialing', has_payment_method: false });

	it('stays quiet when the deployment does not bill', () => {
		expect(isTrialingWithoutPayment(trialingWithoutCard, false)).toBe(false);
	});

	it('still fires when the deployment bills', () => {
		expect(isTrialingWithoutPayment(trialingWithoutCard, true)).toBe(true);
	});
});
