/**
 * TanStack Query hooks for Tags
 *
 * Replaces the writable store pattern with query-based data fetching.
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import { requireSuccess, unwrapData } from '$lib/api/query-helpers';
import type { Tag } from './types/base';
import type { components } from '$lib/api/schema';

// Types for tag assignment operations
export type EntityDiscriminants = components['schemas']['EntityDiscriminants'];
export type BulkTagRequest = components['schemas']['BulkTagRequest'];
export type SetTagsRequest = components['schemas']['SetTagsRequest'];

/**
 * Query hook for fetching all tags
 */
export function useTagsQuery(atGetter?: () => string | undefined) {
	return createQuery(() => {
		const at = atGetter?.();
		return {
			queryKey: at ? [...queryKeys.tags.all, 'asOf', at] : queryKeys.tags.all,
			queryFn: async () => {
				return unwrapData(
					await apiClient.GET('/api/v1/tags', {
						params: { query: { limit: 0, at } }
					})
				);
			}
		};
	});
}

/**
 * Mutation hook for creating a tag
 */
export function useCreateTagMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (tag: Tag) => {
			return unwrapData(await apiClient.POST('/api/v1/tags', { body: tag }));
		},
		onSuccess: (newTag: Tag) => {
			queryClient.setQueryData<Tag[]>(queryKeys.tags.all, (old) =>
				old ? [...old, newTag] : [newTag]
			);
		}
	}));
}

/**
 * Mutation hook for updating a tag
 */
export function useUpdateTagMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (tag: Tag) => {
			return unwrapData(
				await apiClient.PUT('/api/v1/tags/{id}', {
					params: { path: { id: tag.id } },
					body: tag
				})
			);
		},
		onSuccess: (updatedTag: Tag) => {
			queryClient.setQueryData<Tag[]>(
				queryKeys.tags.all,
				(old) => old?.map((t) => (t.id === updatedTag.id ? updatedTag : t)) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for deleting a single tag
 */
export function useDeleteTagMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			requireSuccess(
				await apiClient.DELETE('/api/v1/tags/{id}', {
					params: { path: { id } }
				})
			);
			return id;
		},
		onSuccess: (id: string) => {
			queryClient.setQueryData<Tag[]>(
				queryKeys.tags.all,
				(old) => old?.filter((t) => t.id !== id) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for bulk deleting tags
 */
export function useBulkDeleteTagsMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (ids: string[]) => {
			requireSuccess(await apiClient.POST('/api/v1/tags/bulk-delete', { body: ids }));
			return ids;
		},
		onSuccess: (ids: string[]) => {
			queryClient.setQueryData<Tag[]>(
				queryKeys.tags.all,
				(old) => old?.filter((t) => !ids.includes(t.id)) ?? []
			);
		}
	}));
}

// ============================================================================
// Entity Tag Assignment Mutations
// ============================================================================

/**
 * Map entity types to their query key names for cache invalidation.
 * Using full Record ensures TypeScript errors if a new EntityDiscriminants is added
 * without updating this mapping.
 */
const entityTypeToQueryKeyName: Record<EntityDiscriminants, keyof typeof queryKeys | null> = {
	// Taggable entities
	Host: 'hosts',
	Service: 'services',
	Subnet: 'subnets',
	Dependency: 'dependencies',
	Network: 'networks',
	Discovery: 'discovery',
	Daemon: 'daemons',
	DaemonApiKey: 'apiKeys',
	UserApiKey: 'userApiKeys',
	Credential: 'credentials',
	// Non-taggable entities (null = no cache invalidation needed)
	Organization: null,
	Invite: null,
	Share: null,
	User: null,
	Tag: null,
	Port: null,
	Binding: null,
	IPAddress: null,
	Interface: null,
	Vlan: null,
	Topology: null,
	Snapshot: null,
	Unknown: null
};

/**
 * Get the query key for an entity type, or undefined if not taggable.
 * Returns the list query key pattern (e.g., ['hosts', 'list']) to ensure
 * invalidation matches paginated list queries.
 */
function getQueryKeyForEntityType(entityType: EntityDiscriminants): readonly string[] | undefined {
	const keyName = entityTypeToQueryKeyName[entityType];
	if (keyName) {
		const queryKeyGroup = queryKeys[keyName];
		// Use 'lists' key if available (for paginated queries), otherwise fall back to 'all'
		if ('lists' in queryKeyGroup && typeof queryKeyGroup.lists === 'function') {
			return queryKeyGroup.lists();
		}
		return queryKeyGroup.all;
	}
	return undefined;
}

/**
 * List of taggable entity types (derived from the mapping above)
 */
export const TAGGABLE_ENTITY_TYPES = (
	Object.keys(entityTypeToQueryKeyName) as EntityDiscriminants[]
).filter((key) => entityTypeToQueryKeyName[key] !== null);

/**
 * Mutation hook for bulk adding a tag to multiple entities
 */
export function useBulkAddTagMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (request: BulkTagRequest) => {
			const assignResult = unwrapData(
				await apiClient.POST('/api/v1/tags/assign/bulk-add', { body: request })
			);
			return {
				...assignResult,
				entityType: request.entity_type,
				tagId: request.tag_id,
				entityIds: request.entity_ids
			};
		},
		onSuccess: ({ entityType }) => {
			const queryKey = getQueryKeyForEntityType(entityType);
			if (queryKey) {
				queryClient.invalidateQueries({ queryKey });
			}
		}
	}));
}

/**
 * Mutation hook for bulk removing a tag from multiple entities
 */
export function useBulkRemoveTagMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (request: BulkTagRequest) => {
			const removeResult = unwrapData(
				await apiClient.POST('/api/v1/tags/assign/bulk-remove', { body: request })
			);
			return {
				...removeResult,
				entityType: request.entity_type,
				tagId: request.tag_id,
				entityIds: request.entity_ids
			};
		},
		onSuccess: ({ entityType }) => {
			const queryKey = getQueryKeyForEntityType(entityType);
			if (queryKey) {
				queryClient.invalidateQueries({ queryKey });
			}
		}
	}));
}

/**
 * Mutation hook for setting all tags on an entity
 */
export function useSetEntityTagsMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (request: SetTagsRequest) => {
			requireSuccess(await apiClient.PUT('/api/v1/tags/assign', { body: request }));
			return { entityType: request.entity_type };
		},
		onSuccess: ({ entityType }) => {
			const queryKey = getQueryKeyForEntityType(entityType);
			if (queryKey) {
				queryClient.invalidateQueries({ queryKey });
			}
		}
	}));
}
