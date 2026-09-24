import { derived, readable, writable } from 'svelte/store';

/**
 * How long a scanning session may go without a message before the UI says so. A live daemon
 * reports at least every 30s (its heartbeat in DaemonPoll, the server's poll in ServerPoll), so
 * this is three missed reports.
 */
export const STALL_THRESHOLD_MS = 90_000;

/**
 * When each active session's last stream message arrived, by session id. Keyed on arrival rather
 * than on a change in progress: a healthy scan can hold one progress value for minutes while it
 * works through a large subnet, and still reports every 30s.
 */
export const sessionLastMessageAt = writable<Map<string, number>>(new Map());

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
	sessionLastMessageAt.update((m) => {
		if (!m.has(sessionId)) return m;
		const next = new Map(m);
		next.delete(sessionId);
		return next;
	});
}

/**
 * Minutes since a stalled session's last message, or `null` while it is not stalled. A store of a
 * function, so a component re-renders as messages arrive and the clock advances.
 */
export const sessionStallMinutes = derived(
	[sessionLastMessageAt, discoveryStreamConnected, now],
	([lastMessageAt, connected, current]) =>
		(sessionId: string, phase: string): number | null => {
			const lastAt = lastMessageAt.get(sessionId);
			if (!isStalled(phase, lastAt, connected, current) || lastAt === undefined) return null;
			return Math.floor((current - lastAt) / 60_000);
		}
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
