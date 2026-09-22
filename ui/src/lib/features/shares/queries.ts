/**
 * TanStack Query hooks for Shares
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import { requireSuccess, unwrapData } from '$lib/api/query-helpers';
import type { Share, CreateUpdateShareRequest } from './types/base';

/**
 * Query hook for fetching all shares
 */
export function useSharesQuery() {
	return createQuery(() => ({
		queryKey: queryKeys.shares.all,
		queryFn: async () => {
			return unwrapData(
				await apiClient.GET('/api/v1/shares', {
					params: { query: { limit: 0 } }
				})
			);
		}
	}));
}

/**
 * Mutation hook for creating a share
 */
export function useCreateShareMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (request: CreateUpdateShareRequest) => {
			return unwrapData(await apiClient.POST('/api/v1/shares', { body: request }));
		},
		onSuccess: (newShare: Share) => {
			queryClient.setQueryData<Share[]>(queryKeys.shares.all, (old) =>
				old ? [...old, newShare] : [newShare]
			);
		}
	}));
}

/**
 * Mutation hook for updating a share
 */
export function useUpdateShareMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async ({ id, request }: { id: string; request: CreateUpdateShareRequest }) => {
			return unwrapData(
				await apiClient.PUT('/api/v1/shares/{id}', {
					params: { path: { id } },
					body: request
				})
			);
		},
		onSuccess: (updatedShare: Share) => {
			queryClient.setQueryData<Share[]>(
				queryKeys.shares.all,
				(old) => old?.map((s) => (s.id === updatedShare.id ? updatedShare : s)) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for deleting a share
 */
export function useDeleteShareMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			requireSuccess(
				await apiClient.DELETE('/api/v1/shares/{id}', {
					params: { path: { id } }
				})
			);
			return id;
		},
		onSuccess: (id: string) => {
			queryClient.setQueryData<Share[]>(
				queryKeys.shares.all,
				(old) => old?.filter((s) => s.id !== id) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for bulk deleting shares
 */
export function useBulkDeleteSharesMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (ids: string[]) => {
			requireSuccess(await apiClient.POST('/api/v1/shares/bulk-delete', { body: ids }));
			return ids;
		},
		onSuccess: (ids: string[]) => {
			queryClient.setQueryData<Share[]>(
				queryKeys.shares.all,
				(old) => old?.filter((s) => !ids.includes(s.id)) ?? []
			);
		}
	}));
}

import type { PublicShareMetadata, ShareWithTopology } from './types/base';

// ============================================================================
// Public API Functions (no auth required)
// ============================================================================

/**
 * Fetch public share metadata
 */
export async function getPublicShareMetadata(
	shareId: string
): Promise<{ success: boolean; data?: PublicShareMetadata; error?: string }> {
	try {
		const response = await fetch(`/api/v1/shares/public/${shareId}`, {
			method: 'GET',
			headers: {
				'Content-Type': 'application/json'
			}
		});

		const result = await response.json();

		if (!response.ok || result.error) {
			return { success: false, error: result.error || 'Failed to fetch share' };
		}

		return { success: true, data: result.data };
	} catch {
		return { success: false, error: 'Failed to fetch share' };
	}
}

/**
 * Verify share password and exchange it for a server-issued access token.
 *
 * The returned `access_token` is an opaque string (HS256 JWT) tied to the
 * share's current password hash. Store it and send it on subsequent
 * `/topology` requests in place of the raw password.
 */
export async function verifySharePassword(
	shareId: string,
	password: string
): Promise<{
	success: boolean;
	access_token?: string;
	expires_at?: string;
	error?: string;
}> {
	try {
		const response = await fetch(`/api/v1/shares/public/${shareId}/verify`, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json'
			},
			body: JSON.stringify(password)
		});

		const result = await response.json();

		if (!response.ok || result.error) {
			return { success: false, error: result.error || 'Invalid password' };
		}

		return {
			success: true,
			access_token: result.data?.access_token,
			expires_at: result.data?.expires_at
		};
	} catch {
		return { success: false, error: 'Failed to verify password' };
	}
}

/**
 * Fetch public share topology. Pass the `access_token` obtained from
 * `verifySharePassword` for password-protected shares. On 401 (expired or
 * tampered token), `code === 'share_token_invalid'` is set so callers can
 * clear the stored token and re-prompt.
 */
export async function getPublicShareTopology(
	shareId: string,
	options: { embed?: boolean; access_token?: string; view: string }
): Promise<{
	success: boolean;
	data?: ShareWithTopology;
	error?: string;
	code?: string;
}> {
	try {
		const url = options.embed
			? `/api/v1/shares/public/${shareId}/topology?embed=true`
			: `/api/v1/shares/public/${shareId}/topology`;
		const response = await fetch(url, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json'
			},
			body: JSON.stringify({ access_token: options.access_token, view: options.view })
		});

		const result = await response.json();

		if (!response.ok || result.error) {
			return {
				success: false,
				error: result.error || 'Failed to fetch topology',
				code: result.code
			};
		}

		return { success: true, data: result.data };
	} catch {
		return { success: false, error: 'Failed to fetch topology' };
	}
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Generate share URL
 */
export function generateShareUrl(shareId: string, theme?: 'light' | 'dark'): string {
	const suffix = theme ? `?theme=${theme}` : '';
	if (typeof window !== 'undefined') {
		return `${window.location.origin}/share/${shareId}${suffix}`;
	}
	return `/share/${shareId}${suffix}`;
}

/**
 * Generate embed URL for a share
 */
export function generateEmbedUrl(shareId: string, theme?: 'light' | 'dark'): string {
	const suffix = theme ? `?theme=${theme}` : '';
	if (typeof window !== 'undefined') {
		return `${window.location.origin}/share/${shareId}/embed${suffix}`;
	}
	return `/share/${shareId}/embed${suffix}`;
}

/**
 * Generate embed code for a share
 */
export function generateEmbedCode(
	shareId: string,
	width: string | number = '100%',
	height: string | number = '600px',
	theme?: 'light' | 'dark'
): string {
	const embedUrl = generateEmbedUrl(shareId, theme);
	const widthStr = typeof width === 'number' ? `${width}px` : width;
	const heightStr = typeof height === 'number' ? `${height}px` : height;
	const borderColor = theme === 'light' ? '#e2e8f0' : '#374151';
	return `<iframe src="${embedUrl}" width="${widthStr}" height="${heightStr}" frameborder="0" style="border: 1px solid ${borderColor}; border-radius: 8px;"></iframe>`;
}

/**
 * Store share access token in session storage.
 *
 * The token (not the raw password) is what's persisted across view
 * switches and iframe reloads. Changing the share password invalidates
 * all outstanding tokens server-side.
 */
export function storeShareAccessToken(shareId: string, token: string): void {
	if (typeof window !== 'undefined') {
		sessionStorage.setItem(`share_token_${shareId}`, token);
	}
}

/**
 * Get stored share access token from session storage
 */
export function getStoredShareAccessToken(shareId: string): string | null {
	if (typeof window !== 'undefined') {
		return sessionStorage.getItem(`share_token_${shareId}`);
	}
	return null;
}

/**
 * Drop the stored access token, e.g. after a 401 from the topology endpoint.
 */
export function clearStoredShareAccessToken(shareId: string): void {
	if (typeof window !== 'undefined') {
		sessionStorage.removeItem(`share_token_${shareId}`);
	}
}
