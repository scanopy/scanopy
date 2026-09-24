/**
 * TanStack Query hooks for Snapshots
 *
 * Snapshots capture point-in-time topology state for a network. Live view
 * is the topology row with `snapshot_id IS NULL`; each past snapshot has
 * its own topology row whose `snapshot_id` matches a row in this list.
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import { requireSuccess, unwrapData } from '$lib/api/query-helpers';
import { pushSuccess } from '$lib/shared/stores/feedback';
import { formatTimestamp } from '$lib/shared/utils/formatting';
import { topology_snapshotCreated } from '$lib/paraglide/messages';
import type { components } from '$lib/api/schema';

export type Snapshot = components['schemas']['Snapshot'];

/**
 * Query hook: list snapshots for a network, sorted by `taken_at DESC`.
 *
 * Pass a getter so the query refetches when the network selection changes.
 */
export function useSnapshotsQuery(networkId: () => string | undefined) {
	return createQuery(() => ({
		queryKey: queryKeys.snapshots.byNetwork(networkId() ?? ''),
		queryFn: async () => {
			const id = networkId();
			if (!id) return [] as Snapshot[];
			const snapshots = unwrapData(
				await apiClient.GET('/api/v1/snapshots', {
					params: { query: { network_id: id, limit: 0 } }
				})
			);
			return [...snapshots].sort(
				(a, b) => new Date(b.taken_at).getTime() - new Date(a.taken_at).getTime()
			);
		},
		enabled: () => !!networkId()
	}));
}

/**
 * Mutation hook: capture a new snapshot for the given network.
 *
 * On success: invalidates the per-network snapshots list and the topology
 * list (the backend's snapshot subscriber inserts a topology row for the
 * new snapshot — clients refetch to pick it up).
 *
 * Error toasts (e.g. 409 when discovery is in flight) are surfaced by the
 * apiClient error middleware — no per-mutation onError handler needed.
 */
export function useTakeSnapshotMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async ({ network_id }: { network_id: string }) => {
			return unwrapData(
				await apiClient.POST('/api/v1/snapshots', {
					body: { network_id }
				})
			);
		},
		onSuccess: (snapshot: Snapshot) => {
			queryClient.invalidateQueries({
				queryKey: queryKeys.snapshots.byNetwork(snapshot.network_id)
			});
			queryClient.invalidateQueries({ queryKey: queryKeys.topology.all });
			pushSuccess(topology_snapshotCreated({ time: formatTimestamp(snapshot.taken_at) }));
		}
	}));
}

/**
 * Mutation hook: delete a snapshot. The backend's CASCADE FKs reap the
 * snapshot's topology row and all closed entity rows tied to it.
 */
export function useDeleteSnapshotMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async ({ snapshot_id }: { snapshot_id: string; network_id: string }) => {
			requireSuccess(
				await apiClient.DELETE('/api/v1/snapshots/{id}', {
					params: { path: { id: snapshot_id } }
				})
			);
			return snapshot_id;
		},
		onSuccess: (_id, variables) => {
			queryClient.invalidateQueries({
				queryKey: queryKeys.snapshots.byNetwork(variables.network_id)
			});
			queryClient.invalidateQueries({ queryKey: queryKeys.topology.all });
		}
	}));
}
