import { describe, it, expect } from 'vitest';
import { asksForSite, onboardingSteps } from '#lib/features/auth/stores/onboarding.ts';

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

describe('onboardingSteps', () => {
	it('asks where Scanopy runs before anything else when the server sells both', () => {
		for (const hosting of ['cloud', 'self_hosted', null] as const) {
			expect(onboardingSteps(true, hosting)[0]).toBe('hosting');
		}
	});

	it('skips the use case only for a self-hosted license buyer', () => {
		expect(onboardingSteps(true, 'self_hosted')).not.toContain('use_case');
		expect(onboardingSteps(true, 'cloud')).toContain('use_case');
		expect(onboardingSteps(true, null)).toContain('use_case');
	});

	it('ignores a hosting value on a server that does not ask the question', () => {
		expect(onboardingSteps(false, 'self_hosted')).toEqual(onboardingSteps(false, null));
		expect(onboardingSteps(false, null)).not.toContain('hosting');
	});
});
