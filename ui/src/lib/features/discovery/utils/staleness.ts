import { derived, readable, writable } from 'svelte/store';
import type { DiscoveryUpdatePayload } from '../types/api';

/**
 * How long a scanning session may go without a message before the UI says so. A live daemon
 * reports at least every 30s (its heartbeat in DaemonPoll, the server's poll in ServerPoll), so
 * this is three missed reports.
 */
export const STALL_THRESHOLD_MS = 90_000;

/**
 * How long a scanning session may go without advancing (a progress rise, a host created, a deep
 * scan or an integration finished) while its daemon still reports, before the UI says so. A
 * healthy scan can work through a large subnet for minutes without any of these. The daemon ends
 * a session after 20 minutes without one.
 */
export const ACTIVITY_STALL_THRESHOLD_MS = 5 * 60_000;

/**
 * When each active session's last stream message arrived, by session id. Keyed on arrival rather
 * than on a change in progress: a healthy scan can hold one progress value for minutes while it
 * works through a large subnet, and still reports every 30s.
 */
export const sessionLastMessageAt = writable<Map<string, number>>(new Map());

/**
 * When each active session last advanced on its daemon, by session id, from the session's
 * `last_activity_at`. Daemons that predate the field leave a session out.
 */
export const sessionLastActivityAt = writable<Map<string, number>>(new Map());

/**
 * Whether the discovery stream is open. While it is down, silence says nothing about the daemon,
 * and the stream's own lost-connection error is the message that applies.
 */
export const discoveryStreamConnected = writable(false);

/** Wall-clock time, refreshed often enough for a minute-granularity label. */
export const now = readable(Date.now(), (set) => {
	const interval = setInterval(() => set(Date.now()), 5_000);
	return () => clearInterval(interval);
});

/** Records a message for `sessionId`. */
export function recordSessionMessage(sessionId: string, at: number): void {
	sessionLastMessageAt.update((m) => new Map(m).set(sessionId, at));
}

/** Records when a session last advanced, if its payload says. */
export function recordSessionActivity(
	session: Pick<DiscoveryUpdatePayload, 'session_id' | 'last_activity_at'>
): void {
	if (!session.last_activity_at) return;
	const at = Date.parse(session.last_activity_at);
	if (Number.isNaN(at)) return;
	sessionLastActivityAt.update((m) =>
		m.get(session.session_id) === at ? m : new Map(m).set(session.session_id, at)
	);
}

/**
 * Starts the clock for sessions the page learned about from the session list rather than the
 * stream, so a session already silent when the page loads is still flagged. Leaves any session
 * with a recorded message alone.
 */
export function observeSessions(sessionIds: string[], at: number): void {
	sessionLastMessageAt.update((m) => {
		const missing = sessionIds.filter((id) => !m.has(id));
		if (missing.length === 0) return m;
		const next = new Map(m);
		for (const id of missing) next.set(id, at);
		return next;
	});
}

export function forgetSession(sessionId: string): void {
	for (const store of [sessionLastMessageAt, sessionLastActivityAt]) {
		store.update((m) => {
			if (!m.has(sessionId)) return m;
			const next = new Map(m);
			next.delete(sessionId);
			return next;
		});
	}
}

/**
 * A stalled session: how many minutes it has gone quiet, and on what. `messages` means the
 * daemon stopped reporting the session at all. `activity` means it still reports, but the scan
 * stopped advancing.
 */
export interface SessionStall {
	minutes: number;
	source: 'messages' | 'activity';
}

/**
 * Whether a session has stalled, and how. Silence comes first: a daemon that stopped reporting
 * also stopped advancing, and the silence is what to tell the user. A session from a daemon that
 * sends no activity (`lastActivityAt` undefined) is judged on silence alone.
 */
export function sessionStallOf(
	phase: string,
	lastMessageAt: number | undefined,
	lastActivityAt: number | undefined,
	connected: boolean,
	now: number
): SessionStall | null {
	if (isStalled(phase, lastMessageAt, connected, now) && lastMessageAt !== undefined) {
		return { minutes: Math.floor((now - lastMessageAt) / 60_000), source: 'messages' };
	}
	if (
		phase === 'Scanning' &&
		connected &&
		lastActivityAt !== undefined &&
		now - lastActivityAt > ACTIVITY_STALL_THRESHOLD_MS
	) {
		return { minutes: Math.floor((now - lastActivityAt) / 60_000), source: 'activity' };
	}
	return null;
}

/**
 * A stalled session's [`SessionStall`], or `null` while it is not stalled. A store of a function,
 * so a component re-renders as messages arrive and the clock advances.
 */
export const sessionStall = derived(
	[sessionLastMessageAt, sessionLastActivityAt, discoveryStreamConnected, now],
	([lastMessageAt, lastActivityAt, connected, current]) =>
		(sessionId: string, phase: string): SessionStall | null =>
			sessionStallOf(
				phase,
				lastMessageAt.get(sessionId),
				lastActivityAt.get(sessionId),
				connected,
				current
			)
);

/**
 * Whether a session has gone quiet for long enough to tell the user. Only `Scanning` qualifies:
 * `Queued` and `AwaitingSnapshot` wait on other work by design, and `Pending` and `Starting`
 * are brief handoffs the server's own stall sweep covers.
 */
export function isStalled(
	phase: string,
	lastAt: number | undefined,
	connected: boolean,
	now: number
): boolean {
	return (
		phase === 'Scanning' && connected && lastAt !== undefined && now - lastAt > STALL_THRESHOLD_MS
	);
}
