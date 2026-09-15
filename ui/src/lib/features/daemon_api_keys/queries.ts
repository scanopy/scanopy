/**
 * TanStack Query hooks for Daemon API Keys
 * These are API keys used by daemons to authenticate with the server
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import type { ApiKey } from './types/base';
import { utcTimeZoneSentinel, uuidv4Sentinel } from '$lib/shared/utils/formatting';

/**
 * Query hook for fetching all daemon API keys
 */
export function useApiKeysQuery(options?: { enabled?: () => boolean }) {
	return createQuery(() => ({
		queryKey: queryKeys.apiKeys.all,
		queryFn: async () => {
			const { data } = await apiClient.GET('/api/v1/auth/daemon', {
				params: { query: { limit: 0 } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch API keys');
			}
			return data.data;
		},
		enabled: options?.enabled?.() ?? true
	}));
}

/**
 * Response type from create API key endpoint
 */
interface CreateApiKeyResponse {
	key: string;
	api_key: ApiKey;
}

/**
 * Mutation hook for creating a daemon API key
 * Returns the key string (only shown once) and the created API key
 */
export function useCreateApiKeyMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (apiKey: ApiKey) => {
			const { data } = await apiClient.POST('/api/v1/auth/daemon', { body: apiKey });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to create API key');
			}
			// Response contains both the key string and the created api_key object
			const response = data.data as CreateApiKeyResponse;
			return { keyString: response.key, apiKey: response.api_key };
		},
		onSuccess: ({ apiKey }) => {
			queryClient.setQueryData<ApiKey[]>(queryKeys.apiKeys.all, (old) =>
				old ? [...old, apiKey] : [apiKey]
			);
		}
	}));
}

/**
 * Mutation hook for updating a daemon API key
 */
export function useUpdateApiKeyMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (apiKey: ApiKey) => {
			const { data } = await apiClient.PUT('/api/v1/auth/daemon/{id}', {
				params: { path: { id: apiKey.id } },
				body: apiKey
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to update API key');
			}
			return data.data;
		},
		onSuccess: (updatedKey: ApiKey) => {
			queryClient.setQueryData<ApiKey[]>(
				queryKeys.apiKeys.all,
				(old) => old?.map((k) => (k.id === updatedKey.id ? updatedKey : k)) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for deleting a daemon API key
 */
export function useDeleteApiKeyMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			const { data } = await apiClient.DELETE('/api/v1/auth/daemon/{id}', {
				params: { path: { id } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete API key');
			}
			return id;
		},
		onSuccess: (id: string) => {
			queryClient.setQueryData<ApiKey[]>(
				queryKeys.apiKeys.all,
				(old) => old?.filter((k) => k.id !== id) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for bulk deleting daemon API keys
 */
export function useBulkDeleteApiKeysMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (ids: string[]) => {
			const { data } = await apiClient.POST('/api/v1/auth/daemon/bulk-delete', { body: ids });
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete API keys');
			}
			return ids;
		},
		onSuccess: (ids: string[]) => {
			queryClient.setQueryData<ApiKey[]>(
				queryKeys.apiKeys.all,
				(old) => old?.filter((k) => !ids.includes(k.id)) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for rotating a daemon API key
 */
export function useRotateApiKeyMutation() {
	return createMutation(() => ({
		mutationFn: async (keyId: string) => {
			const { data } = await apiClient.POST('/api/v1/auth/daemon/{id}/rotate', {
				params: { path: { id: keyId } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to rotate API key');
			}
			// Returns the new key string
			return data.data as string;
		}
	}));
}

/**
 * Create empty form data for a new API key
 * @param defaultNetworkId - The network ID to use for the new key
 */
export function createEmptyApiKeyFormData(defaultNetworkId: string): ApiKey {
	return {
		id: uuidv4Sentinel,
		name: '',
		created_at: utcTimeZoneSentinel,
		updated_at: utcTimeZoneSentinel,
		expires_at: null,
		last_used: null,
		network_id: defaultNetworkId,
		key: '',
		is_enabled: true,
		tags: []
	};
}
