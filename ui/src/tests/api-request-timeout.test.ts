import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('$env/dynamic/public', () => ({
	env: { PUBLIC_SERVER_HOSTNAME: 'scanopy.test', PUBLIC_SERVER_PROTOCOL: 'http' }
}));
vi.mock('$lib/shared/stores/feedback', () => ({ pushError: vi.fn() }));

const { apiClient, RequestTimeoutError } = await import('$lib/api/client');
const { pushError } = await import('$lib/shared/stores/feedback');

/** A server that never answers: the request settles only when its signal aborts. */
function hungFetch() {
	return vi.fn(
		(_input: RequestInfo | URL, init?: RequestInit) =>
			new Promise<Response>((_resolve, reject) => {
				init?.signal?.addEventListener('abort', () =>
					reject(new DOMException('aborted', 'AbortError'))
				);
			})
	);
}

describe('API request timeout', () => {
	beforeEach(() => {
		vi.useFakeTimers();
		vi.mocked(pushError).mockClear();
	});

	afterEach(() => {
		vi.useRealTimers();
		vi.unstubAllGlobals();
	});

	it('rejects a request the server never answers and tells the user', async () => {
		vi.stubGlobal('fetch', hungFetch());

		const request = apiClient.GET('/api/v1/hosts');
		const outcome = expect(request).rejects.toBeInstanceOf(RequestTimeoutError);
		await vi.advanceTimersByTimeAsync(30_000);
		await outcome;

		expect(pushError).toHaveBeenCalledTimes(1);
	});

	it('starts the budget once the request holds a slot, and frees the slot on timeout', async () => {
		const fetch = hungFetch();
		vi.stubGlobal('fetch', fetch);

		// Six requests fill every slot; the seventh waits in the queue.
		const holders = Array.from({ length: 6 }, () => apiClient.GET('/api/v1/subnets'));
		const holderOutcomes = holders.map((r) => r.catch((e: unknown) => e));
		const queued = apiClient.GET('/api/v1/networks');
		let queuedSettled = false;
		const queuedOutcome = queued.then(
			() => (queuedSettled = true),
			(e: unknown) => {
				queuedSettled = true;
				return e;
			}
		);

		await vi.advanceTimersByTimeAsync(0);
		expect(fetch).toHaveBeenCalledTimes(6);

		// The holders time out and release their slots; the queued request reaches the server only
		// now, with a full budget of its own despite having waited 30s.
		await vi.advanceTimersByTimeAsync(30_000);
		for (const outcome of await Promise.all(holderOutcomes)) {
			expect(outcome).toBeInstanceOf(RequestTimeoutError);
		}
		expect(fetch).toHaveBeenCalledTimes(7);
		expect(queuedSettled).toBe(false);

		await vi.advanceTimersByTimeAsync(30_000);
		expect(await queuedOutcome).toBeInstanceOf(RequestTimeoutError);
	});
});
