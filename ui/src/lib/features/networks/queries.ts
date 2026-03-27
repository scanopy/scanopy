/**
 * TanStack Query hooks for Networks
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys, queryClient } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
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
			const { data } = await apiClient.GET('/api/v1/networks', {
				params: { query: { limit: 0 } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch networks');
			}
			return data.data;
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
			const { data } = await apiClient.POST('/api/v1/networks', { body: network });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to create network');
			}
			return data.data;
		},
		onSuccess: (newNetwork: Network) => {
			queryClient.setQueryData<Network[]>(queryKeys.networks.all, (old) =>
				old ? [...old, newNetwork] : [newNetwork]
			);
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
			const { data } = await apiClient.PUT('/api/v1/networks/{id}', {
				params: { path: { id: network.id } },
				body: network
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to update network');
			}
			return data.data;
		},
		onSuccess: (updatedNetwork: Network) => {
			queryClient.setQueryData<Network[]>(
				queryKeys.networks.all,
				(old) => old?.map((n) => (n.id === updatedNetwork.id ? updatedNetwork : n)) ?? []
			);
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
			const { data } = await apiClient.DELETE('/api/v1/networks/{id}', {
				params: { path: { id } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete network');
			}
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
			const { data } = await apiClient.POST('/api/v1/networks/bulk-delete', { body: ids });
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete networks');
			}
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
		credential_ids: []
	};
}
