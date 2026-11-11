import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { QueryClient } from '@tanstack/svelte-query';

vi.mock('$app/env/public', () => ({
	PUBLIC_SERVER_HOSTNAME: 'scanopy.test',
	PUBLIC_SERVER_PROTOCOL: 'http',
	PUBLIC_SERVER_PORT: ''
}));
vi.mock('#lib/shared/stores/feedback.ts', () => ({ pushError: vi.fn(), pushSuccess: vi.fn() }));
vi.mock('#lib/api/server-version.ts', () => ({ checkServerVersion: vi.fn() }));

const { fetchCurrentUser } = await import('#lib/features/auth/queries.ts');
const { checkServerVersion } = await import('#lib/api/server-version.ts');

const USER = { id: 'u1', email: 'owner@example.com', permissions: 'Owner' };

function json(status: number, body: unknown): Response {
	return new Response(JSON.stringify(body), {
		status,
		headers: { 'Content-Type': 'application/json' }
	});
}

function envelope(data: unknown, serverVersion = '0.19.3') {
	return {
		success: true,
		data,
		error: null,
		meta: { api_version: 1, server_version: serverVersion }
	};
}

describe('fetchCurrentUser', () => {
	beforeEach(() => {
		// The reachability probe arms a timer on an outage; keep it from firing between tests.
		vi.useFakeTimers();
		vi.mocked(checkServerVersion).mockClear();
	});

	afterEach(() => {
		vi.useRealTimers();
		vi.unstubAllGlobals();
	});

	it('reads a 401 as signed out', async () => {
		vi.stubGlobal(
			'fetch',
			vi.fn(async () => json(401, { success: false, error: 'Not authenticated' }))
		);
		await expect(fetchCurrentUser()).resolves.toBeNull();
	});

	it('returns the user and hands the server version to the version check', async () => {
		vi.stubGlobal(
			'fetch',
			vi.fn(async () => json(200, envelope(USER, '0.20.0')))
		);
		await expect(fetchCurrentUser()).resolves.toMatchObject({ id: 'u1' });
		expect(checkServerVersion).toHaveBeenCalledWith('0.20.0');
	});

	it.each([
		['a gateway error', () => new Response('Bad Gateway', { status: 502 })],
		['a proxy 500 with no server body', () => new Response('', { status: 500 })],
		['a server error', () => json(500, { success: false, error: 'boom' })]
	])('rejects on %s instead of reporting no user', async (_name, response) => {
		vi.stubGlobal(
			'fetch',
			vi.fn(async () => response())
		);
		await expect(fetchCurrentUser()).rejects.toThrow();
	});

	it('rejects when the server cannot be reached', async () => {
		vi.stubGlobal(
			'fetch',
			vi.fn(async () => {
				throw new TypeError('Failed to fetch');
			})
		);
		await expect(fetchCurrentUser()).rejects.toThrow();
	});

	it('keeps a signed-in user through a refetch the server does not answer', async () => {
		const client = new QueryClient();
		const queryKey = ['auth', 'currentUser'];
		client.setQueryData(queryKey, USER);
		vi.stubGlobal(
			'fetch',
			vi.fn(async () => new Response('', { status: 502 }))
		);

		await expect(
			client.fetchQuery({ queryKey, queryFn: fetchCurrentUser, retry: false, staleTime: 0 })
		).rejects.toThrow();

		expect(client.getQueryData(queryKey)).toEqual(USER);
	});
});
