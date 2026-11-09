import { describe, expect, it } from 'vitest';
import { homeDaemonPrompt } from '$lib/shared/onboarding/checklist';

describe('homeDaemonPrompt', () => {
	it('fills in when the checklist is dismissed before a daemon connects', () => {
		expect(homeDaemonPrompt(false, true)).toBe(true);
	});

	it('shows nothing once a daemon has connected', () => {
		expect(homeDaemonPrompt(true, true)).toBe(false);
	});

	it('defers to the checklist while it is visible', () => {
		expect(homeDaemonPrompt(false, false)).toBe(false);
	});
});
