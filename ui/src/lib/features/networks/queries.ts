/**
 * TanStack Query hooks for Networks
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys, queryClient } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import { requireSuccess, unwrapData } from '$lib/api/query-helpers';
import type { Network } from './types';
import type { User } from '$lib/features/users/types';

/**
 * Query hook for fetching all networks
 */
export function useNetworksQuery() {
	return createQuery(() => ({
		queryKey: queryKeys.networks.all,
		queryFn: async () => {
			// Guard: only fetch if user is logged in (check query cache)
			const user = queryClient.getQueryData<User | null>(queryKeys.auth.currentUser());
			if (!user) {
				return [];
			}
			return unwrapData(
				await apiClient.GET('/api/v1/networks', {
					params: { query: { limit: 0 } }
				})
			);
		}
	}));
}

/**
 * Mutation hook for creating a network
 */
export function useCreateNetworkMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (network: Network) => {
			return unwrapData(await apiClient.POST('/api/v1/networks', { body: network }));
		},
		onSuccess: (newNetwork: Network) => {
			queryClient.setQueryData<Network[]>(queryKeys.networks.all, (old) =>
				old ? [...old, newNetwork] : [newNetwork]
			);
			// credential_ids changes are reflected on credentials' assigned_network_ids
			queryClient.invalidateQueries({ queryKey: queryKeys.credentials.all });
		}
	}));
}

/**
 * Mutation hook for updating a network
 */
export function useUpdateNetworkMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (network: Network) => {
			return unwrapData(
				await apiClient.PUT('/api/v1/networks/{id}', {
					params: { path: { id: network.id } },
					body: network
				})
			);
		},
		onSuccess: (updatedNetwork: Network) => {
			queryClient.setQueryData<Network[]>(
				queryKeys.networks.all,
				(old) => old?.map((n) => (n.id === updatedNetwork.id ? updatedNetwork : n)) ?? []
			);
			// credential_ids changes are reflected on credentials' assigned_network_ids
			queryClient.invalidateQueries({ queryKey: queryKeys.credentials.all });
		}
	}));
}

/**
 * Mutation hook for deleting a network
 */
export function useDeleteNetworkMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			requireSuccess(
				await apiClient.DELETE('/api/v1/networks/{id}', {
					params: { path: { id } }
				})
			);
			return id;
		},
		onSuccess: (id: string) => {
			queryClient.setQueryData<Network[]>(
				queryKeys.networks.all,
				(old) => old?.filter((n) => n.id !== id) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for bulk deleting networks
 */
export function useBulkDeleteNetworksMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (ids: string[]) => {
			requireSuccess(await apiClient.POST('/api/v1/networks/bulk-delete', { body: ids }));
			return ids;
		},
		onSuccess: (ids: string[]) => {
			queryClient.setQueryData<Network[]>(
				queryKeys.networks.all,
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
 * Create empty form data for creating a new network
 */
export function createEmptyNetworkFormData(): Network {
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
