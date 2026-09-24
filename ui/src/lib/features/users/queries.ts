/**
 * TanStack Query hooks for Users
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import { requireSuccess, unwrapData } from '$lib/api/query-helpers';
import type { User } from './types';

/**
 * Query hook for fetching all users
 * @param options.enabled - Whether to enable the query (default: true). Can be a boolean or getter function for reactivity.
 */
export function useUsersQuery(options?: { enabled?: boolean | (() => boolean) }) {
	return createQuery(() => {
		const enabled =
			typeof options?.enabled === 'function' ? options.enabled() : (options?.enabled ?? true);
		return {
			queryKey: queryKeys.users.all,
			queryFn: async () => {
				return unwrapData(
					await apiClient.GET('/api/v1/users', {
						params: { query: { limit: 0 } }
					})
				);
			},
			enabled
		};
	});
}

/**
 * Mutation hook for the current user to update their own record. Hits the
 * existing `PUT /api/v1/users/{id}` self-update path. The backend rejects
 * cross-user writes (`auth_user_id != id`) and silently preserves
 * email/password/OIDC/permission/org fields.
 */
export function useUpdateSelfMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (user: User) => {
			return unwrapData(
				await apiClient.PUT('/api/v1/users/{id}', {
					params: { path: { id: user.id } },
					body: user
				})
			);
		},
		onSuccess: (updatedUser: User) => {
			queryClient.setQueryData(queryKeys.auth.currentUser(), updatedUser);
		}
	}));
}

/**
 * Mutation hook for updating a user as admin
 */
export function useUpdateUserAsAdminMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (user: User) => {
			return unwrapData(
				await apiClient.PUT('/api/v1/users/{id}/admin', {
					params: { path: { id: user.id } },
					body: user
				})
			);
		},
		onSuccess: (updatedUser: User) => {
			queryClient.setQueryData<User[]>(
				queryKeys.users.all,
				(old) => old?.map((u) => (u.id === updatedUser.id ? updatedUser : u)) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for deleting a user
 */
export function useDeleteUserMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			requireSuccess(
				await apiClient.DELETE('/api/v1/users/{id}', {
					params: { path: { id } }
				})
			);
			return id;
		},
		onSuccess: (id: string) => {
			queryClient.setQueryData<User[]>(
				queryKeys.users.all,
				(old) => old?.filter((u) => u.id !== id) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for bulk deleting users
 */
export function useBulkDeleteUsersMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (ids: string[]) => {
			requireSuccess(await apiClient.POST('/api/v1/users/bulk-delete', { body: ids }));
			return ids;
		},
		onSuccess: (ids: string[]) => {
			queryClient.setQueryData<User[]>(
				queryKeys.users.all,
				(old) => old?.filter((u) => !ids.includes(u.id)) ?? []
			);
		}
	}));
}
