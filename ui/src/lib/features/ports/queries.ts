/**
 * TanStack Query hooks for Ports
 *
 * Ports are child entities populated by the hosts query.
 * This file provides read-only access to the ports cache.
 */

import { createQuery, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '#lib/api/query-client.ts';
import type { Port } from '#lib/features/hosts/types/base.ts';

// Re-export type for convenience
export type { Port };

/**
 * Query hook for accessing the ports cache
 * This cache is populated by useHostsQuery - it does not fetch directly
 */
export function usePortsQuery() {
	return createQuery(() => ({
		queryKey: queryKeys.ports.all,
		initialData: [] as Port[],
		staleTime: Infinity,
		refetchOnMount: false,
		refetchOnWindowFocus: false,
		enabled: false
	}));
}

/**
 * Get ports for a specific host from the cache
 */
export function getPortsForHostFromCache(
	queryClient: ReturnType<typeof useQueryClient>,
	hostId: string
): Port[] {
	const ports = queryClient.getQueryData<Port[]>(queryKeys.ports.all) ?? [];
	return ports.filter((p) => p.host_id === hostId);
}

/**
 * Get a single port by ID from the cache
 */
export function getPortByIdFromCache(
	queryClient: ReturnType<typeof useQueryClient>,
	id: string
): Port | null {
	const ports = queryClient.getQueryData<Port[]>(queryKeys.ports.all) ?? [];
	return ports.find((p) => p.id === id) ?? null;
}
