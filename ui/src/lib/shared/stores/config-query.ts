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
	cfg.license_status === 'expired' || cfg.license_status === 'invalid';

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
 * True when this server holds a license signing key. Without one the mint
 * endpoints fail, so the License tab and the self-hosted plans are hidden.
 */
export const isLicenseSigningAvailable = (cfg: PublicServerConfig) => cfg.license_signing_available;

/**
 * Days a minted key stays valid past `license_paid_through` before its
 * user-visible expiry, and the further grace days before it hard-stops. Both
 * come from the server so the mint and the UI cannot drift apart.
 */
export const licenseKeyExpiry = (cfg: PublicServerConfig, paidThrough: string): Date =>
	addDays(paidThrough, cfg.license_key_buffer_days);

/** The date a key stops working outright: expiry plus the grace window. */
export const licenseKeyHardStop = (cfg: PublicServerConfig, paidThrough: string): Date =>
	addDays(paidThrough, cfg.license_key_buffer_days + cfg.license_key_grace_days);

/**
 * Days between `license_paid_through` (when an air-gapped org may switch back to
 * an online key) and that key's hard stop.
 */
export const licenseKeySwitchBackWindowDays = (cfg: PublicServerConfig) =>
	cfg.license_key_buffer_days + cfg.license_key_grace_days;

const addDays = (from: string, days: number): Date => {
	const date = new Date(from);
	date.setDate(date.getDate() + days);
	return date;
};

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
