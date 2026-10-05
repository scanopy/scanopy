/**
 * TanStack Query hooks for Sites
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys, queryClient } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import { requireSuccess, unwrapData } from '$lib/api/query-helpers';
import type { Site } from './types';
import type { User } from '$lib/features/users/types';

/**
 * Query hook for fetching all sites
 */
export function useSitesQuery(options?: {
	enabled?: () => boolean;
	/** Refetch every few seconds while the org has no site, e.g. while a
	 * plan-change webhook is still creating it. */
	pollWhileEmpty?: () => boolean;
}) {
	return createQuery(() => ({
		enabled: options?.enabled?.() ?? true,
		refetchInterval: (query: { state: { data?: unknown[] } }) =>
			options?.pollWhileEmpty?.() && query.state.data?.length === 0 ? 3000 : false,
		queryKey: queryKeys.sites.all,
		queryFn: async () => {
			// Guard: only fetch if user is logged in (check query cache)
			const user = queryClient.getQueryData<User | null>(queryKeys.auth.currentUser());
			if (!user) {
				return [];
			}
			return unwrapData(
				await apiClient.GET('/api/v1/sites', {
					params: { query: { limit: 0 } }
				})
			);
		}
	}));
}

/**
 * Mutation hook for creating a site
 */
export function useCreateSiteMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (site: Site) => {
			return unwrapData(await apiClient.POST('/api/v1/sites', { body: site }));
		},
		onSuccess: (newSite: Site) => {
			queryClient.setQueryData<Site[]>(queryKeys.sites.all, (old) =>
				old ? [...old, newSite] : [newSite]
			);
			// credential_ids changes are reflected on credentials' assigned_site_ids
			queryClient.invalidateQueries({ queryKey: queryKeys.credentials.all });
		}
	}));
}

/**
 * Mutation hook for updating a site
 */
export function useUpdateSiteMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (site: Site) => {
			return unwrapData(
				await apiClient.PUT('/api/v1/sites/{id}', {
					params: { path: { id: site.id } },
					body: site
				})
			);
		},
		onSuccess: (updatedSite: Site) => {
			queryClient.setQueryData<Site[]>(
				queryKeys.sites.all,
				(old) => old?.map((n) => (n.id === updatedSite.id ? updatedSite : n)) ?? []
			);
			// credential_ids changes are reflected on credentials' assigned_site_ids
			queryClient.invalidateQueries({ queryKey: queryKeys.credentials.all });
		}
	}));
}

/**
 * Mutation hook for deleting a site
 */
export function useDeleteSiteMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			requireSuccess(
				await apiClient.DELETE('/api/v1/sites/{id}', {
					params: { path: { id } }
				})
			);
			return id;
		},
		onSuccess: (id: string) => {
			queryClient.setQueryData<Site[]>(
				queryKeys.sites.all,
				(old) => old?.filter((n) => n.id !== id) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for bulk deleting sites
 */
export function useBulkDeleteSitesMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (ids: string[]) => {
			requireSuccess(await apiClient.POST('/api/v1/sites/bulk-delete', { body: ids }));
			return ids;
		},
		onSuccess: (ids: string[]) => {
			queryClient.setQueryData<Site[]>(
				queryKeys.sites.all,
				(old) => old?.filter((n) => !ids.includes(n.id)) ?? []
			);
		}
	}));
}

import { utcTimeZoneSentinel, uuidv4Sentinel } from '$lib/shared/utils/formatting';

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Create empty form data for creating a new site
 */
export function createEmptySiteFormData(): Site {
	return {
		id: uuidv4Sentinel,
		name: '',
		created_at: utcTimeZoneSentinel,
		updated_at: utcTimeZoneSentinel,
		organization_id: uuidv4Sentinel,
		tags: [],
		credential_ids: [],
		// null = unset; the server applies its default staleness window.
		stale_after_hours: null
	};
}
