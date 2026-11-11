import { describe, expect, it, vi } from 'vitest';
import { QueryObserver, type QueryObserverResult } from '@tanstack/svelte-query';

vi.mock('$app/env/public', () => ({
	PUBLIC_SERVER_HOSTNAME: 'scanopy.test',
	PUBLIC_SERVER_PROTOCOL: 'http',
	PUBLIC_SERVER_PORT: ''
}));
vi.mock('#lib/shared/stores/feedback.ts', () => ({ pushError: vi.fn(), pushSuccess: vi.fn() }));

const { createQueryClient } = await import('#lib/api/query-client.ts');

/**
 * A query's result as a component holds it under svelte-query: the result taken when the query
 * was created, replaced only when the observer notifies its listener (`createBaseQuery`).
 */
function componentQuery(queryKey: string[], queryFn: () => Promise<unknown>) {
	const client = createQueryClient();
	const options = client.defaultQueryOptions({ queryKey, queryFn });
	const observer = new QueryObserver(client, options);
	let held: QueryObserverResult = observer.trackResult(observer.getOptimisticResult(options));
	observer.subscribe(() => {
		held = observer.trackResult(observer.getOptimisticResult(options));
	});
	return { client, read: () => held };
}

describe('a query property a component first reads after the query settled', () => {
	// DaemonTab, hidden at app load: its loading check stops at the page query, so it reads the
	// sites query's `isPending` only once the tab opens. Before then it reads `isError` alone, for
	// the load-failure check. The sites query settles meanwhile.
	it('reads the settled value', async () => {
		const { client, read } = componentQuery(['sites'], async () => [{ id: 'site-1' }]);

		expect(read().isError).toBe(false);
		await vi.waitFor(() => expect(client.getQueryState(['sites'])?.status).toBe('success'));

		expect(read().isPending).toBe(false);
	});
});
