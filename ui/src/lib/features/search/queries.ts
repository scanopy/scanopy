/**
 * The lookups behind the global search palette: one request to the shared search endpoint, which
 * asks every entity type's service for its first page of matches, and a request per "Show more"
 * for the next page of one type.
 */

import { createQuery, keepPreviousData } from '@tanstack/svelte-query';
import type { EntityDiscriminants } from '#lib/api/entities.ts';
import { queryKeys } from '#lib/api/query-client.ts';
import { apiClient } from '#lib/api/client.ts';
import { unwrapData } from '#lib/api/query-helpers.ts';
import {
	globalSearchQuery,
	hasSearchTerms,
	responsePage,
	type GlobalSearchState,
	type SearchItem
} from './results';

/** Matches each "Show more" adds to a section. */
export const GLOBAL_SEARCH_MORE_PAGE = 20;

export function useGlobalSearch(searchGetter: () => GlobalSearchState) {
	return createQuery(() => {
		const search = searchGetter();
		const query = globalSearchQuery(search);
		return {
			queryKey: queryKeys.globalSearch.results(query.q ?? '', query.tag_ids),
			queryFn: async () => unwrapData(await apiClient.GET('/api/v1/search', { params: { query } })),
			enabled: hasSearchTerms(search),
			// Matches come from every entity type, so no single entity's invalidation reaches this
			// key: refetch on each search instead of serving a cached answer from before an edit.
			staleTime: 0,
			placeholderData: keepPreviousData
		};
	});
}

/** The next page of one type's matches, starting after the `offset` already shown. */
export async function fetchMoreMatches(
	search: GlobalSearchState,
	entityType: EntityDiscriminants,
	offset: number
): Promise<SearchItem[]> {
	const response = unwrapData(
		await apiClient.GET('/api/v1/search', {
			params: {
				query: {
					...globalSearchQuery(search),
					entity_type: entityType,
					offset,
					limit: GLOBAL_SEARCH_MORE_PAGE
				}
			}
		})
	);
	return responsePage(response);
}
