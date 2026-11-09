import { beforeEach, describe, expect, it } from 'vitest';
import { get } from 'svelte/store';
import {
	ACTIVITY_STALL_THRESHOLD_MS,
	STALL_THRESHOLD_MS,
	isStalled,
	observeSessions,
	recordSessionActivity,
	recordSessionMessage,
	sessionLastActivityAt,
	sessionLastMessageAt,
	sessionStallOf
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

describe('sessionStallOf', () => {
	const stuck = T0 + ACTIVITY_STALL_THRESHOLD_MS + 60_000;

	it('flags a session that keeps reporting but stopped advancing, timed from its activity', () => {
		// Messages arrive every 30s; the last advance was at T0.
		expect(sessionStallOf('Scanning', stuck - 30_000, T0, true, stuck)).toEqual({
			minutes: 6,
			source: 'activity'
		});
	});

	it('leaves a reporting session alone while it advanced within the threshold', () => {
		const recent = T0 + ACTIVITY_STALL_THRESHOLD_MS;
		expect(sessionStallOf('Scanning', recent - 30_000, T0, true, recent)).toBeNull();
	});

	it('times an old daemon, which sends no activity, from message arrival', () => {
		expect(sessionStallOf('Scanning', T0, undefined, true, past)).toEqual({
			minutes: 1,
			source: 'messages'
		});
		expect(sessionStallOf('Scanning', T0 + 60_000, undefined, true, stuck)).toEqual({
			minutes: 5,
			source: 'messages'
		});
	});

	it('names silence over a stalled scan when the daemon stopped reporting too', () => {
		expect(sessionStallOf('Scanning', T0, T0, true, stuck)?.source).toBe('messages');
	});

	it('says nothing while the stream is down or the session is not scanning', () => {
		expect(sessionStallOf('Scanning', stuck, T0, false, stuck)).toBeNull();
		expect(sessionStallOf('Queued', stuck, T0, true, stuck)).toBeNull();
	});
});

describe('session clocks', () => {
	beforeEach(() => {
		sessionLastMessageAt.set(new Map());
		sessionLastActivityAt.set(new Map());
	});

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

	it('records activity from the payload and keeps it when a later payload has none', () => {
		const at = '2026-11-06T12:00:00Z';
		recordSessionActivity({ session_id: 'a', last_activity_at: at });
		recordSessionActivity({ session_id: 'a', last_activity_at: null });
		expect(get(sessionLastActivityAt).get('a')).toBe(Date.parse(at));
	});
});
