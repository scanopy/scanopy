import { beforeEach, describe, expect, it } from 'vitest';
import { get } from 'svelte/store';
import {
	STALL_THRESHOLD_MS,
	isStalled,
	observeSessions,
	recordSessionMessage,
	sessionLastMessageAt
} from '$lib/features/discovery/utils/staleness';

const T0 = 1_000_000;
const past = T0 + STALL_THRESHOLD_MS + 1;

describe('isStalled', () => {
	it('flags a scanning session silent past the threshold on a live stream', () => {
		expect(isStalled('Scanning', T0, true, past)).toBe(true);
		expect(isStalled('Scanning', T0, true, T0 + STALL_THRESHOLD_MS)).toBe(false);
	});

	it('says nothing while the stream is down', () => {
		expect(isStalled('Scanning', T0, false, past)).toBe(false);
	});

	it('leaves phases that wait by design alone', () => {
		for (const phase of ['Queued', 'AwaitingSnapshot', 'Pending', 'Starting', 'Cancelling']) {
			expect(isStalled(phase, T0, true, past)).toBe(false);
		}
	});

	it('needs a starting point before it can call anything silent', () => {
		expect(isStalled('Scanning', undefined, true, past)).toBe(false);
	});
});

describe('session clocks', () => {
	beforeEach(() => sessionLastMessageAt.set(new Map()));

	it('starts a clock for a session first seen in the session list', () => {
		observeSessions(['a'], T0);
		expect(get(sessionLastMessageAt).get('a')).toBe(T0);
	});

	it('keeps the arrival time of a session the stream already reported', () => {
		recordSessionMessage('a', T0 + 500);
		observeSessions(['a', 'b'], T0 + 1_000);
		expect(get(sessionLastMessageAt).get('a')).toBe(T0 + 500);
		expect(get(sessionLastMessageAt).get('b')).toBe(T0 + 1_000);
	});
});
