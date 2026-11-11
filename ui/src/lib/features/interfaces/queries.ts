import { createQuery, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '#lib/api/query-client.ts';
import { apiClient } from '#lib/api/client.ts';
import { unwrapData } from '#lib/api/query-helpers.ts';
import type { components } from '#lib/api/schema';

export type Interface = components['schemas']['Interface'];

export function useInterfacesQuery() {
	const queryClient = useQueryClient();
	return createQuery(() => ({
		queryKey: queryKeys.interfaces.all,
		queryFn: () => {
			// Interfaces are populated by hosts query - read from cache
			return queryClient.getQueryData<Interface[]>(queryKeys.interfaces.all) ?? [];
		}
	}));
}

/**
 * Query hook for fetching specific interfaces by IDs (for selective loading).
 *
 * For interfaces that belong to a host other than the ones loaded, such as the interface a
 * virtualizing host presents a network identity from.
 *
 * @param idsGetter - Getter function returning the distinct interface ids to fetch
 */
export function useInterfacesByIds(idsGetter: () => string[]) {
	return createQuery(() => {
		const ids = idsGetter();
		return {
			queryKey: [...queryKeys.interfaces.all, 'byIds', ids],
			queryFn: async (): Promise<Interface[]> =>
				unwrapData(
					await apiClient.GET('/api/v1/interfaces', { params: { query: { ids, limit: 0 } } })
				),
			enabled: ids.length > 0
		};
	});
}

export type InterfaceNeighbours = components['schemas']['InterfaceNeighbours'];

/**
 * One interface's resolved adjacencies and the raw LLDP/CDP evidence behind them.
 *
 * @param interfaceId - Getter for the interface to read
 */
export function useInterfaceNeighboursQuery(interfaceId: () => string) {
	return createQuery(() => {
		const id = interfaceId();
		return {
			queryKey: [...queryKeys.interfaces.all, 'neighbours', id],
			queryFn: async (): Promise<InterfaceNeighbours> =>
				unwrapData(
					await apiClient.GET('/api/v1/interfaces/{id}/neighbours', { params: { path: { id } } })
				)
		};
	});
}
