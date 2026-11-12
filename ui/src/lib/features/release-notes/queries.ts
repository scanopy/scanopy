/**
 * TanStack Query hooks for the release notes modal
 */

import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '#lib/api/query-client.ts';
import { apiClient } from '#lib/api/client.ts';
import { requireSuccess, unwrapData } from '#lib/api/query-helpers.ts';
import type { components } from '#lib/api/schema';

export type UnseenReleaseNotes = components['schemas']['UnseenReleaseNotes'];

/**
 * The releases the current user hasn't dismissed, up to the server's version.
 * Fetched once per page load: the list only changes when the server upgrades.
 */
export function useUnseenReleaseNotesQuery(options: { enabled: () => boolean }) {
	return createQuery(() => ({
		queryKey: queryKeys.releaseNotes.unseen(),
		queryFn: async () => unwrapData(await apiClient.GET('/api/v1/release-notes/unseen')),
		enabled: options.enabled(),
		staleTime: Infinity
	}));
}

/** Dismiss every release up to the server's version, for this user on every device. */
export function useMarkReleaseNotesSeenMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async () => {
			requireSuccess(await apiClient.POST('/api/v1/release-notes/seen'));
		},
		onSuccess: () => {
			queryClient.setQueryData<UnseenReleaseNotes>(queryKeys.releaseNotes.unseen(), {
				releases: [],
				changelog_url: null,
				auto_open: false
			});
		}
	}));
}
