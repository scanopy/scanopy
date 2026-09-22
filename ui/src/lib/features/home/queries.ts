/**
 * TanStack Query hooks for Dashboard
 */

import { createQuery } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import { unwrapData } from '$lib/api/query-helpers';

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
