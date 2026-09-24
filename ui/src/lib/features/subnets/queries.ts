/**
 * TanStack Query hooks for Subnets
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import { requireSuccess, unwrapData } from '$lib/api/query-helpers';
import type { Subnet } from './types/base';

/**
 * Query hook for fetching all subnets
 */
/**
 * Subnets list. Called with no arguments this is the shared full-list cache
 * that topology, host cards and the like read, so any narrowing argument must
 * also change the query key — otherwise a filtered fetch would overwrite the
 * shared cache with a subset.
 */
export function useSubnetsQuery(
	atGetter?: () => string | undefined,
	staleGetter?: () => boolean | undefined
) {
	return createQuery(() => {
		const at = atGetter?.();
		const stale = staleGetter?.();
		const baseKey = at ? [...queryKeys.subnets.all, 'asOf', at] : queryKeys.subnets.all;
		return {
			queryKey: stale === undefined ? baseKey : [...baseKey, 'stale', stale],
			queryFn: async () => {
				return unwrapData(
					await apiClient.GET('/api/v1/subnets', {
						params: { query: { limit: 0, at, stale } }
					})
				);
			}
		};
	});
}

/**
 * Mutation hook for creating a subnet
 */
export function useCreateSubnetMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (subnet: Subnet) => {
			return unwrapData(await apiClient.POST('/api/v1/subnets', { body: subnet }));
		},
		onSuccess: (newSubnet: Subnet) => {
			queryClient.setQueryData<Subnet[]>(queryKeys.subnets.all, (old) =>
				old ? [...old, newSubnet] : [newSubnet]
			);
		}
	}));
}

/**
 * Mutation hook for updating a subnet
 */
/**
 * Fold a subnet into the range that contains it.
 *
 * The resolution for a range Scanopy assumed that a later reading turned out to cover. Discovery
 * corrects such a range on its own only where the answer is unambiguous; folding several assumed
 * ranges into one means deleting rows, so it is a person's call and this is where they make it.
 */
export function useMergeSubnetMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async ({ id, into }: { id: string; into: string }) => {
			return unwrapData(
				await apiClient.POST('/api/v1/subnets/{id}/merge', {
					params: { path: { id } },
					body: { into }
				})
			);
		},
		// The merged row is gone and the target may have gained addresses, so nothing local is
		// authoritative any more.
		onSuccess: () => queryClient.invalidateQueries({ queryKey: queryKeys.subnets.all })
	}));
}

export function useUpdateSubnetMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (subnet: Subnet) => {
			return unwrapData(
				await apiClient.PUT('/api/v1/subnets/{id}', {
					params: { path: { id: subnet.id } },
					body: subnet
				})
			);
		},
		onSuccess: (updatedSubnet: Subnet) => {
			queryClient.setQueryData<Subnet[]>(
				queryKeys.subnets.all,
				(old) => old?.map((s) => (s.id === updatedSubnet.id ? updatedSubnet : s)) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for deleting a subnet
 */
export function useDeleteSubnetMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			requireSuccess(
				await apiClient.DELETE('/api/v1/subnets/{id}', {
					params: { path: { id } }
				})
			);
			return id;
		},
		onSuccess: (id: string) => {
			queryClient.setQueryData<Subnet[]>(
				queryKeys.subnets.all,
				(old) => old?.filter((s) => s.id !== id) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for bulk deleting subnets
 */
export function useBulkDeleteSubnetsMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (ids: string[]) => {
			requireSuccess(await apiClient.POST('/api/v1/subnets/bulk-delete', { body: ids }));
			return ids;
		},
		onSuccess: (ids: string[]) => {
			queryClient.setQueryData<Subnet[]>(
				queryKeys.subnets.all,
				(old) => old?.filter((s) => !ids.includes(s.id)) ?? []
			);
		}
	}));
}

import { utcTimeZoneSentinel, uuidv4Sentinel } from '$lib/shared/utils/formatting';
import { subnetTypes } from '$lib/shared/stores/metadata';

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Check if a subnet is a container subnet (CIDR is 0.0.0.0/0 and source is System)
 */
export function isContainerSubnet(subnet: Subnet): boolean {
	return subnet.cidr === '0.0.0.0/0' && subnet.source.type === 'System';
}

/**
 * Whether this subnet belongs to the inventory the user curates, and so belongs in
 * the management lists (Subnets, Networks, Daemon and VLAN tabs).
 *
 * Mirrors `Subnet::is_user_managed` (`backend/src/server/subnets/impl/base.rs`), which
 * the dashboard's subnet count uses — the two must agree or the totals disagree with
 * the pages, as they did in GH #677.
 *
 * Provenance, not category: Scanopy fabricates the per-network `0.0.0.0/0` Internet and
 * Remote supernets and the loopback rows, and those stay out of the way; a subnet the
 * user created is theirs to manage whatever category they gave it.
 *
 * Fails open, like the metadata store it reads: a `subnet_type` this build doesn't
 * recognise yields no metadata, and an unrecognised subnet is shown rather than hidden.
 */
export function isUserManagedSubnet(subnet: Subnet): boolean {
	return (
		subnet.source.type === 'Manual' ||
		!subnetTypes.getMetadata(subnet.subnet_type).is_synthetic_category
	);
}

/**
 * Get a subnet by ID from a list of subnets
 */
export function getSubnetById(subnets: Subnet[], id: string): Subnet | null {
	return subnets.find((s) => s.id === id) ?? null;
}

/**
 * Get a subnet by ID from the cache
 */
export function getSubnetByIdFromCache(
	queryClient: ReturnType<typeof useQueryClient>,
	id: string
): Subnet | null {
	const subnets = queryClient.getQueryData<Subnet[]>(queryKeys.subnets.all) ?? [];
	return subnets.find((s) => s.id === id) ?? null;
}

/**
 * Create empty form data for a new subnet
 */
export function createEmptySubnetFormData(defaultNetworkId?: string): Subnet {
	return {
		id: uuidv4Sentinel,
		created_at: utcTimeZoneSentinel,
		updated_at: utcTimeZoneSentinel,
		// A range a person is about to type is theirs to assert; the server stamps this too when it
		// marks the row Manual, and setting it here keeps the form's own value honest in between.
		cidr_source: 'Manual',
		tags: [],
		name: '',
		network_id: defaultNetworkId ?? '',
		cidr: '',
		description: '',
		subnet_type: 'Unknown',
		virtualization_service_id: null,
		source: {
			type: 'Manual'
		}
	};
}
