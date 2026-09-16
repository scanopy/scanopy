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
import { hostDisplayName } from './host-display-name';
import { hosts_consolidatedToast, hosts_rescanStartedToast } from '$lib/paraglide/messages';
import { discoverySSEManager } from '$lib/features/discovery/queries';
import type { DiscoveryUpdatePayload } from '$lib/features/discovery/types/api';
import type {
	Host,
	HostResponse,
	HostFormData,
	IPAddress,
	AllIPAddresses,
	Interface,
	Port,
	CreateHostWithServicesRequest,
	UpdateHostWithServicesRequest,
	CreateHostRequest,
	UpdateHostRequest,
	IPAddressInput,
	PortInput,
	ServiceInput,
	BindingInput
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
	const { ip_addresses, ports, services, interfaces, ...hostFields } = response;

	// Normalize optional fields from HostResponse to required nullable fields in Host.
	//
	// The discovered attributes, hostname included, go the other way: `HostResponse` still sends
	// them as nullable strings, while on `Host` each travels with the source that produced it, so
	// absence is `undefined` — a value with no source is not a state that exists.
	return {
		...hostFields,
		description: hostFields.description ?? null,
		virtualization_metadata: hostFields.virtualization_metadata ?? null,
		virtualization_service_id: hostFields.virtualization_service_id ?? null,
		credential_assignments: hostFields.credential_assignments ?? [],
		hostname: hostFields.hostname ?? undefined,
		sys_descr: hostFields.sys_descr ?? undefined,
		sys_object_id: hostFields.sys_object_id ?? undefined,
		sys_location: hostFields.sys_location ?? undefined,
		sys_contact: hostFields.sys_contact ?? undefined,
		management_url: hostFields.management_url ?? undefined,
		chassis_id: hostFields.chassis_id ?? undefined,
		sys_name: hostFields.sys_name ?? undefined,
		manufacturer: hostFields.manufacturer ?? undefined,
		model: hostFields.model ?? undefined,
		serial_number: hostFields.serial_number ?? undefined,
		firmware_revision: hostFields.firmware_revision ?? undefined,
		software_revision: hostFields.software_revision ?? undefined
	};
}

/**
 * Extract Host primitive from HostFormData (excludes children).
 * Uses rest spread to automatically include all Host fields.
 */
export function formDataToHostPrimitive(formData: HostFormData): Host {
	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	const { ip_addresses, ports, services, interfaces, ...hostFields } = formData;
	return hostFields;
}

/**
 * Helper to convert Service binding to BindingInput format for API
 */
function toBindingInput(binding: Service['bindings'][0]): BindingInput {
	if (binding.type === 'IPAddress') {
		return {
			type: 'IPAddress',
			id: binding.id,
			ip_address_id: binding.ip_address_id
		};
	} else {
		return {
			type: 'Port',
			id: binding.id,
			port_id: binding.port_id,
			ip_address_id: binding.ip_address_id ?? undefined
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
		hostname: formData.hostname ?? null,
		description: formData.description,
		virtualization_metadata: formData.virtualization_metadata,
		virtualization_service_id: formData.virtualization_service_id,
		hidden: formData.hidden,
		tags: formData.tags,
		ip_addresses: formData.ip_addresses.map(
			(iface, index): IPAddressInput => ({
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
				virtualization_metadata: service.virtualization_metadata,
				virtualization_service_id: service.virtualization_service_id,
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
	/** Filter by network ID. Several narrows to the union of them. */
	network_ids?: string[];
	/** Filter by the `hidden` flag. Omit for no constraint. */
	hidden?: boolean[];
	/** Filter by the service virtualizing the host. */
	virtualization_service_ids?: string[];
	/** Also return hosts nothing virtualizes; on its own, only those. */
	include_unvirtualized?: boolean;
	/** Filter to hosts running a service with one of these names. */
	service_names?: string[];
	/** Filter by how the host came to exist (`source.type`). */
	sources?: components['schemas']['EntitySourceDiscriminants'][];
	/** Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
	group_by?: components['schemas']['HostOrderField'];
	/** Secondary ordering field (sorting within groups or standalone sort). */
	order_by?: components['schemas']['HostOrderField'];
	/** Direction for order_by field. */
	order_direction?: components['schemas']['OrderDirection'];
	/** Filter by tag IDs (returns hosts that have ANY of the specified tags). */
	tag_ids?: string[];
	/** `true` returns only hosts discovery hasn't observed within their network's
	 * staleness window; omit for no staleness constraint. */
	stale?: boolean;
	/** Free-text search across host name, hostname, description, IP addresses
	 * and the names of services running on the host. */
	search?: string;
	/** As-of timestamp (ISO 8601). When set, returns SCD2 state as of this instant
	 * (snapshot view) instead of live state. */
	at?: string;
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
							network_ids: options.network_ids,
							group_by: options.group_by,
							order_by: options.order_by,
							order_direction: options.order_direction,
							tag_ids: options.tag_ids,
							stale: options.stale,
							search: options.search,
							at: options.at,
							hidden: options.hidden,
							virtualization_service_ids: options.virtualization_service_ids,
							include_unvirtualized: options.include_unvirtualized,
							service_names: options.service_names,
							sources: options.sources
						}
					}
				});
				if (!data?.success || !data.data) {
					throw new Error(data?.error || 'Failed to fetch hosts');
				}

				const responses = data.data;

				// Extract child data from current response
				const allIPAddresses = responses.flatMap((r) => r.ip_addresses);
				const allPorts = responses.flatMap((r) => r.ports);
				const allServices = responses.flatMap((r) => r.services);
				const allInterfaces = responses.flatMap((r) => r.interfaces);

				// Get host IDs from current response to merge correctly
				const currentHostIds = new Set(responses.map((r) => r.id));

				// Merge: keep data from other hosts, update current hosts
				queryClient.setQueryData<IPAddress[]>(queryKeys.ipAddresses.all, (old) => {
					if (!old) return allIPAddresses;
					const others = old.filter((i) => !currentHostIds.has(i.host_id));
					return [...others, ...allIPAddresses];
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

				queryClient.setQueryData<Interface[]>(queryKeys.interfaces.all, (old) => {
					if (!old) return allInterfaces;
					const others = old.filter((e) => !currentHostIds.has(e.host_id));
					return [...others, ...allInterfaces];
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
 * Options for {@link useHostSummariesQuery}. Deliberately narrower than
 * {@link HostQueryOptions}: a summary query is for picking and labelling hosts,
 * so it takes scoping and ordering but no offset paging.
 */
export interface HostSummaryQueryOptions {
	/** Filter by network ID. Prefer this over an org-wide query wherever the surface is network-scoped. */
	network_id?: string;
	/** Filter by specific host IDs. */
	ids?: string[];
	/** Filter by tag IDs (returns hosts that have ANY of the specified tags). */
	tag_ids?: string[];
	/** Maximum number of results. Defaults to 0 (no limit) — scope with `network_id` or `ids`. */
	limit?: number;
	/** As-of timestamp (ISO 8601) for a snapshot read instead of live state. */
	at?: string;
	/**
	 * Set false to hold the fetch — e.g. a picker inside a modal that is rendered
	 * unconditionally but not yet open. Excluded from the query key, so toggling it
	 * reuses the same cache entry rather than starting a new one.
	 */
	enabled?: boolean;
}

/**
 * Query hook for host identity only — no nested children.
 *
 * Use this for anything that needs hosts as *labels or options*: name lookups,
 * pickers, per-network lists. It requests `include_children=false`, so the
 * response carries the host row and its tags but not ip_addresses, ports,
 * services or interfaces — which are the bulk of a host payload.
 *
 * Deliberately a separate hook rather than an option on {@link useHostsQuery},
 * for two reasons:
 *
 *  1. `useHostsQuery` populates the ip-addresses/ports/services/interfaces
 *     caches as a side effect, and that write *replaces* every cached row for
 *     the hosts in its response. A children-free response run through it would
 *     silently delete real child rows for those hosts. This hook has no such
 *     side effect, which is only safe because it never claims to carry children.
 *  2. It keys off `['hosts','summary',…]`, so a summary result can never be
 *     mistaken for — or dedupe against — a full nested `['hosts','list',…]`
 *     entry that some other feature is relying on.
 *
 * If you need children, you need {@link useHostsQuery} (paginated) or
 * `GET /api/v1/hosts/{id}` for a single host.
 *
 * @param optionsOrGetter - Options, or a getter for reactive options.
 */
export function useHostSummariesQuery(
	optionsOrGetter: HostSummaryQueryOptions | (() => HostSummaryQueryOptions) = {}
) {
	return createQuery(() => {
		const { enabled = true, ...options } =
			typeof optionsOrGetter === 'function' ? optionsOrGetter() : optionsOrGetter;
		// An explicitly empty id list means "nothing to look up", not "fetch everything".
		const hasEmptyIdFilter = options.ids !== undefined && options.ids.length === 0;

		return {
			queryKey: [...queryKeys.hosts.all, 'summary', options],
			queryFn: async (): Promise<PaginatedResult<Host>> => {
				const { data } = await apiClient.GET('/api/v1/hosts', {
					params: {
						query: {
							// One network stays the ergonomic shape for a picker; the
							// wire param takes a list.
							network_ids: options.network_id ? [options.network_id] : undefined,
							ids: options.ids,
							tag_ids: options.tag_ids,
							limit: options.limit ?? 0,
							at: options.at,
							include_children: false
						}
					}
				});
				if (!data?.success || !data.data) {
					throw new Error(data?.error || 'Failed to fetch hosts');
				}

				return {
					items: data.data.map(toHostPrimitive),
					pagination: data.meta?.pagination ?? null
				};
			},
			enabled: enabled && !hasEmptyIdFilter,
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
							limit: 0, // No pagination when fetching by IDs
							// This hook returns primitives (`toHostPrimitive` drops the
							// children) and populates no child cache, so the nested
							// entities were downloaded and thrown away.
							include_children: false
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
			// credential_assignments changes are reflected on credentials' host_assignments
			queryClient.invalidateQueries({ queryKey: queryKeys.credentials.all });

			// Add children to their caches
			queryClient.setQueryData<IPAddress[]>(queryKeys.ipAddresses.all, (old) =>
				old ? [...old, ...response.ip_addresses] : response.ip_addresses
			);
			queryClient.setQueryData<Port[]>(queryKeys.ports.all, (old) =>
				old ? [...old, ...response.ports] : response.ports
			);
			queryClient.setQueryData<Service[]>(queryKeys.services.all, (old) =>
				old ? [...old, ...response.services] : response.services
			);
			queryClient.setQueryData<Interface[]>(queryKeys.interfaces.all, (old) =>
				old ? [...old, ...response.interfaces] : response.interfaces
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
				hostname: data.host.hostname ?? null,
				description: data.host.description,
				virtualization_metadata: data.host.virtualization_metadata,
				virtualization_service_id: data.host.virtualization_service_id,
				hidden: data.host.hidden,
				tags: data.host.tags,
				credential_assignments: data.host.credential_assignments ?? undefined,
				expected_updated_at: data.host.updated_at,
				// Only send arrays if provided (undefined = preserve existing)
				ip_addresses: data.ip_addresses
					? data.ip_addresses.map(
							(iface, index): IPAddressInput => ({
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
								virtualization_metadata: service.virtualization_metadata,
								virtualization_service_id: service.virtualization_service_id,
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
			// credential_assignments changes are reflected on credentials' host_assignments
			queryClient.invalidateQueries({ queryKey: queryKeys.credentials.all });

			// Replace ip addresses for this host
			queryClient.setQueryData<IPAddress[]>(queryKeys.ipAddresses.all, (old) => {
				const others = old?.filter((i) => i.host_id !== hostId) ?? [];
				return [...others, ...response.ip_addresses];
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

			// Replace interfaces for this host
			queryClient.setQueryData<Interface[]>(queryKeys.interfaces.all, (old) => {
				const others = old?.filter((e) => e.host_id !== hostId) ?? [];
				return [...others, ...response.interfaces];
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
					hostname: data.host.hostname ?? null,
					description: data.description,
					virtualization_metadata: data.host.virtualization_metadata,
					virtualization_service_id: data.host.virtualization_service_id,
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
			queryClient.setQueryData<IPAddress[]>(
				queryKeys.ipAddresses.all,
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
			queryClient.setQueryData<Interface[]>(
				queryKeys.interfaces.all,
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
			queryClient.setQueryData<IPAddress[]>(
				queryKeys.ipAddresses.all,
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
			queryClient.setQueryData<Interface[]>(
				queryKeys.interfaces.all,
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
			queryClient.setQueryData<IPAddress[]>(queryKeys.ipAddresses.all, (old) => {
				const others =
					old?.filter((i) => i.host_id !== otherHostId && i.host_id !== response.id) ?? [];
				return [...others, ...response.ip_addresses];
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
			queryClient.setQueryData<Interface[]>(queryKeys.interfaces.all, (old) => {
				const others =
					old?.filter((e) => e.host_id !== otherHostId && e.host_id !== response.id) ?? [];
				return [...others, ...response.interfaces];
			});

			if (otherHostName) {
				pushSuccess(
					hosts_consolidatedToast({
						source: otherHostName,
						destination: hostDisplayName(response)
					})
				);
			}
		}
	}));
}

/**
 * Rescan a single host.
 *
 * Returns a discovery session, so the result streams over the same SSE channel
 * as any other scan — seed the sessions cache and connect, or the UI won't see
 * it until the next poll. Refusals (never scanned, daemon gone, daemon too old,
 * daemon not on a subnet holding the host's IPs) come back as a 400 whose
 * message the API client toasts automatically.
 */
export function useRescanHostMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async ({ id }: { id: string; name?: string }) => {
			const { data } = await apiClient.POST('/api/v1/hosts/{id}/rescan', {
				params: { path: { id } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to start rescan');
			}
			return data.data as DiscoveryUpdatePayload;
		},
		onSuccess: (session: DiscoveryUpdatePayload, { name }) => {
			queryClient.setQueryData<DiscoveryUpdatePayload[]>(queryKeys.discovery.sessions(), (old) => {
				if (!old) return [session];
				const exists = old.find((s) => s.session_id === session.session_id);
				return exists
					? old.map((s) => (s.session_id === session.session_id ? session : s))
					: [...old, session];
			});

			discoverySSEManager.connect();

			pushSuccess(hosts_rescanStartedToast({ name: name ?? '' }));
		}
	}));
}

/**
 * Format an interface for display
 */
export function formatIPAddress(
	i: IPAddress | AllIPAddresses,
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
	const allIPAddresses = queryClient.getQueryData<IPAddress[]>(queryKeys.ipAddresses.all) ?? [];
	const allPorts = queryClient.getQueryData<Port[]>(queryKeys.ports.all) ?? [];
	const allServices = queryClient.getQueryData<Service[]>(queryKeys.services.all) ?? [];
	const allInterfaces = queryClient.getQueryData<Interface[]>(queryKeys.interfaces.all) ?? [];

	// Sort explicitly rather than trusting the cache's arrival order. The form's
	// ListManagers render these in array order and `useUpdateHostMutation` writes
	// `position: index` back from that order — so if the cache is ever populated
	// from a source with different ordering (the standalone child endpoints return
	// `created_at ASC`, not `position ASC`), opening a host and saving it would
	// silently rewrite and persist the user's ordering.
	return {
		...host,
		// `position` is optional on IPAddress; unordered entries sort first.
		ip_addresses: allIPAddresses
			.filter((i) => i.host_id === host.id)
			.toSorted((a, b) => (a.position ?? 0) - (b.position ?? 0)),
		ports: allPorts.filter((p) => p.host_id === host.id),
		services: allServices
			.filter((s) => s.host_id === host.id)
			.toSorted((a, b) => a.position - b.position),
		interfaces: allInterfaces
			.filter((e) => e.host_id === host.id)
			// Unread indexes sort last, matching the `if_index ASC` the server orders by: a port
			// learned from a neighbour's advertisement has none.
			.toSorted((a, b) => (a.if_index ?? Infinity) - (b.if_index ?? Infinity)),
		// SNMP fields from host
		sys_descr: host.sys_descr,
		sys_object_id: host.sys_object_id,
		sys_location: host.sys_location,
		sys_contact: host.sys_contact,
		management_url: host.management_url,
		chassis_id: host.chassis_id,
		sys_name: host.sys_name,
		// Hardware identity from host
		manufacturer: host.manufacturer,
		model: host.model,
		serial_number: host.serial_number,
		firmware_revision: host.firmware_revision,
		software_revision: host.software_revision,
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
		services: [],
		ip_addresses: [],
		ports: [],
		source: {
			type: 'Manual'
		},
		virtualization_metadata: null,
		virtualization_service_id: null,
		network_id: defaultNetworkId ?? '',
		hidden: false,
		// The discovered attributes are simply absent on a host nothing has scanned yet. They each
		// travel with the source that produced them, so there is no value here without one — which
		// is why these are omitted rather than set to null.
		credential_assignments: [],
		interfaces: []
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
export function getHostFromIPAddressIdFromCache(
	queryClient: ReturnType<typeof useQueryClient>,
	interfaceId: string
): Host | null {
	const interfaces = queryClient.getQueryData<Interface[]>(queryKeys.interfaces.all) ?? [];
	const iface = interfaces.find((i) => i.id === interfaceId);
	if (!iface) return null;

	return getHostByIdFromCache(queryClient, iface.host_id);
}
