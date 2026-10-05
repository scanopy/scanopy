import { describe, it, expect } from 'vitest';
import { asksForSite } from '$lib/features/auth/stores/onboarding';

describe('asksForSite', () => {
	it('skips the site step only for a self-hosted license buyer on cloud', () => {
		expect(asksForSite('self_hosted', true)).toBe(false);
		expect(asksForSite('cloud', true)).toBe(true);
		expect(asksForSite(null, true)).toBe(true);
	});

	it('always asks on a self-hosted instance, where nothing creates a site later', () => {
		expect(asksForSite('self_hosted', false)).toBe(true);
	});
});
