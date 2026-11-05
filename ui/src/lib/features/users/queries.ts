/**
 * TanStack Query hooks for Users
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import { requireSuccess, unwrapData } from '$lib/api/query-helpers';
import { mergeCurrentUser } from '$lib/features/auth/queries';
import type { User, UserOrInvite } from './types';
import { useListPageQuery, type ListPageParams } from '$lib/api/list-page';

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
 * Query hook for fetching specific users by id, one `GET /api/v1/users/{id}` each.
 *
 * For naming the users other entities reference (a daemon's maintainer). The users list
 * needs Admin and leaves out peer admins and the owner; the per-id route needs only a user of
 * the same organization, so this works for every role.
 *
 * @param idsGetter - Getter function returning the distinct user ids to fetch
 */
export function useUsersByIds(idsGetter: () => string[]) {
	return createQuery(() => {
		const ids = idsGetter();
		return {
			queryKey: [...queryKeys.users.all, 'byIds', ids],
			queryFn: async (): Promise<User[]> =>
				Promise.all(
					ids.map(async (id) =>
						unwrapData(await apiClient.GET('/api/v1/users/{id}', { params: { path: { id } } }))
					)
				),
			enabled: ids.length > 0
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
			mergeCurrentUser(queryClient, updatedUser);
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
			queryClient.invalidateQueries({ queryKey: queryKeys.users.members() });
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
			queryClient.invalidateQueries({ queryKey: queryKeys.users.members() });
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
			queryClient.invalidateQueries({ queryKey: queryKeys.users.members() });
			queryClient.setQueryData<User[]>(
				queryKeys.users.all,
				(old) => old?.filter((u) => !ids.includes(u.id)) ?? []
			);
		}
	}));
}

/** Every user and pending invite the caller may see, for counts and filter options. */
export function useMembersQuery() {
	return createQuery(() => ({
		queryKey: [...queryKeys.users.members(), 'all'],
		queryFn: async () =>
			unwrapData(await apiClient.GET('/api/v1/users/members', { params: { query: { limit: 0 } } }))
	}));
}

/** One page of the members list: users and pending invites, paged on the server. */
export function useMembersListQuery(params: () => ListPageParams) {
	return useListPageQuery<UserOrInvite>(
		queryKeys.users.members(),
		({ query, querySerializer }) =>
			apiClient.GET('/api/v1/users/members', { params: { query }, querySerializer }),
		params
	);
}
