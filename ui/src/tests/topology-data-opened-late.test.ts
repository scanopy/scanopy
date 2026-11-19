import { describe, expect, it, vi } from 'vitest';
import { QueryObserver } from '@tanstack/svelte-query';
import { apiClient } from '#lib/api/client.ts';
import { createQueryClient } from '#lib/api/query-client.ts';
import { topologyDataObserverOptions } from '#lib/features/topology/queries.ts';

describe('the topology data query opened after its site and view settled', () => {
	// Home first, then Topology: the site and the starting view are applied while the tab is
	// hidden, so opening it changes nothing but the gate. The same holds for a `?view=` link.
	it('requests the bundle when the tab opens', async () => {
		const get = vi
			.spyOn(apiClient, 'GET')
			.mockResolvedValue({ data: { success: true, data: {} } } as never);
		const client = createQueryClient();
		const options = (open: boolean) =>
			client.defaultQueryOptions(
				topologyDataObserverOptions('site-1', undefined, 'L2Physical', open)
			);

		const observer = new QueryObserver(client, options(false));
		const unsubscribe = observer.subscribe(() => {});
		expect(get).not.toHaveBeenCalled();

		observer.setOptions(options(true));

		await vi.waitFor(() => expect(get).toHaveBeenCalledTimes(1));
		unsubscribe();
		get.mockRestore();
	});
});
