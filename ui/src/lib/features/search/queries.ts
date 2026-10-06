/**
 * The four lookups behind the global search palette: hosts, services, subnets and VLANs, each searched on
 * the server and capped at a handful of rows.
 *
 * One query per entity rather than one combined query, each keyed under its entity's root key: a
 * host edit invalidates `['hosts']`, and the palette's host results go stale with everything else
 * that shows hosts instead of surviving on a key nothing invalidates. It also lets each section
 * render as soon as its own response lands.
 */

import { createQuery, keepPreviousData } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import { unwrapData } from '$lib/api/query-helpers';
import { toHostWithAddresses } from '$lib/features/hosts/queries';
import type { HostWithAddresses } from '$lib/features/hosts/types/base';
import type { Service } from '$lib/features/services/types/base';
import type { Subnet } from '$lib/features/subnets/types/base';
import type { Vlan } from '$lib/features/vlans/types/base';

/** Rows per section. The palette jumps to a match; browsing belongs to the entity's own tab. */
export const GLOBAL_SEARCH_LIMIT = 5;

/** The trimmed search, or `undefined` while there is nothing to search for. */
function searchTerm(getter: () => string): string | undefined {
	return getter().trim() || undefined;
}

export function useGlobalHostSearch(searchGetter: () => string) {
	return createQuery(() => {
		const search = searchTerm(searchGetter);
		return {
			queryKey: [...queryKeys.hosts.all, 'global-search', search],
			queryFn: async (): Promise<HostWithAddresses[]> => {
				const hosts = unwrapData(
					await apiClient.GET('/api/v1/hosts', {
						params: {
							query: {
								search,
								limit: GLOBAL_SEARCH_LIMIT,
								order_by: 'name',
								order_direction: 'asc',
								include_children: false
							}
						}
					})
				);
				return hosts.map(toHostWithAddresses);
			},
			enabled: search !== undefined,
			placeholderData: keepPreviousData
		};
	});
}

export function useGlobalServiceSearch(searchGetter: () => string) {
	return createQuery(() => {
		const search = searchTerm(searchGetter);
		return {
			queryKey: [...queryKeys.services.all, 'global-search', search],
			queryFn: async (): Promise<Service[]> =>
				unwrapData(
					await apiClient.GET('/api/v1/services', {
						params: {
							query: {
								search,
								limit: GLOBAL_SEARCH_LIMIT,
								order_by: 'name',
								order_direction: 'asc'
							}
						}
					})
				),
			enabled: search !== undefined,
			placeholderData: keepPreviousData
		};
	});
}

export function useGlobalSubnetSearch(searchGetter: () => string) {
	return createQuery(() => {
		const search = searchTerm(searchGetter);
		return {
			queryKey: [...queryKeys.subnets.all, 'global-search', search],
			queryFn: async (): Promise<Subnet[]> =>
				unwrapData(
					await apiClient.GET('/api/v1/subnets', {
						params: {
							query: {
								search,
								limit: GLOBAL_SEARCH_LIMIT,
								order_by: 'name',
								order_direction: 'asc'
							}
						}
					})
				),
			enabled: search !== undefined,
			placeholderData: keepPreviousData
		};
	});
}

export function useGlobalVlanSearch(searchGetter: () => string) {
	return createQuery(() => {
		const search = searchTerm(searchGetter);
		return {
			queryKey: [...queryKeys.vlans.all, 'global-search', search],
			queryFn: async (): Promise<Vlan[]> =>
				unwrapData(
					await apiClient.GET('/api/v1/vlans', {
						params: {
							query: {
								search,
								limit: GLOBAL_SEARCH_LIMIT,
								order_by: 'vlan_number',
								order_direction: 'asc'
							}
						}
					})
				),
			enabled: search !== undefined,
			placeholderData: keepPreviousData
		};
	});
}
