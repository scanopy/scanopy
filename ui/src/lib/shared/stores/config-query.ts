/**
 * TanStack Query hook for server configuration
 */

import { createQuery } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import type { components } from '$lib/api/schema';

export type OidcProviderMetadata = components['schemas']['OidcProviderMetadata'];
export type DeploymentType = components['schemas']['DeploymentType'];
export type PublicServerConfig = components['schemas']['PublicConfigResponse'];

export const isLicenseLocked = (cfg: PublicServerConfig) =>
	cfg.license_status === 'expired' ||
	cfg.license_status === 'invalid' ||
	cfg.license_status === 'pending';

/**
 * Soft-warning threshold: show an "approaching expiry" banner when the
 * user-visible expiry is within this many days of now. Pre-grace only —
 * once past `intended_exp` the grace banner takes over.
 */
const APPROACHING_EXPIRY_DAYS = 7;

/**
 * True when the license is valid, not yet in grace, and within
 * `APPROACHING_EXPIRY_DAYS` of its user-visible expiry. The backend does
 * not emit this flag because it's a UX threshold we want to tune without
 * a server release.
 */
export const isLicenseApproachingExpiry = (cfg: PublicServerConfig): boolean => {
	if (cfg.license_status !== 'valid') return false;
	if (cfg.license_in_grace_period) return false;
	const intended = cfg.license_intended_expiry;
	if (!intended) return false;
	const intendedMs = Date.parse(intended);
	if (Number.isNaN(intendedMs)) return false;
	const msPerDay = 1000 * 60 * 60 * 24;
	const daysUntil = (intendedMs - Date.now()) / msPerDay;
	return daysUntil >= 0 && daysUntil <= APPROACHING_EXPIRY_DAYS;
};

// Helper functions for deployment type checks
export const isCloud = (cfg: PublicServerConfig) => cfg.deployment_type === 'cloud';
export const isCommercial = (cfg: PublicServerConfig) => cfg.deployment_type === 'commercial';
export const isCommunity = (cfg: PublicServerConfig) => cfg.deployment_type === 'community';
export const isSelfHosted = (cfg: PublicServerConfig) =>
	cfg.deployment_type === 'commercial' || cfg.deployment_type === 'community';

/**
 * Query hook for fetching server configuration
 */
export function useConfigQuery() {
	return createQuery(() => ({
		queryKey: queryKeys.config.all,
		queryFn: async () => {
			const { data } = await apiClient.GET('/api/config', {});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch config');
			}
			return data.data as PublicServerConfig;
		},
		staleTime: Infinity, // Config rarely changes
		gcTime: Infinity
	}));
}
