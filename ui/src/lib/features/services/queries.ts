/**
 * TanStack Query hooks for Services
 *
 * Services are populated by the hosts query but also have direct CRUD operations.
 */

import {
	createQuery,
	createMutation,
	useQueryClient,
	keepPreviousData
} from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import type { Service } from './types/base';
import { utcTimeZoneSentinel } from '$lib/shared/utils/formatting';
import { v4 as uuidv4 } from 'uuid';
import type { components } from '$lib/api/schema';

// Re-export type for convenience
export type { Service };

/**
 * Query parameters for services query including pagination and ordering
 */
export interface ServicesQueryParams {
	limit?: number;
	offset?: number;
	network_id?: string;
	host_id?: string;
	/** Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
	group_by?: components['schemas']['ServiceOrderField'];
	/** Secondary ordering field (sorting within groups or standalone sort). */
	order_by?: components['schemas']['ServiceOrderField'];
	/** Direction for order_by field. */
	order_direction?: components['schemas']['OrderDirection'];
	/** Filter by tag IDs (returns services that have ANY of the specified tags). */
	tag_ids?: string[];
	/** Exclude services belonging to these categories. */
	exclude_categories?: components['schemas']['ServiceCategory'][];
}

/**
 * Pagination metadata from API response
 */
export interface PaginationMeta {
	total_count: number;
	limit: number;
	offset: number;
	has_more: boolean;
}

/**
 * Result of a paginated query
 */
export interface PaginatedResult<T> {
	items: T[];
	pagination: PaginationMeta | null;
}

/**
 * Query hook for accessing the services cache with pagination and ordering
 * This cache is primarily populated by useHostsQuery
 *
 * @param paramsOrGetter - Query parameters or getter function returning parameters.
 *                         Use getter function for reactive options (e.g., when offset or ordering changes).
 */
export function useServicesQuery(
	paramsOrGetter: ServicesQueryParams | (() => ServicesQueryParams) = {}
) {
	return createQuery(() => {
		const params = typeof paramsOrGetter === 'function' ? paramsOrGetter() : paramsOrGetter;
		const {
			limit,
			offset,
			network_id,
			host_id,
			group_by,
			order_by,
			order_direction,
			tag_ids,
			exclude_categories
		} = params;

		return {
			queryKey: [
				...queryKeys.services.all,
				{
					limit,
					offset,
					network_id,
					host_id,
					group_by,
					order_by,
					order_direction,
					tag_ids,
					exclude_categories
				}
			],
			queryFn: async (): Promise<PaginatedResult<Service>> => {
				const { data } = await apiClient.GET('/api/v1/services', {
					params: {
						query: {
							limit,
							offset,
							network_id,
							host_id,
							group_by,
							order_by,
							order_direction,
							tag_ids,
							exclude_categories
						}
					}
				});
				if (!data?.success || !data.data) {
					throw new Error(data?.error || 'Failed to fetch services');
				}
				return {
					items: data.data,
					pagination: data.meta?.pagination ?? null
				};
			},
			// Keep showing previous page data while fetching next page
			placeholderData: keepPreviousData
		};
	});
}

/**
 * Query hook for accessing the services cache populated by useHostsQuery.
 * This does NOT make API calls - it reads from the cache that hosts query populates.
 * Use this when you need all services for filtering (e.g., by host_id in HostCard).
 *
 * For paginated/filtered API calls, use useServicesQuery() instead.
 */
export function useServicesCacheQuery() {
	return createQuery(() => ({
		queryKey: queryKeys.services.all,
		initialData: [] as Service[],
		staleTime: Infinity,
		refetchOnMount: false,
		refetchOnWindowFocus: false,
		enabled: false
	}));
}

/**
 * Query hook for fetching specific services by IDs (for selective loading)
 * Used for lookups where only a subset of services is needed (e.g., virtualization lookups)
 *
 * @param idsGetter - Getter function returning array of service IDs to fetch
 */
export function useServicesByIds(idsGetter: () => string[]) {
	return createQuery(() => {
		const ids = idsGetter();

		return {
			queryKey: [...queryKeys.services.all, 'byIds', ids],
			queryFn: async (): Promise<Service[]> => {
				if (ids.length === 0) return [];

				const { data } = await apiClient.GET('/api/v1/services', {
					params: {
						query: {
							ids: ids,
							limit: 0 // No pagination when fetching by IDs
						}
					}
				});
				if (!data?.success || !data.data) {
					throw new Error(data?.error || 'Failed to fetch services');
				}

				return data.data;
			},
			enabled: ids.length > 0
		};
	});
}

/**
 * Mutation hook for creating a service
 */
export function useCreateServiceMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (service: Service) => {
			const { data } = await apiClient.POST('/api/v1/services', { body: service });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to create service');
			}
			return data.data;
		},
		onSuccess: (newService: Service) => {
			queryClient.setQueryData<Service[]>(queryKeys.services.all, (old) =>
				old ? [...old, newService] : [newService]
			);
		}
	}));
}

/**
 * Mutation hook for updating a service
 */
export function useUpdateServiceMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (service: Service) => {
			const { data } = await apiClient.PUT('/api/v1/services/{id}', {
				params: { path: { id: service.id } },
				body: service
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to update service');
			}
			return data.data;
		},
		onSuccess: (updatedService: Service) => {
			queryClient.setQueryData<Service[]>(
				queryKeys.services.all,
				(old) => old?.map((s) => (s.id === updatedService.id ? updatedService : s)) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for deleting a service
 */
export function useDeleteServiceMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			const { data } = await apiClient.DELETE('/api/v1/services/{id}', {
				params: { path: { id } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete service');
			}
			return id;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: queryKeys.services.all });
		}
	}));
}

/**
 * Mutation hook for bulk deleting services
 */
export function useBulkDeleteServicesMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (ids: string[]) => {
			const { data } = await apiClient.POST('/api/v1/services/bulk-delete', { body: ids });
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete services');
			}
			return ids;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: queryKeys.services.all });
		}
	}));
}

/**
 * Mutation hook for bulk updating services for a host
 * Orchestrates create/update/delete operations
 */
export function useBulkUpdateServicesMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async ({ hostId, services }: { hostId: string; services: Service[] }) => {
			const currentServices = queryClient.getQueryData<Service[]>(queryKeys.services.all) ?? [];
			const hostServices = currentServices.filter((s) => s.host_id === hostId);

			const newServiceIds = new Set(services.map((s) => s.id));
			const currentServiceIds = new Set(hostServices.map((s) => s.id));

			// Detect creates, updates, deletes
			const toCreate = services.filter(
				(s) => !currentServiceIds.has(s.id) || s.id.startsWith('00000000')
			);
			const toUpdate = services.filter(
				(s) => currentServiceIds.has(s.id) && !s.id.startsWith('00000000')
			);
			const toDelete = hostServices.filter((s) => !newServiceIds.has(s.id));

			// Execute all operations
			const results = await Promise.all([
				...toCreate.map((s) =>
					apiClient.POST('/api/v1/services', {
						body: { ...s, id: undefined } as unknown as Service
					})
				),
				...toUpdate.map((s) =>
					apiClient.PUT('/api/v1/services/{id}', { params: { path: { id: s.id } }, body: s })
				),
				...toDelete.map((s) =>
					apiClient.DELETE('/api/v1/services/{id}', { params: { path: { id: s.id } } })
				)
			]);

			// Collect created/updated services from results
			const createdUpdated: Service[] = [];
			for (let i = 0; i < toCreate.length + toUpdate.length; i++) {
				const result = results[i];
				if (result.data?.success && result.data.data) {
					createdUpdated.push(result.data.data as Service);
				}
			}

			return { hostId, createdUpdated, deletedIds: toDelete.map((s) => s.id) };
		},
		onSuccess: ({ hostId, createdUpdated, deletedIds }) => {
			queryClient.setQueryData<Service[]>(queryKeys.services.all, (old) => {
				if (!old) return createdUpdated;

				// Remove deleted and old host services
				const others = old.filter((s) => s.host_id !== hostId || !deletedIds.includes(s.id));

				// Replace host services with created/updated
				const nonHostServices = others.filter((s) => s.host_id !== hostId);
				return [...nonHostServices, ...createdUpdated];
			});
		}
	}));
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Create a default empty service for a host
 */
export function createDefaultService(
	serviceType: string,
	host_id: string,
	host_network_id: string
): Service {
	return {
		id: uuidv4(), // Generate real UUID for client-provided ID
		created_at: utcTimeZoneSentinel,
		updated_at: utcTimeZoneSentinel,
		network_id: host_network_id,
		host_id,
		tags: [],
		service_definition: serviceType,
		name: serviceType,
		bindings: [],
		virtualization: null,
		position: 0, // Will be set by server based on existing services
		source: {
			type: 'Manual'
		}
	};
}

/**
 * Get a display name for a service
 */
export function getServiceName(service: Service): string {
	return service.name || service.service_definition;
}
