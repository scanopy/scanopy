import { describe, it, expect } from 'vitest';
import { asksForNetwork } from '$lib/features/auth/stores/onboarding';

describe('asksForNetwork', () => {
	it('skips the network step only for a self-hosted license buyer on cloud', () => {
		expect(asksForNetwork('self_hosted', true)).toBe(false);
		expect(asksForNetwork('cloud', true)).toBe(true);
		expect(asksForNetwork(null, true)).toBe(true);
	});

	it('always asks on a self-hosted instance, where nothing creates a network later', () => {
		expect(asksForNetwork('self_hosted', false)).toBe(true);
	});
});
