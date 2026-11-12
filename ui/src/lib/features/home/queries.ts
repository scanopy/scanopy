/**
 * TanStack Query hooks for Dashboard
 */

import { createQuery } from '@tanstack/svelte-query';
import { queryKeys } from '#lib/api/query-client.ts';
import { apiClient } from '#lib/api/client.ts';
import { unwrapData } from '#lib/api/query-helpers.ts';
import { useListPageQuery, type ListPageParams } from '#lib/api/list-page.ts';
import type { SetupListEntry } from './setup-list';

/**
 * Query hook for fetching the dashboard summary
 * @param options.enabled - Optional getter function to control when query is enabled
 */
export function useDashboardQuery(options?: { enabled?: () => boolean }) {
	return createQuery(() => ({
		queryKey: queryKeys.dashboard.summary(),
		queryFn: async () => {
			return unwrapData(await apiClient.GET('/api/v1/dashboard/summary'));
		},
		enabled: options?.enabled?.() ?? true
	}));
}

/** One page of the Issues the caller sees, grouped by category. `list` carries `site_ids`. */
export function useIssuesListQuery(params: () => ListPageParams) {
	return useListPageQuery<SetupListEntry>(
		queryKeys.dashboard.issues(),
		({ query, querySerializer }) =>
			apiClient.GET('/api/v1/dashboard/issues', { params: { query }, querySerializer }),
		params
	);
}
