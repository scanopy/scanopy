import type { components } from '$lib/api/schema';
import {
	common_entityName,
	common_internalIt,
	common_home,
	common_homelab,
	common_site,
	common_organization,
	common_other,
	onboarding_internalItDescription,
	onboarding_internalItSiteHelp,
	onboarding_internalItSiteLabel,
	onboarding_internalItSitePlaceholder,
	onboarding_internalItOrgPlaceholder,
	onboarding_homelabDescription,
	onboarding_homelabSiteHelp,
	onboarding_homelabOrgLabel,
	onboarding_homelabOrgPlaceholder,
	onboarding_mspDescription,
	onboarding_mspLabel,
	onboarding_mspSiteHelp,
	onboarding_mspSiteLabel,
	onboarding_mspSitePlaceholder,
	onboarding_mspOrgLabel,
	onboarding_mspOrgPlaceholder,
	onboarding_orgHelp,
	onboarding_otherDescription,
	onboarding_otherSitePlaceholder,
	onboarding_otherOrgPlaceholder
} from '$lib/paraglide/messages';

// Re-export generated types
export type LoginRequest = components['schemas']['LoginRequest'];
export type RegisterRequest = components['schemas']['RegisterRequest'];
export type SetupRequest = components['schemas']['SetupRequest'];
export type SetupResponse = components['schemas']['SetupResponse'];
export type ForgotPasswordRequest = components['schemas']['ForgotPasswordRequest'];
export type ResetPasswordRequest = components['schemas']['ResetPasswordRequest'];
export type VerifyEmailRequest = components['schemas']['VerifyEmailRequest'];
export type ResendVerificationRequest = components['schemas']['ResendVerificationRequest'];

// SiteSetup extended with optional id (assigned after setup API returns site_ids)
export type SiteSetup = components['schemas']['SiteSetup'] & {
	id?: string;
};

// Frontend-only types (not in backend)
export interface SessionUser {
	user_id: string;
	name: string;
}

// Onboarding use case types — derived from backend-generated schema
export type UseCase = components['schemas']['UseCase'];

// Consolidated use case configuration
// Icons are mapped separately in components (Svelte component references)
export interface UseCaseConfig {
	label: string;
	description: string;
	orgLabel: string;
	orgPlaceholder: string;
	orgHelp: string;
	siteLabel: string;
	sitePlaceholder: string;
	siteHelp: string;
	colors: {
		ring: string;
		bg: string;
		text: string;
	};
}

export function getUseCases(): Record<UseCase, UseCaseConfig> {
	return {
		internal_it: {
			label: common_internalIt(),
			description: onboarding_internalItDescription(),
			orgLabel: common_entityName({ entity: common_organization() }),
			orgPlaceholder: onboarding_internalItOrgPlaceholder(),
			orgHelp: onboarding_orgHelp(),
			siteLabel: onboarding_internalItSiteLabel(),
			sitePlaceholder: onboarding_internalItSitePlaceholder(),
			siteHelp: onboarding_internalItSiteHelp(),
			colors: {
				ring: 'ring-blue-500',
				bg: 'bg-blue-500/20',
				text: 'text-blue-400'
			}
		},
		homelab: {
			label: common_homelab(),
			description: onboarding_homelabDescription(),
			orgLabel: onboarding_homelabOrgLabel(),
			orgPlaceholder: onboarding_homelabOrgPlaceholder(),
			orgHelp: onboarding_orgHelp(),
			siteLabel: common_entityName({ entity: common_site() }),
			sitePlaceholder: common_home(),
			siteHelp: onboarding_homelabSiteHelp(),
			colors: {
				ring: 'ring-emerald-500',
				bg: 'bg-emerald-500/20',
				text: 'text-emerald-400'
			}
		},
		msp: {
			label: onboarding_mspLabel(),
			description: onboarding_mspDescription(),
			orgLabel: onboarding_mspOrgLabel(),
			orgPlaceholder: onboarding_mspOrgPlaceholder(),
			orgHelp: onboarding_orgHelp(),
			siteLabel: onboarding_mspSiteLabel(),
			sitePlaceholder: onboarding_mspSitePlaceholder(),
			siteHelp: onboarding_mspSiteHelp(),
			colors: {
				ring: 'ring-violet-500',
				bg: 'bg-violet-500/20',
				text: 'text-violet-400'
			}
		},
		other: {
			label: common_other(),
			description: onboarding_otherDescription(),
			orgLabel: common_entityName({ entity: common_organization() }),
			orgPlaceholder: onboarding_otherOrgPlaceholder(),
			orgHelp: onboarding_orgHelp(),
			siteLabel: common_entityName({ entity: common_site() }),
			sitePlaceholder: onboarding_otherSitePlaceholder(),
			siteHelp: onboarding_homelabSiteHelp(),
			colors: {
				ring: 'ring-amber-500',
				bg: 'bg-amber-500/20',
				text: 'text-amber-400'
			}
		}
	};
}
