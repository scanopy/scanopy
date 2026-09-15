/**
 * TanStack Query hooks for Dashboard
 */

import { createQuery } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';

/**
 * Query hook for fetching the dashboard summary
 * @param options.enabled - Optional getter function to control when query is enabled
 */
export function useDashboardQuery(options?: { enabled?: () => boolean }) {
	return createQuery(() => ({
		queryKey: queryKeys.dashboard.summary(),
		queryFn: async () => {
			const { data } = await apiClient.GET('/api/v1/dashboard/summary');
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch dashboard summary');
			}
			return data.data;
		},
		enabled: options?.enabled?.() ?? true
	}));
}
