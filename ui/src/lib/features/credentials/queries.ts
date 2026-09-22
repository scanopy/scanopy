/**
 * TanStack Query hooks for Credentials
 *
 * Provides query and mutation hooks for managing universal credentials.
 */

import {
	createQuery,
	createMutation,
	useQueryClient,
	type QueryClient
} from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import { requireSuccess, unwrapData } from '$lib/api/query-helpers';
import type { Credential } from './types/base';

/**
 * A credential's assignments live in the network/host junction tables, so any
 * credential mutation can change `Network.credential_ids` / `Host.credential_assignments`.
 * Invalidate those caches so the Networks/Hosts views reflect changes without a reload.
 */
function invalidateAssignmentTargets(queryClient: QueryClient): void {
	queryClient.invalidateQueries({ queryKey: queryKeys.networks.all });
	queryClient.invalidateQueries({ queryKey: queryKeys.hosts.all });
}

/**
 * Query hook for fetching all credentials
 */
export function useCredentialsQuery() {
	return createQuery(() => ({
		queryKey: queryKeys.credentials.all,
		queryFn: async () => {
			return unwrapData(
				await apiClient.GET('/api/v1/credentials', {
					params: { query: { limit: 0 } }
				})
			);
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
			return unwrapData(
				await apiClient.GET('/api/v1/credentials/{id}', {
					params: { path: { id } }
				})
			);
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
			return unwrapData(await apiClient.POST('/api/v1/credentials', { body: credential }));
		},
		onSuccess: (newCredential: Credential) => {
			queryClient.setQueryData<Credential[]>(queryKeys.credentials.all, (old) =>
				old ? [...old, newCredential] : [newCredential]
			);
			// Assignments are written server-side to the network/host junctions
			invalidateAssignmentTargets(queryClient);
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
			return unwrapData(
				await apiClient.PUT('/api/v1/credentials/{id}', {
					params: { path: { id: credential.id } },
					body: credential
				})
			);
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
			invalidateAssignmentTargets(queryClient);
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
			requireSuccess(
				await apiClient.DELETE('/api/v1/credentials/{id}', {
					params: { path: { id } }
				})
			);
			return id;
		},
		onSuccess: (id: string) => {
			queryClient.setQueryData<Credential[]>(
				queryKeys.credentials.all,
				(old) => old?.filter((c) => c.id !== id) ?? []
			);
			queryClient.removeQueries({ queryKey: queryKeys.credentials.detail(id) });
			invalidateAssignmentTargets(queryClient);
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
			return unwrapData(await apiClient.POST('/api/v1/credentials/bulk', { body: credentials }));
		},
		onSuccess: (newCredentials: Credential[]) => {
			queryClient.setQueryData<Credential[]>(queryKeys.credentials.all, (old) =>
				old ? [...old, ...newCredentials] : newCredentials
			);
			invalidateAssignmentTargets(queryClient);
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
			requireSuccess(await apiClient.POST('/api/v1/credentials/bulk-delete', { body: ids }));
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
			invalidateAssignmentTargets(queryClient);
		}
	}));
}
