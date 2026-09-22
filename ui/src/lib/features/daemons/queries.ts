/**
 * TanStack Query hooks for Daemons
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import { requireSuccess, unwrapData } from '$lib/api/query-helpers';
import type { components, paths } from '$lib/api/schema';
import type { Daemon } from './types/base';
import type { DiscoveryUpdatePayload } from '../discovery/types/api';
import type { ProvisionDaemonRequest, ProvisionDaemonResponse } from './types/base';
/**
 * Query hook for fetching all daemons
 * @param options.enabled - Optional getter function to control when query is enabled
 */
export function useDaemonsQuery(options?: { enabled?: () => boolean }) {
	return createQuery(() => ({
		queryKey: queryKeys.daemons.all,
		queryFn: async () => {
			return unwrapData(
				await apiClient.GET('/api/v1/daemons', {
					params: { query: { limit: 0 } }
				})
			);
		},
		enabled: options?.enabled?.() ?? true
	}));
}

/**
 * Query hook for fetching a single daemon by ID
 */
export function useDaemonQuery(id: () => string | null, options?: { enabled?: () => boolean }) {
	return createQuery(() => ({
		queryKey: queryKeys.daemons.detail(id() ?? ''),
		queryFn: async () => {
			const daemonId = id();
			if (!daemonId) throw new Error('No daemon ID');
			return unwrapData(
				await apiClient.GET('/api/v1/daemons/{id}', {
					params: { path: { id: daemonId } }
				})
			);
		},
		enabled: (options?.enabled?.() ?? true) && !!id()
	}));
}

/**
 * Query hook for a daemon's install command — the pure, idempotent builder. `purpose: 'install'`
 * returns a command with an `<API_KEY>` placeholder to fill in; `purpose: 'reconfigure'` returns
 * a credential-free command that re-asserts the server-held connectivity config. Never mints.
 */
export type InstallCommandParams = NonNullable<
	paths['/api/v1/daemons/{id}/install-command']['get']['parameters']['query']
>;

export function useDaemonInstallCommandQuery(
	id: () => string | null,
	params: () => InstallCommandParams,
	options?: { enabled?: () => boolean }
) {
	return createQuery(() => {
		const query = params();
		return {
			queryKey: [...queryKeys.daemons.detail(id() ?? ''), 'install-command', query],
			queryFn: async () => {
				const daemonId = id();
				if (!daemonId) throw new Error('No daemon ID');
				return unwrapData(
					await apiClient.GET('/api/v1/daemons/{id}/install-command', {
						params: { path: { id: daemonId }, query }
					})
				);
			},
			enabled: (options?.enabled?.() ?? true) && !!id()
		};
	});
}

/**
 * Mutation hook for updating a daemon's server-side record (name, maintainer, tags, and
 * the ServerPoll url). Identity and server-managed fields are restored server-side.
 */
export function useUpdateDaemonMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (daemon: Daemon) => {
			return unwrapData(
				await apiClient.PUT('/api/v1/daemons/{id}', {
					params: { path: { id: daemon.id } },
					body: daemon
				})
			);
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: queryKeys.daemons.all });
		}
	}));
}

/**
 * Mutation hook for deleting a daemon
 */
export function useDeleteDaemonMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			requireSuccess(
				await apiClient.DELETE('/api/v1/daemons/{id}', {
					params: { path: { id } }
				})
			);
			return id;
		},
		onSuccess: (id: string) => {
			queryClient.setQueryData<Daemon[]>(
				queryKeys.daemons.all,
				(old) => old?.filter((d) => d.id !== id) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for bulk deleting daemons
 */
export function useBulkDeleteDaemonsMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (ids: string[]) => {
			requireSuccess(await apiClient.POST('/api/v1/daemons/bulk-delete', { body: ids }));
			return ids;
		},
		onSuccess: (ids: string[]) => {
			queryClient.setQueryData<Daemon[]>(
				queryKeys.daemons.all,
				(old) => old?.filter((d) => !ids.includes(d.id)) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for provisioning a daemon (either mode) before install.
 * Creates the daemon record + its 1:1 API key server-side.
 */
export function useProvisionDaemonMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (request: ProvisionDaemonRequest): Promise<ProvisionDaemonResponse> => {
			return unwrapData(
				await apiClient.POST('/api/v1/daemons/provision', {
					body: request
				})
			);
		},
		onSuccess: (response: ProvisionDaemonResponse) => {
			// Re-provisioning returns an existing daemon, so replace it in place rather than
			// appending a duplicate; a fresh provision appends.
			queryClient.setQueryData<Daemon[]>(queryKeys.daemons.all, (old) => {
				const existing = old ?? [];
				return existing.some((d) => d.id === response.daemon.id)
					? existing.map((d) => (d.id === response.daemon.id ? response.daemon : d))
					: [...existing, response.daemon];
			});
		}
	}));
}

/**
 * Mutation hook for retrying connection to an unreachable daemon
 */
export function useRetryDaemonConnectionMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			requireSuccess(
				await apiClient.POST('/api/v1/daemons/{id}/retry-connection', {
					params: { path: { id } }
				})
			);
			return id;
		},
		onSuccess: (id: string) => {
			// Update the daemon in the cache to mark as reachable
			queryClient.setQueryData<Daemon[]>(
				queryKeys.daemons.all,
				(old) => old?.map((d) => (d.id === id ? { ...d, is_unreachable: false } : d)) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for testing daemon URL reachability
 */
export function useTestReachabilityMutation() {
	return createMutation(() => ({
		mutationFn: async (request: { url: string; check_health: boolean }) => {
			return unwrapData(
				await apiClient.POST('/api/v1/daemons/test-reachability', {
					body: request
				})
			);
		}
	}));
}

/**
 * Mutation to email the install command to the current user
 */
export function useEmailInstallCommandMutation() {
	return createMutation(() => ({
		mutationFn: async ({
			installCommand,
			os
		}: {
			installCommand: string;
			os: components['schemas']['DaemonOs'];
		}) => {
			requireSuccess(
				await apiClient.POST('/api/v1/daemons/email-install-command', {
					body: { install_command: installCommand, os }
				})
			);
		}
	}));
}

/**
 * Helper to check if a daemon is currently running a discovery session
 */
export function getDaemonIsRunningDiscovery(
	daemon_id: string | null,
	sessions: DiscoveryUpdatePayload[]
): boolean {
	if (!daemon_id) return false;

	// Find any active session for this daemon
	for (const session of sessions) {
		if (
			session.daemon_id === daemon_id &&
			(session.phase === 'Pending' ||
				session.phase === 'Starting' ||
				session.phase === 'Started' ||
				session.phase === 'Scanning')
		) {
			return true;
		}
	}
	return false;
}
