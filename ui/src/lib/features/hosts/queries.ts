/**
 * TanStack Query hooks for Hosts
 *
 * Hosts are the parent entity that populates child caches for interfaces, ports, and services.
 */

import {
	createQuery,
	createMutation,
	useQueryClient,
	keepPreviousData
} from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import { pushSuccess } from '$lib/shared/stores/feedback';
import type {
	Host,
	HostResponse,
	HostFormData,
	Interface,
	Port,
	CreateHostWithServicesRequest,
	UpdateHostWithServicesRequest,
	CreateHostRequest,
	UpdateHostRequest,
	InterfaceInput,
	PortInput,
	ServiceInput,
	BindingInput,
	AllInterfaces,
	IfEntry
} from './types/base';
import type { Service } from '$lib/features/services/types/base';
import type { components } from '$lib/api/schema';

// Re-export types for convenience
export type { Host, HostResponse, HostFormData, Interface, Port };

/**
 * Extract Host primitive from HostResponse (excludes children).
 * Uses rest spread to automatically include all Host fields.
 * If HostResponse adds new child arrays, TypeScript will error
 * because Host type won't have those fields.
 */
export function toHostPrimitive(response: HostResponse): Host {
	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	const { interfaces, ports, services, if_entries, ...hostFields } = response;

	// Normalize optional fields from HostResponse to required nullable fields in Host
	return {
		...hostFields,
		description: hostFields.description ?? null,
		hostname: hostFields.hostname ?? null,
		virtualization: hostFields.virtualization ?? null,
		credential_assignments: hostFields.credential_assignments ?? []
	};
}

/**
 * Extract Host primitive from HostFormData (excludes children).
 * Uses rest spread to automatically include all Host fields.
 */
export function formDataToHostPrimitive(formData: HostFormData): Host {
	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	const { interfaces, ports, services, if_entries, ...hostFields } = formData;
	return hostFields;
}

/**
 * Helper to convert Service binding to BindingInput format for API
 */
function toBindingInput(binding: Service['bindings'][0]): BindingInput {
	if (binding.type === 'Interface') {
		return {
			type: 'Interface',
			id: binding.id,
			interface_id: binding.interface_id
		};
	} else {
		return {
			type: 'Port',
			id: binding.id,
			port_id: binding.port_id,
			interface_id: binding.interface_id ?? undefined
		};
	}
}

/**
 * Transform HostFormData to CreateHostRequest format for API.
 * Now includes services for single-step host creation with all children.
 */
function toCreateHostRequest(formData: HostFormData): CreateHostRequest {
	return {
		name: formData.name,
		network_id: formData.network_id,
		hostname: formData.hostname,
		description: formData.description,
		virtualization: formData.virtualization,
		hidden: formData.hidden,
		tags: formData.tags,
		interfaces: formData.interfaces.map(
			(iface, index): InterfaceInput => ({
				id: iface.id,
				subnet_id: iface.subnet_id,
				ip_address: iface.ip_address,
				mac_address: iface.mac_address,
				name: iface.name,
				position: index // Use array order as position
			})
		),
		ports: formData.ports.map(
			(port): PortInput => ({
				id: port.id,
				number: port.number,
				protocol: port.protocol
			})
		),
		services: formData.services.map(
			(service, index): ServiceInput => ({
				id: service.id,
				service_definition: service.service_definition,
				name: service.name,
				bindings: service.bindings.map(toBindingInput),
				virtualization: service.virtualization,
				tags: service.tags,
				position: index
			})
		)
	};
}

/**
 * Query options for host list queries including pagination and ordering
 */
export interface HostQueryOptions {
	limit?: number;
	offset?: number;
	/** Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
	group_by?: components['schemas']['HostOrderField'];
	/** Secondary ordering field (sorting within groups or standalone sort). */
	order_by?: components['schemas']['HostOrderField'];
	/** Direction for order_by field. */
	order_direction?: components['schemas']['OrderDirection'];
	/** Filter by tag IDs (returns hosts that have ANY of the specified tags). */
	tag_ids?: string[];
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
 * Query hook for fetching hosts with optional pagination and ordering
 * Populates interfaces, ports, and services caches from the response
 *
 * @param optionsOrGetter - Query options or getter function returning options.
 *                          Use getter function for reactive options (e.g., when offset or ordering changes).
 *                          Omit or pass {} for default (limit=50).
 *                          Pass { limit: 0 } for unlimited (all hosts).
 */
export function useHostsQuery(optionsOrGetter: HostQueryOptions | (() => HostQueryOptions) = {}) {
	const queryClient = useQueryClient();

	return createQuery(() => {
		const options = typeof optionsOrGetter === 'function' ? optionsOrGetter() : optionsOrGetter;

		return {
			queryKey: queryKeys.hosts.list(options as Record<string, unknown>),
			queryFn: async (): Promise<PaginatedResult<Host>> => {
				const { data } = await apiClient.GET('/api/v1/hosts', {
					params: {
						query: {
							limit: options.limit,
							offset: options.offset,
							group_by: options.group_by,
							order_by: options.order_by,
							order_direction: options.order_direction,
							tag_ids: options.tag_ids
						}
					}
				});
				if (!data?.success || !data.data) {
					throw new Error(data?.error || 'Failed to fetch hosts');
				}

				const responses = data.data;

				// Extract child data from current response
				const allInterfaces = responses.flatMap((r) => r.interfaces);
				const allPorts = responses.flatMap((r) => r.ports);
				const allServices = responses.flatMap((r) => r.services);
				const allIfEntries = responses.flatMap((r) => r.if_entries);

				// Get host IDs from current response to merge correctly
				const currentHostIds = new Set(responses.map((r) => r.id));

				// Merge: keep data from other hosts, update current hosts
				queryClient.setQueryData<Interface[]>(queryKeys.interfaces.all, (old) => {
					if (!old) return allInterfaces;
					const others = old.filter((i) => !currentHostIds.has(i.host_id));
					return [...others, ...allInterfaces];
				});

				queryClient.setQueryData<Port[]>(queryKeys.ports.all, (old) => {
					if (!old) return allPorts;
					const others = old.filter((p) => !currentHostIds.has(p.host_id));
					return [...others, ...allPorts];
				});

				queryClient.setQueryData<Service[]>(queryKeys.services.all, (old) => {
					if (!old) return allServices;
					const others = old.filter((s) => !currentHostIds.has(s.host_id));
					return [...others, ...allServices];
				});

				queryClient.setQueryData<IfEntry[]>(queryKeys.ifEntries.all, (old) => {
					if (!old) return allIfEntries;
					const others = old.filter((e) => !currentHostIds.has(e.host_id));
					return [...others, ...allIfEntries];
				});

				// Return host primitives with pagination metadata
				return {
					items: responses.map(toHostPrimitive),
					pagination: data.meta?.pagination ?? null
				};
			},
			// Keep showing previous page data while fetching next page
			placeholderData: keepPreviousData
		};
	});
}

/**
 * Query hook for fetching specific hosts by IDs (for selective loading)
 * Used for lookups where only a subset of hosts is needed (e.g., service → host name)
 *
 * @param idsGetter - Getter function returning array of host IDs to fetch
 */
export function useHostsByIds(idsGetter: () => string[]) {
	return createQuery(() => {
		const ids = idsGetter();

		return {
			queryKey: [...queryKeys.hosts.all, 'byIds', ids],
			queryFn: async (): Promise<Host[]> => {
				if (ids.length === 0) return [];

				const { data } = await apiClient.GET('/api/v1/hosts', {
					params: {
						query: {
							ids: ids,
							limit: 0 // No pagination when fetching by IDs
						}
					}
				});
				if (!data?.success || !data.data) {
					throw new Error(data?.error || 'Failed to fetch hosts');
				}

				return data.data.map(toHostPrimitive);
			},
			enabled: ids.length > 0
		};
	});
}

/**
 * Mutation hook for creating a host
 */
export function useCreateHostMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (data: CreateHostWithServicesRequest) => {
			const request = toCreateHostRequest(data.host);
			const { data: result } = await apiClient.POST('/api/v1/hosts', { body: request });
			if (!result?.success || !result.data) {
				throw new Error(result?.error || 'Failed to create host');
			}
			return result.data;
		},
		onSuccess: (response: HostResponse) => {
			// Invalidate all host list queries to refetch with updated data
			queryClient.invalidateQueries({ queryKey: queryKeys.hosts.lists() });

			// Add children to their caches
			queryClient.setQueryData<Interface[]>(queryKeys.interfaces.all, (old) =>
				old ? [...old, ...response.interfaces] : response.interfaces
			);
			queryClient.setQueryData<Port[]>(queryKeys.ports.all, (old) =>
				old ? [...old, ...response.ports] : response.ports
			);
			queryClient.setQueryData<Service[]>(queryKeys.services.all, (old) =>
				old ? [...old, ...response.services] : response.services
			);
			queryClient.setQueryData<IfEntry[]>(queryKeys.ifEntries.all, (old) =>
				old ? [...old, ...response.if_entries] : response.if_entries
			);
		}
	}));
}

/**
 * Mutation hook for updating a host.
 * All entities (interfaces, ports, services, bindings) use client-provided UUIDs.
 * Backend determines create vs update by checking if the ID exists for this host.
 */
export function useUpdateHostMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (data: UpdateHostWithServicesRequest) => {
			const request: UpdateHostRequest = {
				id: data.host.id,
				name: data.host.name,
				hostname: data.host.hostname,
				description: data.host.description,
				virtualization: data.host.virtualization,
				hidden: data.host.hidden,
				tags: data.host.tags,
				credential_assignments: data.host.credential_assignments ?? undefined,
				expected_updated_at: data.host.updated_at,
				// Only send arrays if provided (undefined = preserve existing)
				interfaces: data.interfaces
					? data.interfaces.map(
							(iface, index): InterfaceInput => ({
								id: iface.id,
								subnet_id: iface.subnet_id,
								ip_address: iface.ip_address,
								mac_address: iface.mac_address,
								name: iface.name,
								position: index
							})
						)
					: undefined,
				ports: data.ports
					? data.ports.map(
							(port): PortInput => ({
								id: port.id,
								number: port.number,
								protocol: port.protocol
							})
						)
					: undefined,
				services: data.services
					? data.services.map(
							(service, index): ServiceInput => ({
								id: service.id,
								service_definition: service.service_definition,
								name: service.name,
								bindings: service.bindings.map(toBindingInput),
								virtualization: service.virtualization,
								tags: service.tags,
								position: index
							})
						)
					: undefined
			};

			const { data: result } = await apiClient.PUT('/api/v1/hosts/{id}', {
				params: { path: { id: data.host.id } },
				body: request
			});
			if (!result?.success || !result.data) {
				throw new Error(result?.error || 'Failed to update host');
			}

			return { response: result.data };
		},
		onSuccess: ({ response }) => {
			const hostId = response.id;

			// Invalidate all host list queries to refetch with updated data
			queryClient.invalidateQueries({ queryKey: queryKeys.hosts.lists() });

			// Replace interfaces for this host
			queryClient.setQueryData<Interface[]>(queryKeys.interfaces.all, (old) => {
				const others = old?.filter((i) => i.host_id !== hostId) ?? [];
				return [...others, ...response.interfaces];
			});

			// Replace ports for this host
			queryClient.setQueryData<Port[]>(queryKeys.ports.all, (old) => {
				const others = old?.filter((p) => p.host_id !== hostId) ?? [];
				return [...others, ...response.ports];
			});

			// Replace services for this host (synced via host endpoint with positions)
			queryClient.setQueryData<Service[]>(queryKeys.services.all, (old) => {
				const others = old?.filter((s) => s.host_id !== hostId) ?? [];
				return [...others, ...response.services];
			});

			// Replace ifEntries for this host
			queryClient.setQueryData<IfEntry[]>(queryKeys.ifEntries.all, (old) => {
				const others = old?.filter((e) => e.host_id !== hostId) ?? [];
				return [...others, ...response.if_entries];
			});
		}
	}));
}

/**
 * Mutation hook for updating only a host's description.
 * Omits tags from the request body to avoid set_tags() conflicts.
 */
export function useUpdateHostDescriptionMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (data: { host: Host; description: string | null }) => {
			const { data: result } = await apiClient.PUT('/api/v1/hosts/{id}', {
				params: { path: { id: data.host.id } },
				body: {
					id: data.host.id,
					name: data.host.name,
					hostname: data.host.hostname,
					description: data.description,
					virtualization: data.host.virtualization,
					hidden: data.host.hidden,
					expected_updated_at: data.host.updated_at,
					tags: data.host.tags ?? []
				}
			});
			if (!result?.success || !result.data) {
				throw new Error(result?.error || 'Failed to update host');
			}
			return result.data;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: queryKeys.hosts.lists() });
		}
	}));
}

/**
 * Mutation hook for deleting a host
 */
export function useDeleteHostMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			const { data } = await apiClient.DELETE('/api/v1/hosts/{id}', {
				params: { path: { id } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete host');
			}
			return id;
		},
		onSuccess: (id: string) => {
			// Invalidate all host list queries to refetch with updated data
			queryClient.invalidateQueries({ queryKey: queryKeys.hosts.lists() });

			// Remove children from their caches
			queryClient.setQueryData<Interface[]>(
				queryKeys.interfaces.all,
				(old) => old?.filter((i) => i.host_id !== id) ?? []
			);
			queryClient.setQueryData<Port[]>(
				queryKeys.ports.all,
				(old) => old?.filter((p) => p.host_id !== id) ?? []
			);
			queryClient.setQueryData<Service[]>(
				queryKeys.services.all,
				(old) => old?.filter((s) => s.host_id !== id) ?? []
			);
			queryClient.setQueryData<IfEntry[]>(
				queryKeys.ifEntries.all,
				(old) => old?.filter((e) => e.host_id !== id) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for bulk deleting hosts
 */
export function useBulkDeleteHostsMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (ids: string[]) => {
			const { data } = await apiClient.POST('/api/v1/hosts/bulk-delete', { body: ids });
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete hosts');
			}
			return ids;
		},
		onSuccess: (ids: string[]) => {
			const idSet = new Set(ids);

			// Invalidate all host list queries to refetch with updated data
			queryClient.invalidateQueries({ queryKey: queryKeys.hosts.lists() });

			// Remove children from their caches
			queryClient.setQueryData<Interface[]>(
				queryKeys.interfaces.all,
				(old) => old?.filter((i) => !idSet.has(i.host_id)) ?? []
			);
			queryClient.setQueryData<Port[]>(
				queryKeys.ports.all,
				(old) => old?.filter((p) => !idSet.has(p.host_id)) ?? []
			);
			queryClient.setQueryData<Service[]>(
				queryKeys.services.all,
				(old) => old?.filter((s) => !idSet.has(s.host_id)) ?? []
			);
			queryClient.setQueryData<IfEntry[]>(
				queryKeys.ifEntries.all,
				(old) => old?.filter((e) => !idSet.has(e.host_id)) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for consolidating hosts
 */
export function useConsolidateHostsMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async ({
			destinationHostId,
			otherHostId,
			otherHostName
		}: {
			destinationHostId: string;
			otherHostId: string;
			otherHostName?: string;
		}) => {
			const { data } = await apiClient.PUT(
				'/api/v1/hosts/{destination_host}/consolidate/{other_host}',
				{
					params: { path: { destination_host: destinationHostId, other_host: otherHostId } }
				}
			);
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to consolidate hosts');
			}
			return { response: data.data, otherHostId, otherHostName };
		},
		onSuccess: ({ response, otherHostId, otherHostName }) => {
			// Invalidate all host list queries to refetch with updated data
			queryClient.invalidateQueries({ queryKey: queryKeys.hosts.lists() });

			// Remove children of consolidated host and update destination host children
			queryClient.setQueryData<Interface[]>(queryKeys.interfaces.all, (old) => {
				const others =
					old?.filter((i) => i.host_id !== otherHostId && i.host_id !== response.id) ?? [];
				return [...others, ...response.interfaces];
			});
			queryClient.setQueryData<Port[]>(queryKeys.ports.all, (old) => {
				const others =
					old?.filter((p) => p.host_id !== otherHostId && p.host_id !== response.id) ?? [];
				return [...others, ...response.ports];
			});
			queryClient.setQueryData<Service[]>(queryKeys.services.all, (old) => {
				const others =
					old?.filter((s) => s.host_id !== otherHostId && s.host_id !== response.id) ?? [];
				return [...others, ...response.services];
			});
			queryClient.setQueryData<IfEntry[]>(queryKeys.ifEntries.all, (old) => {
				const others =
					old?.filter((e) => e.host_id !== otherHostId && e.host_id !== response.id) ?? [];
				return [...others, ...response.if_entries];
			});

			if (otherHostName) {
				pushSuccess(`Consolidated host "${otherHostName}" into host "${response.name}"`);
			}
		}
	}));
}

/**
 * Format an interface for display
 */
export function formatInterface(
	i: Interface | AllInterfaces,
	isContainerSubnetFn: (subnetId: string) => boolean
): string {
	if (i.id == null) return i.name;
	return isContainerSubnetFn(i.subnet_id)
		? (i.name ?? i.ip_address)
		: (i.name ? i.name + ': ' : '') + i.ip_address;
}

/**
 * Hydrate a Host primitive to HostFormData using TanStack Query cache.
 * Used for form editing where the full form structure is needed.
 */
export function hydrateHostToFormData(
	host: Host,
	queryClient: ReturnType<typeof useQueryClient>
): HostFormData {
	const allInterfaces = queryClient.getQueryData<Interface[]>(queryKeys.interfaces.all) ?? [];
	const allPorts = queryClient.getQueryData<Port[]>(queryKeys.ports.all) ?? [];
	const allServices = queryClient.getQueryData<Service[]>(queryKeys.services.all) ?? [];
	const allIfEntries = queryClient.getQueryData<IfEntry[]>(queryKeys.ifEntries.all) ?? [];

	return {
		...host,
		interfaces: allInterfaces.filter((i) => i.host_id === host.id),
		ports: allPorts.filter((p) => p.host_id === host.id),
		services: allServices.filter((s) => s.host_id === host.id),
		if_entries: allIfEntries.filter((e) => e.host_id === host.id),
		// SNMP fields from host
		sys_descr: host.sys_descr ?? null,
		sys_object_id: host.sys_object_id ?? null,
		sys_location: host.sys_location ?? null,
		sys_contact: host.sys_contact ?? null,
		management_url: host.management_url ?? null,
		chassis_id: host.chassis_id ?? null,
		credential_assignments: host.credential_assignments ?? []
	};
}

import { utcTimeZoneSentinel, uuidv4Sentinel } from '$lib/shared/utils/formatting';

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Create empty form data for creating a new host.
 * @param defaultNetworkId - Optional network ID to use as default.
 */
export function createEmptyHostFormData(defaultNetworkId?: string): HostFormData {
	return {
		id: uuidv4Sentinel,
		created_at: utcTimeZoneSentinel,
		updated_at: utcTimeZoneSentinel,
		name: '',
		description: null,
		tags: [],
		hostname: null,
		services: [],
		interfaces: [],
		ports: [],
		source: {
			type: 'Manual'
		},
		virtualization: null,
		network_id: defaultNetworkId ?? '',
		hidden: false,
		// SNMP fields (populated by discovery)
		sys_descr: null,
		sys_object_id: null,
		sys_location: null,
		sys_contact: null,
		management_url: null,
		chassis_id: null,
		credential_assignments: [],
		if_entries: []
	};
}

/**
 * Get a host by ID from the cache.
 * Searches through all paginated host query caches.
 */
export function getHostByIdFromCache(
	queryClient: ReturnType<typeof useQueryClient>,
	id: string
): Host | null {
	// Get all data from paginated host list queries
	const queriesData = queryClient.getQueriesData<PaginatedResult<Host>>({
		queryKey: queryKeys.hosts.lists()
	});

	for (const [, data] of queriesData) {
		if (data?.items) {
			const found = data.items.find((h) => h.id === id);
			if (found) return found;
		}
	}

	return null;
}

/**
 * Get a host by interface ID from the cache.
 * Searches through all paginated host query caches.
 */
export function getHostFromInterfaceIdFromCache(
	queryClient: ReturnType<typeof useQueryClient>,
	interfaceId: string
): Host | null {
	const interfaces = queryClient.getQueryData<Interface[]>(queryKeys.interfaces.all) ?? [];
	const iface = interfaces.find((i) => i.id === interfaceId);
	if (!iface) return null;

	return getHostByIdFromCache(queryClient, iface.host_id);
}
