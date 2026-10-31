/**
 * TanStack Query hooks for Authentication
 */

import {
	createQuery,
	createMutation,
	useQueryClient,
	type QueryClient
} from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import { requireSuccess, unwrapData, unwrapEnvelope } from '$lib/api/query-helpers';
import { checkServerVersion } from '$lib/api/server-version';
import { pushSuccess } from '$lib/shared/stores/feedback';
import {
	auth_emailVerified,
	auth_loggedOut,
	auth_passwordHasBeenReset,
	auth_passwordResetLinkSent,
	auth_verificationEmailSent,
	auth_welcome,
	auth_welcomeBack
} from '$lib/paraglide/messages';
import { resetIdentity } from '$lib/shared/utils/analytics';
import { useConfigQuery } from '$lib/shared/stores/config-query';
import type { User } from '../users/types';
import type { components } from '$lib/api/schema';
import type {
	ForgotPasswordRequest,
	LoginRequest,
	RegisterRequest,
	ResendVerificationRequest,
	ResetPasswordRequest,
	SetupRequest,
	SetupResponse,
	VerifyEmailRequest
} from './types/base';

export type CurrentUser = components['schemas']['CurrentUser'];

/**
 * The signed-in user, or `null` when the server says there is none (401).
 *
 * Any other failure throws: a server that is down or restarting says nothing about the session,
 * and caching `null` for it reads as a logout, which clears every query in the app.
 */
export async function fetchCurrentUser() {
	const result = await apiClient.POST('/api/auth/me', {});
	if (result.response.status === 401) {
		return null;
	}
	const envelope = unwrapEnvelope(result);
	checkServerVersion(envelope.meta.server_version);
	return envelope.data;
}

/**
 * Loads a new session's user and grants. The endpoints that start a session return a plain `User`,
 * so this reads `/me` before the app mounts, the way a page load does.
 */
async function loadCurrentUser(queryClient: QueryClient) {
	await queryClient.fetchQuery({
		queryKey: queryKeys.auth.currentUser(),
		queryFn: fetchCurrentUser,
		staleTime: 0
	});
}

/** Updates the cached user from an endpoint that returns a plain `User`, keeping their grants. */
export function mergeCurrentUser(queryClient: QueryClient, user: User) {
	queryClient.setQueryData<CurrentUser | null>(queryKeys.auth.currentUser(), (current) =>
		current ? { ...user, access: current.access } : current
	);
}

/**
 * Query hook for fetching current authenticated user
 */
export function useCurrentUserQuery() {
	return createQuery(() => ({
		queryKey: queryKeys.auth.currentUser(),
		queryFn: fetchCurrentUser,
		// Auth state should be checked frequently
		staleTime: 60 * 1000,
		// While unverified, refetch on every return to the tab, so verifying from the emailed
		// link (in another tab or on another device) clears the verification notices without a
		// reload.
		refetchOnWindowFocus: (query) => (query.state.data?.email_verified === false ? 'always' : true)
	}));
}

/**
 * Whether the current user still has to verify their email before the server allows gated
 * actions (invites, API keys, shares, emailing the install command, a second daemon).
 *
 * Display only: the server's `auth_email_verification_required` refusal is the actual gate,
 * so controls stay enabled and show this as help text. A server with no email transport lifts
 * the gate, and a missing flag counts as verified, matching the backend.
 */
export function useEmailVerificationPending() {
	const currentUserQuery = useCurrentUserQuery();
	const configQuery = useConfigQuery();
	return {
		get pending(): boolean {
			return (
				(configQuery.data?.has_email_service ?? false) &&
				currentUserQuery.data?.email_verified === false
			);
		},
		get email(): string | null {
			return currentUserQuery.data?.email ?? null;
		}
	};
}

/**
 * Mutation hook for logging in
 */
export function useLoginMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (request: LoginRequest) => {
			return unwrapData(await apiClient.POST('/api/auth/login', { body: request }));
		},
		onSuccess: async (user: User) => {
			await loadCurrentUser(queryClient);
			// Mark that user has an account (for redirect logic after logout)
			if (typeof localStorage !== 'undefined') {
				localStorage.setItem('hasAccount', 'true');
			}
			pushSuccess(auth_welcomeBack({ email: user.email }));
		}
	}));
}

/**
 * Mutation hook for checking email availability
 */
export function useCheckEmailMutation() {
	return createMutation(() => ({
		mutationFn: async (request: { email: string }) => {
			return unwrapData(await apiClient.POST('/api/auth/check-email', { body: request })).available;
		}
	}));
}

/**
 * Mutation hook for registering
 */
export function useRegisterMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (request: RegisterRequest) => {
			return unwrapData(await apiClient.POST('/api/auth/register', { body: request }));
		},
		onSuccess: async (user: User) => {
			await loadCurrentUser(queryClient);
			// Mark that user has an account (for redirect logic after logout)
			if (typeof localStorage !== 'undefined') {
				localStorage.setItem('hasAccount', 'true');
			}
			pushSuccess(auth_welcome({ email: user.email }));
		}
	}));
}

/**
 * Mutation hook for logging out
 */
export function useLogoutMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async () => {
			requireSuccess(await apiClient.POST('/api/auth/logout', {}));
			return true;
		},
		onSuccess: () => {
			queryClient.setQueryData(queryKeys.auth.currentUser(), null);
			// Invalidate all queries on logout
			queryClient.clear();
			resetIdentity();
			pushSuccess(auth_loggedOut());
		}
	}));
}

/**
 * Mutation hook for accepting the current terms. The returned user no longer
 * needs to accept, which lifts the terms gate.
 */
export function useAcceptTermsMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async () => {
			return unwrapData(await apiClient.POST('/api/auth/accept-terms', {}));
		},
		onSuccess: (user: User) => {
			mergeCurrentUser(queryClient, user);
		}
	}));
}

/**
 * Mutation hook for forgot password
 */
export function useForgotPasswordMutation() {
	return createMutation(() => ({
		mutationFn: async (request: ForgotPasswordRequest) => {
			requireSuccess(await apiClient.POST('/api/auth/forgot-password', { body: request }));
			return true;
		},
		onSuccess: () => {
			pushSuccess(auth_passwordResetLinkSent());
		}
	}));
}

/**
 * Mutation hook for reset password
 */
export function useResetPasswordMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (request: ResetPasswordRequest) => {
			return unwrapData(await apiClient.POST('/api/auth/reset-password', { body: request }));
		},
		onSuccess: async (user: User) => {
			await loadCurrentUser(queryClient);
			// Mark that user has an account (for redirect logic after logout)
			if (typeof localStorage !== 'undefined') {
				localStorage.setItem('hasAccount', 'true');
			}
			pushSuccess(auth_passwordHasBeenReset());
			pushSuccess(auth_welcome({ email: user.email }));
		}
	}));
}

/**
 * Mutation hook for pre-registration setup
 */
export function useSetupMutation() {
	return createMutation(() => ({
		mutationFn: async (request: SetupRequest) => {
			return unwrapData(
				await apiClient.POST('/api/auth/setup', { body: request })
			) as SetupResponse;
		}
	}));
}

/**
 * Mutation hook for verifying email
 */
export function useVerifyEmailMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (request: VerifyEmailRequest) => {
			return unwrapData(await apiClient.POST('/api/auth/verify-email', { body: request }));
		},
		onSuccess: async () => {
			await loadCurrentUser(queryClient);
			// Mark that user has an account (for redirect logic after logout)
			if (typeof localStorage !== 'undefined') {
				localStorage.setItem('hasAccount', 'true');
			}
			pushSuccess(auth_emailVerified());
		}
	}));
}

/**
 * Mutation hook for resending verification email
 */
export function useResendVerificationMutation() {
	return createMutation(() => ({
		mutationFn: async (request: ResendVerificationRequest) => {
			requireSuccess(await apiClient.POST('/api/auth/resend-verification', { body: request }));
			return true;
		},
		onSuccess: () => {
			pushSuccess(auth_verificationEmailSent());
		}
	}));
}

/**
 * Mutation hook for updating user profile (job title, company size)
 */
export function useProfileUpdateMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (request: { job_title?: string; company_size?: string }) => {
			requireSuccess(
				await apiClient.POST('/api/v1/organizations/profile', {
					body: request
				})
			);
			return true;
		},
		onSuccess: () => {
			// Refetch organization to pick up ProfileCompleted milestone
			queryClient.invalidateQueries({ queryKey: queryKeys.organizations.all });
		}
	}));
}

// Helper to check if user is authenticated from query data
export function isAuthenticated(user: User | null | undefined): boolean {
	return user !== null && user !== undefined;
}

/**
 * Mutation hook for saving onboarding step (and optionally use_case)
 */
export function useOnboardingStepMutation() {
	type UseCase = components['schemas']['UseCase'];
	return createMutation(() => ({
		mutationFn: async (params: {
			step: string;
			use_case?: UseCase;
			referral_source?: string;
			referral_source_other?: string;
		}) => {
			requireSuccess(
				await apiClient.POST('/api/auth/onboarding-step', {
					body: {
						step: params.step,
						use_case: params.use_case,
						referral_source: params.referral_source,
						referral_source_other: params.referral_source_other
					}
				})
			);
			return true;
		}
	}));
}

/**
 * Query hook for fetching onboarding state
 */
export function useOnboardingStateQuery() {
	return createQuery(() => ({
		queryKey: queryKeys.auth.onboardingState(),
		queryFn: async () => {
			const { data } = await apiClient.GET('/api/auth/onboarding-state', {});
			if (!data?.success || !data.data) {
				return { step: null, use_case: null, org_name: null, site: null, site_id: null };
			}
			return data.data;
		},
		// Don't retry - if it fails, just use defaults
		retry: false,
		staleTime: 60 * 1000
	}));
}
