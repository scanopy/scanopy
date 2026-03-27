import type { components } from '$lib/api/schema';
import {
	common_company,
	common_homelab,
	common_other,
	onboarding_companyDescription,
	onboarding_companyNetworkHelp,
	onboarding_companyNetworkLabel,
	onboarding_companyNetworkPlaceholder,
	onboarding_companyOrgLabel,
	onboarding_companyOrgPlaceholder,
	onboarding_homelabDescription,
	onboarding_homelabNetworkHelp,
	onboarding_homelabNetworkLabel,
	onboarding_homelabNetworkPlaceholder,
	onboarding_homelabOrgLabel,
	onboarding_homelabOrgPlaceholder,
	onboarding_mspDescription,
	onboarding_mspLabel,
	onboarding_mspNetworkHelp,
	onboarding_mspNetworkLabel,
	onboarding_mspNetworkPlaceholder,
	onboarding_mspOrgLabel,
	onboarding_mspOrgPlaceholder,
	onboarding_orgHelp,
	onboarding_otherDescription,
	onboarding_otherNetworkPlaceholder,
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

// NetworkSetup extended with optional id (assigned after setup API returns network_ids)
export type NetworkSetup = components['schemas']['NetworkSetup'] & {
	id?: string;
};

// Frontend-only types (not in backend)
export interface SessionUser {
	user_id: string;
	name: string;
}

// Onboarding use case types
export type UseCase = 'homelab' | 'company' | 'msp' | 'other';

// Consolidated use case configuration
// Icons are mapped separately in components (Svelte component references)
export interface UseCaseConfig {
	label: string;
	description: string;
	orgLabel: string;
	orgPlaceholder: string;
	orgHelp: string;
	networkLabel: string;
	networkPlaceholder: string;
	networkHelp: string;
	colors: {
		ring: string;
		bg: string;
		text: string;
	};
}

export function getUseCases(): Record<UseCase, UseCaseConfig> {
	return {
		homelab: {
			label: common_homelab(),
			description: onboarding_homelabDescription(),
			orgLabel: onboarding_homelabOrgLabel(),
			orgPlaceholder: onboarding_homelabOrgPlaceholder(),
			orgHelp: onboarding_orgHelp(),
			networkLabel: onboarding_homelabNetworkLabel(),
			networkPlaceholder: onboarding_homelabNetworkPlaceholder(),
			networkHelp: onboarding_homelabNetworkHelp(),
			colors: {
				ring: 'ring-emerald-500',
				bg: 'bg-emerald-500/20',
				text: 'text-emerald-400'
			}
		},
		company: {
			label: common_company(),
			description: onboarding_companyDescription(),
			orgLabel: onboarding_companyOrgLabel(),
			orgPlaceholder: onboarding_companyOrgPlaceholder(),
			orgHelp: onboarding_orgHelp(),
			networkLabel: onboarding_companyNetworkLabel(),
			networkPlaceholder: onboarding_companyNetworkPlaceholder(),
			networkHelp: onboarding_companyNetworkHelp(),
			colors: {
				ring: 'ring-blue-500',
				bg: 'bg-blue-500/20',
				text: 'text-blue-400'
			}
		},
		msp: {
			label: onboarding_mspLabel(),
			description: onboarding_mspDescription(),
			orgLabel: onboarding_mspOrgLabel(),
			orgPlaceholder: onboarding_mspOrgPlaceholder(),
			orgHelp: onboarding_orgHelp(),
			networkLabel: onboarding_mspNetworkLabel(),
			networkPlaceholder: onboarding_mspNetworkPlaceholder(),
			networkHelp: onboarding_mspNetworkHelp(),
			colors: {
				ring: 'ring-violet-500',
				bg: 'bg-violet-500/20',
				text: 'text-violet-400'
			}
		},
		other: {
			label: common_other(),
			description: onboarding_otherDescription(),
			orgLabel: onboarding_companyOrgLabel(),
			orgPlaceholder: onboarding_otherOrgPlaceholder(),
			orgHelp: onboarding_orgHelp(),
			networkLabel: onboarding_homelabNetworkLabel(),
			networkPlaceholder: onboarding_otherNetworkPlaceholder(),
			networkHelp: onboarding_homelabNetworkHelp(),
			colors: {
				ring: 'ring-amber-500',
				bg: 'bg-amber-500/20',
				text: 'text-amber-400'
			}
		}
	};
}
