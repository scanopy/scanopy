import { describe, expect, it } from 'vitest';
import type { components } from '$lib/api/schema';
import { homeDaemonPrompt } from '$lib/shared/onboarding/checklist';

type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];

const preDaemon: OnboardingOperation[] = ['OrgCreated'];
const withDaemon: OnboardingOperation[] = ['OrgCreated', 'FirstDaemonRegistered'];

describe('homeDaemonPrompt', () => {
	it('offers the install CTA when the checklist is dismissed before the first daemon', () => {
		expect(homeDaemonPrompt(preDaemon, true, false)).toBe('install');
	});

	it('shows nothing once the first daemon registers', () => {
		expect(homeDaemonPrompt(withDaemon, true, false)).toBeNull();
		expect(homeDaemonPrompt(withDaemon, true, true)).toBeNull();
	});

	it('defers to the checklist while it is visible', () => {
		expect(homeDaemonPrompt(preDaemon, false, false)).toBeNull();
		expect(homeDaemonPrompt(preDaemon, false, true)).toBeNull();
	});

	it('drops the install action for read-only users', () => {
		expect(homeDaemonPrompt(preDaemon, true, true)).toBe('readOnly');
	});
});
