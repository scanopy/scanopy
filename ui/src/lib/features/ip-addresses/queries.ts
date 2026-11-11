/**
 * TanStack Query hooks for Interfaces
 *
 * Interfaces are child entities populated by the hosts query.
 * This file provides read-only access to the interfaces cache.
 */

import { createQuery, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '#lib/api/query-client.ts';
import type { IPAddress } from '#lib/features/hosts/types/base.ts';

// Re-export type for convenience
export type { IPAddress };

/**
 * Query hook for accessing the interfaces cache
 * This cache is populated by useHostsQuery - it does not fetch directly
 */
export function useIPAddressesQuery() {
	return createQuery(() => ({
		queryKey: queryKeys.ipAddresses.all,
		initialData: [] as IPAddress[],
		staleTime: Infinity,
		refetchOnMount: false,
		refetchOnWindowFocus: false,
		enabled: false
	}));
}

/**
 * Get interfaces for a specific host from the cache
 */
export function getIPAddressesForHostFromCache(
	queryClient: ReturnType<typeof useQueryClient>,
	hostId: string
): IPAddress[] {
	const interfaces = queryClient.getQueryData<IPAddress[]>(queryKeys.ipAddresses.all) ?? [];
	return interfaces.filter((i) => i.host_id === hostId);
}

/**
 * Get interfaces for a specific subnet from the cache
 */
export function getIPAddressesForSubnetFromCache(
	queryClient: ReturnType<typeof useQueryClient>,
	subnetId: string
): IPAddress[] {
	const interfaces = queryClient.getQueryData<IPAddress[]>(queryKeys.ipAddresses.all) ?? [];
	return interfaces.filter((i) => i.subnet_id === subnetId);
}

/**
 * Get a single interface by ID from the cache
 */
export function getIPAddressByIdFromCache(
	queryClient: ReturnType<typeof useQueryClient>,
	id: string
): IPAddress | null {
	const interfaces = queryClient.getQueryData<IPAddress[]>(queryKeys.ipAddresses.all) ?? [];
	return interfaces.find((i) => i.id === id) ?? null;
}
