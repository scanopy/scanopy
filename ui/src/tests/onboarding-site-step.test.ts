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
	it('asks where Scanopy runs first only when the server sells both', () => {
		expect(onboardingSteps(true)[0]).toBe('hosting');
		expect(onboardingSteps(false)).not.toContain('hosting');
	});

	it('asks every new org, a license buyer included, whose network it is and then what it wants', () => {
		for (const asksHosting of [true, false]) {
			const steps = onboardingSteps(asksHosting);
			expect(steps.indexOf('intents')).toBe(steps.indexOf('use_case') + 1);
			expect(steps.indexOf('use_case')).toBeLessThan(steps.indexOf('setup'));
			expect(steps.at(-1)).toBe('register');
		}
	});
});
