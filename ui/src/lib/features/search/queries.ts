/**
 * The lookup behind the global search palette: one request to the shared search endpoint, which
 * asks every entity type's service and returns a handful of matches of each.
 */

import { createQuery, keepPreviousData } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import { unwrapData } from '$lib/api/query-helpers';
import { hasSearchTerms, type GlobalSearchState } from './results';

export function useGlobalSearch(searchGetter: () => GlobalSearchState) {
	return createQuery(() => {
		const { text, tagIds } = searchGetter();
		const q = text.trim();
		return {
			queryKey: queryKeys.globalSearch.results(q, tagIds),
			queryFn: async () =>
				unwrapData(
					await apiClient.GET('/api/v1/search', {
						params: { query: { q: q || undefined, tag_ids: tagIds } }
					})
				),
			enabled: hasSearchTerms({ text: q, tagIds }),
			// Matches come from every entity type, so no single entity's invalidation reaches this
			// key: refetch on each search instead of serving a cached answer from before an edit.
			staleTime: 0,
			placeholderData: keepPreviousData
		};
	});
}
