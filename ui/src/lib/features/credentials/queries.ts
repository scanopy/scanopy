/**
 * TanStack Query hooks for Credentials
 *
 * Provides query and mutation hooks for managing universal credentials.
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import type { Credential } from './types/base';

/**
 * Query hook for fetching all credentials
 */
export function useCredentialsQuery() {
	return createQuery(() => ({
		queryKey: queryKeys.credentials.all,
		queryFn: async () => {
			const { data } = await apiClient.GET('/api/v1/credentials', {
				params: { query: { limit: 0 } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch credentials');
			}
			return data.data;
		}
	}));
}

/**
 * Query hook for fetching a single credential by ID
 */
export function useCredentialQuery(id: string) {
	return createQuery(() => ({
		queryKey: queryKeys.credentials.detail(id),
		queryFn: async () => {
			const { data } = await apiClient.GET('/api/v1/credentials/{id}', {
				params: { path: { id } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch credential');
			}
			return data.data;
		},
		enabled: !!id
	}));
}

/**
 * Mutation hook for creating a credential
 */
export function useCreateCredentialMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (credential: Credential) => {
			const { data } = await apiClient.POST('/api/v1/credentials', { body: credential });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to create credential');
			}
			return data.data;
		},
		onSuccess: (newCredential: Credential) => {
			queryClient.setQueryData<Credential[]>(queryKeys.credentials.all, (old) =>
				old ? [...old, newCredential] : [newCredential]
			);
		}
	}));
}

/**
 * Mutation hook for updating a credential
 */
export function useUpdateCredentialMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (credential: Credential) => {
			const { data } = await apiClient.PUT('/api/v1/credentials/{id}', {
				params: { path: { id: credential.id } },
				body: credential
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to update credential');
			}
			return data.data;
		},
		onSuccess: (updatedCredential: Credential) => {
			queryClient.setQueryData<Credential[]>(
				queryKeys.credentials.all,
				(old) => old?.map((c) => (c.id === updatedCredential.id ? updatedCredential : c)) ?? []
			);
			queryClient.setQueryData(
				queryKeys.credentials.detail(updatedCredential.id),
				updatedCredential
			);
		}
	}));
}

/**
 * Mutation hook for deleting a credential
 */
export function useDeleteCredentialMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			const { data } = await apiClient.DELETE('/api/v1/credentials/{id}', {
				params: { path: { id } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete credential');
			}
			return id;
		},
		onSuccess: (id: string) => {
			queryClient.setQueryData<Credential[]>(
				queryKeys.credentials.all,
				(old) => old?.filter((c) => c.id !== id) ?? []
			);
			queryClient.removeQueries({ queryKey: queryKeys.credentials.detail(id) });
		}
	}));
}

/**
 * Mutation hook for bulk creating credentials
 */
export function useBulkCreateCredentialsMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (credentials: Credential[]) => {
			const res = await fetch('/api/v1/credentials/bulk', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(credentials),
				credentials: 'include'
			});
			const data = await res.json();
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to bulk create credentials');
			}
			return data.data as Credential[];
		},
		onSuccess: (newCredentials: Credential[]) => {
			queryClient.setQueryData<Credential[]>(queryKeys.credentials.all, (old) =>
				old ? [...old, ...newCredentials] : newCredentials
			);
		}
	}));
}

/**
 * Mutation hook for bulk deleting credentials
 */
export function useBulkDeleteCredentialsMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (ids: string[]) => {
			const { data } = await apiClient.POST('/api/v1/credentials/bulk-delete', { body: ids });
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to bulk delete credentials');
			}
			return ids;
		},
		onSuccess: (ids: string[]) => {
			queryClient.setQueryData<Credential[]>(
				queryKeys.credentials.all,
				(old) => old?.filter((c) => !ids.includes(c.id)) ?? []
			);
			ids.forEach((id) => {
				queryClient.removeQueries({ queryKey: queryKeys.credentials.detail(id) });
			});
		}
	}));
}
