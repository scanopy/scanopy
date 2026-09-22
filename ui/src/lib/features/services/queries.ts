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
import { requireSuccess, unwrapData, unwrapEnvelope } from '$lib/api/query-helpers';
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
	/** Filter by network ID. Several narrows to the union of them. */
	network_ids?: string[];
	/** Filter by host ID. Several narrows to the union of them. */
	host_ids?: string[];
	/** Only services with one of these definitions (raw definition ids). */
	service_definitions?: string[];
	/** Filter by the service containerizing this one. */
	virtualization_service_ids?: string[];
	/** Also return services nothing containerizes; on its own, only those. */
	include_uncontainerized?: boolean;
	/** Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
	group_by?: components['schemas']['ServiceOrderField'];
	/** Secondary ordering field (sorting within groups or standalone sort). */
	order_by?: components['schemas']['ServiceOrderField'];
	/** Direction for order_by field. */
	order_direction?: components['schemas']['OrderDirection'];
	/** Filter by tag IDs (returns services that have ANY of the specified tags). */
	tag_ids?: string[];
	/** `true` returns only services discovery hasn't observed within their
	 * network's staleness window; omit for no staleness constraint. */
	stale?: boolean;
	/** Free-text search across service name, service definition, and the name
	 * of the host the service runs on. */
	search?: string;
	/** Only services exposed on one of these port numbers, over either protocol. */
	ports?: number[];
	/** Exclude services belonging to these categories. */
	exclude_categories?: components['schemas']['ServiceCategory'][];
}

/**
 * Pagination metadata from API response. Derived from the generated schema so
 * fields the server adds (e.g. per-group totals) reach consumers automatically.
 */
export type PaginationMeta = components['schemas']['PaginationMeta'];

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
			network_ids,
			host_ids,
			service_definitions,
			virtualization_service_ids,
			include_uncontainerized,
			group_by,
			order_by,
			order_direction,
			tag_ids,
			stale,
			search,
			ports,
			exclude_categories
		} = params;

		return {
			queryKey: [
				...queryKeys.services.all,
				{
					limit,
					offset,
					network_ids,
					host_ids,
					service_definitions,
					virtualization_service_ids,
					include_uncontainerized,
					group_by,
					order_by,
					order_direction,
					tag_ids,
					stale,
					search,
					ports,
					exclude_categories
				}
			],
			queryFn: async (): Promise<PaginatedResult<Service>> => {
				const envelope = unwrapEnvelope(
					await apiClient.GET('/api/v1/services', {
						params: {
							query: {
								limit,
								offset,
								network_ids,
								host_ids,
								service_definitions,
								virtualization_service_ids,
								include_uncontainerized,
								group_by,
								order_by,
								order_direction,
								tag_ids,
								stale,
								search,
								ports,
								exclude_categories
							}
						}
					})
				);
				return {
					items: envelope.data,
					pagination: envelope.meta?.pagination ?? null
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

				return unwrapData(
					await apiClient.GET('/api/v1/services', {
						params: {
							query: {
								ids: ids,
								limit: 0 // No pagination when fetching by IDs
							}
						}
					})
				);
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
			return unwrapData(await apiClient.POST('/api/v1/services', { body: service }));
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
			return unwrapData(
				await apiClient.PUT('/api/v1/services/{id}', {
					params: { path: { id: service.id } },
					body: service
				})
			);
		},
		onSuccess: (updatedService: Service) => {
			queryClient.setQueryData<Service[]>(
				queryKeys.services.all,
				(old) => old?.map((s) => (s.id === updatedService.id ? updatedService : s)) ?? []
			);
			// Invalidate paginated service queries so ServiceTab reflects the update
			queryClient.invalidateQueries({ queryKey: queryKeys.services.all });
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
			requireSuccess(
				await apiClient.DELETE('/api/v1/services/{id}', {
					params: { path: { id } }
				})
			);
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
			requireSuccess(await apiClient.POST('/api/v1/services/bulk-delete', { body: ids }));
			return ids;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: queryKeys.services.all });
		}
	}));
}

// `useBulkUpdateServicesMutation` was removed here. It had no call sites, and it
// derived its DELETE set from `getQueryData(queryKeys.services.all)` — a
// synchronous snapshot of a cache with no fetcher of its own — so any host whose
// services were not in that cache would have had its unseen services deleted.
// Host service sync already goes through `PUT /api/v1/hosts/{id}`
// (`useUpdateHostMutation`), which reconciles create/update/delete server-side
// against the real rows.

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
		virtualization_metadata: null,
		virtualization_service_id: null,
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
