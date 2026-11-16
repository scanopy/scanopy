/**
 * TanStack Query hooks for VLANs
 *
 * VLANs are populated by SNMP discovery, and a person can create one discovery hasn't found
 * so a subnet can be linked to it.
 */

import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '#lib/api/query-client.ts';
import { apiClient } from '#lib/api/client.ts';
import { requireSuccess, unwrapData } from '#lib/api/query-helpers.ts';
import type { Vlan } from './types/base';
import { invalidateListPages, useListPageQuery, type ListPageParams } from '#lib/api/list-page.ts';
import { utcTimeZoneSentinel, uuidv4Sentinel } from '#lib/shared/utils/formatting.ts';

/**
 * VLANs list. Called with no arguments this is the shared full-list cache, so
 * any narrowing argument must also change the query key — otherwise a filtered
 * fetch would overwrite the shared cache with a subset.
 */
export function useVlansQuery(atGetter?: () => string | undefined) {
	return createQuery(() => {
		const at = atGetter?.();
		return {
			queryKey: at ? [...queryKeys.vlans.all, 'asOf', at] : queryKeys.vlans.all,
			queryFn: async () => {
				return unwrapData(
					await apiClient.GET('/api/v1/vlans', {
						params: { query: { limit: 0, at } }
					})
				);
			}
		};
	});
}

/** One page of the VLANs list, kept apart from the full list other consumers read. */
export function useVlansListQuery(params: () => ListPageParams) {
	return useListPageQuery<Vlan>(
		queryKeys.vlans.all,
		({ query, querySerializer }) =>
			apiClient.GET('/api/v1/vlans', { params: { query }, querySerializer }),
		params
	);
}

/**
 * Mutation hook for creating a VLAN
 */
export function useCreateVlanMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (vlan: Vlan) => {
			return unwrapData(await apiClient.POST('/api/v1/vlans', { body: vlan }));
		},
		onSuccess: () => queryClient.invalidateQueries({ queryKey: queryKeys.vlans.all })
	}));
}

/** A VLAN a person is about to create. The server stamps its name `Manual` and its source. */
export function createEmptyVlanFormData(siteId: string, organizationId: string): Vlan {
	return {
		id: uuidv4Sentinel,
		created_at: utcTimeZoneSentinel,
		updated_at: utcTimeZoneSentinel,
		vlan_number: 1,
		name: '',
		name_source: 'Manual',
		description: null,
		site_id: siteId,
		organization_id: organizationId,
		source: { type: 'Manual' },
		tags: []
	};
}

/**
 * Mutation hook for updating a VLAN
 */
export function useUpdateVlanMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (vlan: Vlan) => {
			return unwrapData(
				await apiClient.PUT('/api/v1/vlans/{id}', {
					params: { path: { id: vlan.id } },
					body: vlan
				})
			);
		},
		onSuccess: (updatedVlan: Vlan) => {
			queryClient.setQueryData<Vlan[]>(
				queryKeys.vlans.all,
				(old) => old?.map((v) => (v.id === updatedVlan.id ? updatedVlan : v)) ?? []
			);
			invalidateListPages(queryClient, queryKeys.vlans.all);
		}
	}));
}

/**
 * Mutation hook for deleting a VLAN
 */
export function useDeleteVlanMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			requireSuccess(
				await apiClient.DELETE('/api/v1/vlans/{id}', {
					params: { path: { id } }
				})
			);
			return id;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: queryKeys.vlans.all });
		}
	}));
}

/**
 * Mutation hook for bulk deleting VLANs
 */
export function useBulkDeleteVlansMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (ids: string[]) => {
			requireSuccess(await apiClient.POST('/api/v1/vlans/bulk-delete', { body: ids }));
			return ids;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: queryKeys.vlans.all });
		}
	}));
}
