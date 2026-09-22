/**
 * TanStack Query hooks for Dependencies
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import { requireSuccess, unwrapData } from '$lib/api/query-helpers';
import type { Dependency } from './types/base';

/**
 * Query hook for fetching all dependencies
 */
export function useDependenciesQuery(atGetter?: () => string | undefined) {
	return createQuery(() => {
		const at = atGetter?.();
		return {
			queryKey: at ? [...queryKeys.dependencies.all, 'asOf', at] : queryKeys.dependencies.all,
			queryFn: async () => {
				return unwrapData(
					await apiClient.GET('/api/v1/dependencies', {
						params: { query: { limit: 0, at } }
					})
				);
			}
		};
	});
}

/**
 * Mutation hook for creating a dependency
 */
export function useCreateDependencyMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (dependency: Dependency) => {
			return unwrapData(await apiClient.POST('/api/v1/dependencies', { body: dependency }));
		},
		onSuccess: (newDependency: Dependency) => {
			queryClient.setQueryData<Dependency[]>(queryKeys.dependencies.all, (old) =>
				old ? [...old, newDependency] : [newDependency]
			);
			// Invalidate services as dependency creation may affect service bindings
			queryClient.invalidateQueries({ queryKey: queryKeys.services.all });
		}
	}));
}

/**
 * Mutation hook for updating a dependency
 */
export function useUpdateDependencyMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (dependency: Dependency) => {
			return unwrapData(
				await apiClient.PUT('/api/v1/dependencies/{id}', {
					params: { path: { id: dependency.id } },
					body: dependency
				})
			);
		},
		onSuccess: (updatedDependency: Dependency) => {
			queryClient.setQueryData<Dependency[]>(
				queryKeys.dependencies.all,
				(old) => old?.map((d) => (d.id === updatedDependency.id ? updatedDependency : d)) ?? []
			);
			// Invalidate services as dependency update may affect service bindings
			queryClient.invalidateQueries({ queryKey: queryKeys.services.all });
		}
	}));
}

/**
 * Mutation hook for deleting a dependency
 */
export function useDeleteDependencyMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			requireSuccess(
				await apiClient.DELETE('/api/v1/dependencies/{id}', {
					params: { path: { id } }
				})
			);
			return id;
		},
		onSuccess: (id: string) => {
			queryClient.setQueryData<Dependency[]>(
				queryKeys.dependencies.all,
				(old) => old?.filter((d) => d.id !== id) ?? []
			);
			// The topology graph is built on request (not stored on the row), so
			// invalidate the topology queries to refetch a graph rebuilt without
			// the deleted dependency's edges.
			queryClient.invalidateQueries({ queryKey: queryKeys.topology.all });
			// Invalidate services as dependency deletion may affect service bindings
			queryClient.invalidateQueries({ queryKey: queryKeys.services.all });
		}
	}));
}

/**
 * Mutation hook for bulk deleting dependencies
 */
export function useBulkDeleteDependenciesMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (ids: string[]) => {
			requireSuccess(await apiClient.POST('/api/v1/dependencies/bulk-delete', { body: ids }));
			return ids;
		},
		onSuccess: (ids: string[]) => {
			queryClient.setQueryData<Dependency[]>(
				queryKeys.dependencies.all,
				(old) => old?.filter((d) => !ids.includes(d.id)) ?? []
			);
			// Invalidate services as dependency deletion may affect service bindings
			queryClient.invalidateQueries({ queryKey: queryKeys.services.all });
		}
	}));
}

/**
 * Mutation hook for updating only a dependency's description.
 * Reads authoritative dependency data (with correct members) from the dependencies query cache
 * to avoid re-saving members from potentially stale topology snapshot data.
 */
export function useUpdateDependencyDescriptionMutation() {
	const queryClient = useQueryClient();
	return createMutation(() => ({
		mutationFn: async (data: { dependencyId: string; description: string | null }) => {
			const dependencies = queryClient.getQueryData<Dependency[]>(queryKeys.dependencies.all);
			const currentDependency = dependencies?.find((d) => d.id === data.dependencyId);
			if (!currentDependency) throw new Error('Dependency not found in cache');

			return unwrapData(
				await apiClient.PUT('/api/v1/dependencies/{id}', {
					params: { path: { id: data.dependencyId } },
					body: { ...currentDependency, description: data.description }
				})
			);
		},
		onSuccess: (updatedDependency: Dependency) => {
			queryClient.setQueryData<Dependency[]>(
				queryKeys.dependencies.all,
				(old) => old?.map((d) => (d.id === updatedDependency.id ? updatedDependency : d)) ?? []
			);
		}
	}));
}

import { utcTimeZoneSentinel, uuidv4Sentinel } from '$lib/shared/utils/formatting';
import { entities } from '$lib/shared/stores/metadata';
import type { Color } from '$lib/shared/utils/styling';

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Create empty form data for creating a new dependency
 */
export function createEmptyDependencyFormData(defaultNetworkId?: string): Dependency {
	return {
		id: uuidv4Sentinel,
		name: '',
		description: '',
		members: { type: 'Services', service_ids: [] },
		created_at: utcTimeZoneSentinel,
		updated_at: utcTimeZoneSentinel,
		dependency_type: 'RequestPath',
		source: {
			type: 'Manual'
		},
		network_id: defaultNetworkId ?? '',
		color: entities.getColorHelper('Dependency').color as Color,
		edge_style: 'Straight',
		tags: []
	};
}
