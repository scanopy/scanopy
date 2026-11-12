import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';

vi.mock('$app/env/public', () => ({
	PUBLIC_SERVER_HOSTNAME: 'scanopy.test',
	PUBLIC_SERVER_PROTOCOL: 'http',
	PUBLIC_SERVER_PORT: ''
}));
vi.mock('#lib/shared/stores/feedback.ts', () => ({ pushError: vi.fn() }));
const invalidateQueries = vi.fn();
vi.mock('#lib/api/query-client.ts', () => ({ queryClient: { invalidateQueries } }));

const { apiClient } = await import('#lib/api/client.ts');
const { pushError } = await import('#lib/shared/stores/feedback.ts');
const { PROBE_INTERVAL_MS, isServerUnreachable, isUnreachableResponse, serverUnreachable } =
	await import('#lib/api/server-reachability.ts');

function json(status: number, body: unknown): Response {
	return new Response(JSON.stringify(body), {
		status,
		headers: { 'Content-Type': 'application/json' }
	});
}

const ok = () => json(200, { success: true, data: [], error: null, meta: {} });
const proxyDown = () => new Response('', { status: 500 });

/** Lets the lazily imported query client resolve. */
async function settle() {
	await vi.advanceTimersByTimeAsync(0);
}

/** Bring the module back to "reachable" between tests through its public path. */
async function recover() {
	vi.stubGlobal(
		'fetch',
		vi.fn(async () => ok())
	);
	await apiClient.GET('/api/v1/hosts');
	await settle();
}

describe('isUnreachableResponse', () => {
	it('separates a down server from an error the server sent', () => {
		expect(isUnreachableResponse(new Response('', { status: 502 }))).toBe(true);
		expect(isUnreachableResponse(new Response('<html>', { status: 503 }))).toBe(true);
		expect(isUnreachableResponse(proxyDown())).toBe(true);
		expect(isUnreachableResponse(json(500, { success: false, error: 'boom' }))).toBe(false);
		expect(isUnreachableResponse(json(404, { success: false, error: 'missing' }))).toBe(false);
		expect(isUnreachableResponse(ok())).toBe(false);
	});
});

describe('server reachability', () => {
	beforeEach(async () => {
		vi.useFakeTimers();
		await recover();
		vi.mocked(pushError).mockClear();
		invalidateQueries.mockClear();
	});

	afterEach(() => {
		vi.useRealTimers();
		vi.unstubAllGlobals();
	});

	it('reports an outage through the banner state, not a toast per failed request', async () => {
		vi.stubGlobal(
			'fetch',
			vi.fn(async () => proxyDown())
		);
		await apiClient.GET('/api/v1/hosts');
		await apiClient.GET('/api/v1/subnets');

		expect(get(serverUnreachable)).toBe(true);
		expect(pushError).not.toHaveBeenCalled();

		await recover();
		expect(get(serverUnreachable)).toBe(false);
	});

	it('probes until the server answers, then refetches every query once', async () => {
		const fetch = vi.fn(async (input: RequestInfo | URL) => {
			const url = input instanceof Request ? input.url : String(input);
			if (url.endsWith('/api/health/ready')) return new Response('', { status: 503 });
			throw new TypeError('Failed to fetch');
		});
		vi.stubGlobal('fetch', fetch);
		await apiClient.GET('/api/v1/hosts').catch(() => undefined);
		expect(isServerUnreachable()).toBe(true);

		// Still down (the database isn't ready): the probe keeps going.
		await vi.advanceTimersByTimeAsync(PROBE_INTERVAL_MS * 2);
		expect(invalidateQueries).not.toHaveBeenCalled();

		fetch.mockImplementation(async () => ok());
		await vi.advanceTimersByTimeAsync(PROBE_INTERVAL_MS);
		await settle();

		expect(isServerUnreachable()).toBe(false);
		expect(invalidateQueries).toHaveBeenCalledTimes(1);

		// Answers after recovery don't refetch again.
		await apiClient.GET('/api/v1/hosts');
		await settle();
		expect(invalidateQueries).toHaveBeenCalledTimes(1);
	});

	it('recovers on the first answer from any request, before the probe fires', async () => {
		vi.stubGlobal(
			'fetch',
			vi.fn(async () => new Response('', { status: 502 }))
		);
		await apiClient.GET('/api/v1/hosts');

		vi.stubGlobal(
			'fetch',
			vi.fn(async () => ok())
		);
		await apiClient.GET('/api/v1/hosts');
		await settle();

		expect(invalidateQueries).toHaveBeenCalledTimes(1);
		// The probe was stopped: nothing more happens when its interval passes.
		await vi.advanceTimersByTimeAsync(PROBE_INTERVAL_MS);
		await settle();
		expect(invalidateQueries).toHaveBeenCalledTimes(1);
	});

	it('does not treat an error the server sent as an outage', async () => {
		vi.stubGlobal(
			'fetch',
			vi.fn(async () => json(500, { success: false, error: 'boom' }))
		);
		await apiClient.GET('/api/v1/hosts');

		expect(isServerUnreachable()).toBe(false);
		await vi.advanceTimersByTimeAsync(PROBE_INTERVAL_MS);
		expect(invalidateQueries).not.toHaveBeenCalled();
	});
});
