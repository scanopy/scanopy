/**
 * TanStack Query hooks for Authentication
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
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

/**
 * Query hook for fetching current authenticated user
 */
export function useCurrentUserQuery() {
	return createQuery(() => ({
		queryKey: queryKeys.auth.currentUser(),
		queryFn: async () => {
			const { data } = await apiClient.POST('/api/auth/me', {});
			if (!data?.success || !data.data) {
				return null;
			}
			return data.data;
		},
		// Don't retry auth checks - if it fails, user is not authenticated
		retry: false,
		// Auth state should be checked frequently
		staleTime: 60 * 1000
	}));
}

/**
 * Mutation hook for logging in
 */
export function useLoginMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (request: LoginRequest) => {
			const { data } = await apiClient.POST('/api/auth/login', { body: request });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Login failed. Please check your credentials.');
			}
			return data.data;
		},
		onSuccess: (user: User) => {
			queryClient.setQueryData(queryKeys.auth.currentUser(), user);
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
			const { data } = await apiClient.POST('/api/auth/check-email', { body: request });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Email check failed');
			}
			return data.data.available;
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
			const { data, error: apiError } = await apiClient.POST('/api/auth/register', {
				body: request
			});
			if (!data?.success || !data.data) {
				const err = new Error(
					(apiError as Record<string, string> | undefined)?.error ||
						data?.error ||
						'Registration failed. Please try again.'
				);
				(err as Error & { code?: string }).code = (
					apiError as Record<string, string> | undefined
				)?.code;
				throw err;
			}
			return data.data;
		},
		onSuccess: (user: User) => {
			queryClient.setQueryData(queryKeys.auth.currentUser(), user);
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
			const { data } = await apiClient.POST('/api/auth/logout', {});
			if (!data?.success) {
				throw new Error(data?.error || 'Logout failed');
			}
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
 * Mutation hook for forgot password
 */
export function useForgotPasswordMutation() {
	return createMutation(() => ({
		mutationFn: async (request: ForgotPasswordRequest) => {
			const { data } = await apiClient.POST('/api/auth/forgot-password', { body: request });
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to send password reset link');
			}
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
			const { data } = await apiClient.POST('/api/auth/reset-password', { body: request });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to reset password');
			}
			return data.data;
		},
		onSuccess: (user: User) => {
			queryClient.setQueryData(queryKeys.auth.currentUser(), user);
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
			const { data } = await apiClient.POST('/api/auth/setup', { body: request });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to save setup data');
			}
			return data.data as SetupResponse;
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
			const { data } = await apiClient.POST('/api/auth/verify-email', { body: request });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Email verification failed');
			}
			return data.data;
		},
		onSuccess: (user: User) => {
			queryClient.setQueryData(queryKeys.auth.currentUser(), user);
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
			const { data } = await apiClient.POST('/api/auth/resend-verification', { body: request });
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to resend verification email');
			}
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
			const { data } = await apiClient.POST('/api/v1/organizations/profile', {
				body: request
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to update profile');
			}
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
			const { data } = await apiClient.POST('/api/auth/onboarding-step', {
				body: {
					step: params.step,
					use_case: params.use_case,
					referral_source: params.referral_source,
					referral_source_other: params.referral_source_other
				}
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to save onboarding step');
			}
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
				return { step: null, use_case: null, org_name: null, network: null, network_id: null };
			}
			return data.data;
		},
		// Don't retry - if it fails, just use defaults
		retry: false,
		staleTime: 60 * 1000
	}));
}
