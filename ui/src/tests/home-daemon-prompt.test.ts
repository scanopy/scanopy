import { describe, expect, it } from 'vitest';
import type { components } from '$lib/api/schema';
import { homeDaemonPrompt } from '$lib/shared/onboarding/checklist';

type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];

const preDaemon: OnboardingOperation[] = ['OrgCreated'];
const withDaemon: OnboardingOperation[] = ['OrgCreated', 'FirstDaemonRegistered'];

describe('homeDaemonPrompt', () => {
	it('fills in when the checklist is dismissed before the first daemon', () => {
		expect(homeDaemonPrompt(preDaemon, true)).toBe(true);
	});

	it('shows nothing once the first daemon registers', () => {
		expect(homeDaemonPrompt(withDaemon, true)).toBe(false);
	});

	it('defers to the checklist while it is visible', () => {
		expect(homeDaemonPrompt(preDaemon, false)).toBe(false);
	});
});
