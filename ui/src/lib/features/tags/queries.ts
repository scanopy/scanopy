/**
 * TanStack Query hooks for Tags
 *
 * Replaces the writable store pattern with query-based data fetching.
 */

import {
	createQuery,
	createMutation,
	useQueryClient,
	type QueryClient
} from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import type { Tag } from './types/base';
import type { components } from '$lib/api/schema';
import type { Topology } from '$lib/features/topology/types/base';

// Types for tag assignment operations
export type EntityDiscriminants = components['schemas']['EntityDiscriminants'];
export type BulkTagRequest = components['schemas']['BulkTagRequest'];
export type SetTagsRequest = components['schemas']['SetTagsRequest'];

/**
 * Query hook for fetching all tags
 */
export function useTagsQuery() {
	return createQuery(() => ({
		queryKey: queryKeys.tags.all,
		queryFn: async () => {
			const { data } = await apiClient.GET('/api/v1/tags', {
				params: { query: { limit: 0 } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch tags');
			}
			return data.data;
		}
	}));
}

/**
 * Mutation hook for creating a tag
 */
export function useCreateTagMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (tag: Tag) => {
			const { data } = await apiClient.POST('/api/v1/tags', { body: tag });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to create tag');
			}
			return data.data;
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
			const { data } = await apiClient.PUT('/api/v1/tags/{id}', {
				params: { path: { id: tag.id } },
				body: tag
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to update tag');
			}
			return data.data;
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
			const { data } = await apiClient.DELETE('/api/v1/tags/{id}', {
				params: { path: { id } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete tag');
			}
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
			const { data } = await apiClient.POST('/api/v1/tags/bulk-delete', { body: ids });
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete tags');
			}
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
// Optimistic Topology Cache Update
// ============================================================================

/**
 * Optimistically update the topology cache after tag mutations.
 * The backend topology is a stored snapshot that doesn't reflect tag changes,
 * so we update the frontend cache directly for instant UI feedback.
 * Entity events will later persist the change across refresh.
 */
function updateTopologyEntityTags(
	queryClient: QueryClient,
	entityType: EntityDiscriminants,
	entityIds: string[],
	tagId: string,
	action: 'add' | 'remove'
) {
	const fieldMap: Partial<Record<EntityDiscriminants, 'hosts' | 'services' | 'subnets'>> = {
		Host: 'hosts',
		Service: 'services',
		Subnet: 'subnets'
	};
	const field = fieldMap[entityType];
	if (!field) return;

	queryClient.setQueryData<Topology[]>(queryKeys.topology.all, (old) => {
		if (!old) return old;
		return old.map((topo) => {
			const entities = topo[field];
			let changed = false;
			const updatedEntities = entities.map((entity) => {
				if (!entityIds.includes(entity.id)) return entity;
				const tags = entity.tags;
				if (action === 'add' && !tags.includes(tagId)) {
					changed = true;
					return { ...entity, tags: [...tags, tagId] };
				} else if (action === 'remove' && tags.includes(tagId)) {
					changed = true;
					return { ...entity, tags: tags.filter((t) => t !== tagId) };
				}
				return entity;
			});

			if (!changed) return topo;

			return { ...topo, [field]: updatedEntities };
		});
	});
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
	Group: 'groups',
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
	Interface: null,
	IfEntry: null,
	Topology: null,
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
			const { data } = await apiClient.POST('/api/v1/tags/assign/bulk-add', { body: request });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to add tag to entities');
			}
			return {
				...data.data,
				entityType: request.entity_type,
				tagId: request.tag_id,
				entityIds: request.entity_ids
			};
		},
		onSuccess: ({ entityType, tagId, entityIds }) => {
			updateTopologyEntityTags(queryClient, entityType, entityIds, tagId, 'add');
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
			const { data } = await apiClient.POST('/api/v1/tags/assign/bulk-remove', { body: request });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to remove tag from entities');
			}
			return {
				...data.data,
				entityType: request.entity_type,
				tagId: request.tag_id,
				entityIds: request.entity_ids
			};
		},
		onSuccess: ({ entityType, tagId, entityIds }) => {
			updateTopologyEntityTags(queryClient, entityType, entityIds, tagId, 'remove');
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
			const { data } = await apiClient.PUT('/api/v1/tags/assign', { body: request });
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to set entity tags');
			}
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
