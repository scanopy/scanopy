/**
 * TanStack Query hooks for Billing
 */

import { createQuery, createMutation } from '@tanstack/svelte-query';
import { queryKeys, queryClient } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import type { BillingPlan, BillingRate } from './types';
import type { components } from '$lib/api/schema';
import { pushError, pushSuccess } from '$lib/shared/stores/feedback';
import {
	billing_errorApplyingDiscount,
	billing_errorBillingPortal,
	billing_errorCancellingSubscription,
	billing_errorChangingPlan,
	billing_errorExtendingTrial,
	billing_errorPausingSubscription,
	billing_errorReactivatingSubscription,
	billing_errorResumingSubscription,
	billing_errorSavingPaymentMethod,
	billing_errorStartingCardSetup
} from '$lib/paraglide/messages';

type PauseDuration = components['schemas']['PauseDuration'];
type CancelSubscriptionRequest = components['schemas']['CancelSubscriptionRequest'];
type CancelSubscriptionResponse = components['schemas']['CancelSubscriptionResponse'];
type LicenseKeyType = components['schemas']['LicenseKeyType'];
type LicenseKeyResponse = components['schemas']['LicenseKeyResponse'];

/**
 * Query hook for fetching current billing plans
 */
export function useBillingPlansQuery() {
	return createQuery(() => ({
		queryKey: queryKeys.billing.plans(),
		queryFn: async () => {
			const { data } = await apiClient.GET('/api/billing/plans');
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch billing plans');
			}
			return data.data;
		}
	}));
}

/**
 * Mutation hook for checkout
 */
export function useCheckoutMutation() {
	return createMutation(() => ({
		mutationFn: async (plan: BillingPlan) => {
			const { data } = await apiClient.POST('/api/billing/checkout', {
				body: { plan, url: window.location.origin }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to get checkout URL');
			}
			return data.data;
		},
		onSuccess: (data: string) => {
			// Non-URL response means plan was changed directly (existing subscriber)
			if (!data.startsWith('http')) {
				pushSuccess(data);
			}
		},
		onError: (error: Error) => {
			pushError(billing_errorChangingPlan({ message: error.message }));
		}
	}));
}

/**
 * Mutation hook for opening customer portal
 */
export function useCustomerPortalMutation() {
	return createMutation(() => ({
		mutationFn: async () => {
			const { data } = await apiClient.POST('/api/billing/portal', {
				body: window.location.origin
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to get billing portal URL');
			}
			return data.data;
		},
		onError: (error: Error) => {
			pushError(billing_errorBillingPortal({ message: error.message }));
		}
	}));
}

/**
 * Mutation hook to create a SetupIntent for in-app card collection.
 * Returns the client secret used to mount the Stripe Payment Element.
 */
export function useCreateSetupIntentMutation() {
	return createMutation(() => ({
		mutationFn: async () => {
			const { data } = await apiClient.POST('/api/billing/payment-method-setup-intent', {});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to start card setup');
			}
			return data.data.client_secret;
		},
		onError: (error: Error) => {
			pushError(billing_errorStartingCardSetup({ message: error.message }));
		}
	}));
}

/**
 * Mutation hook to finalize a client-confirmed SetupIntent — sets the collected
 * card as the customer's default payment method and flips has_payment_method.
 */
export function useFinalizePaymentMethodMutation() {
	return createMutation(() => ({
		mutationFn: async (setupIntentId: string) => {
			const { data } = await apiClient.POST('/api/billing/finalize-payment-method', {
				body: { setup_intent_id: setupIntentId }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to save payment method');
			}
			return true;
		},
		onError: (error: Error) => {
			pushError(billing_errorSavingPaymentMethod({ message: error.message }));
		}
	}));
}

/**
 * Mutation hook for changing plan
 */
export function useChangePlanMutation() {
	return createMutation(() => ({
		mutationFn: async ({ plan, rate }: { plan: BillingPlan; rate: BillingRate }) => {
			const { data } = await apiClient.POST('/api/billing/change-plan', {
				body: { plan, rate }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to change plan');
			}
			return data.data;
		},
		onSuccess: (data: string) => {
			pushSuccess(data);
		},
		onError: (error: Error) => {
			pushError(billing_errorChangingPlan({ message: error.message }));
		}
	}));
}

/**
 * Mutation hook for pausing the subscription
 */
export function usePauseSubscriptionMutation() {
	return createMutation(() => ({
		mutationFn: async (duration_days: PauseDuration) => {
			const { data } = await apiClient.POST('/api/billing/pause', {
				body: { duration_days }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to pause subscription');
			}
			return data.data;
		},
		// No onSuccess toast — the call site fires it AFTER waitForOrgUpdate
		// confirms the org actually flipped to paused. The API 200 only means
		// Stripe accepted the request, not that downstream state is consistent.
		onError: (error: Error) => {
			pushError(billing_errorPausingSubscription({ message: error.message }));
		}
	}));
}

/**
 * Mutation hook for resuming a paused subscription
 */
export function useResumeSubscriptionMutation() {
	return createMutation(() => ({
		mutationFn: async () => {
			const { data } = await apiClient.POST('/api/billing/resume', {});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to resume subscription');
			}
			return data.data;
		},
		// No onSuccess toast — call site fires it after waitForOrgUpdate.
		onError: (error: Error) => {
			pushError(billing_errorResumingSubscription({ message: error.message }));
		}
	}));
}

/**
 * Mutation hook for reactivating a subscription pending cancellation
 */
export function useReactivateSubscriptionMutation() {
	return createMutation(() => ({
		mutationFn: async () => {
			const { data } = await apiClient.POST('/api/billing/reactivate', {});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to reactivate subscription');
			}
			return data.data;
		},
		// No onSuccess toast — call site fires it after waitForOrgUpdate.
		onError: (error: Error) => {
			pushError(billing_errorReactivatingSubscription({ message: error.message }));
		}
	}));
}

/**
 * Mutation hook for self-serve trial extend (+7 days, once per lifetime)
 */
export function useExtendTrialMutation() {
	return createMutation(() => ({
		mutationFn: async () => {
			const { data } = await apiClient.POST('/api/billing/extend-trial', {});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to extend trial');
			}
			return data.data;
		},
		// No onSuccess toast — call site fires it after waitForOrgUpdate.
		onError: (error: Error) => {
			pushError(billing_errorExtendingTrial({ message: error.message }));
		}
	}));
}

/**
 * Mutation hook for in-app subscription cancel.
 * Returns the period_end so the modal can render the retention disclosure.
 */
export function useCancelSubscriptionMutation() {
	return createMutation(() => ({
		mutationFn: async (request: CancelSubscriptionRequest): Promise<CancelSubscriptionResponse> => {
			const { data } = await apiClient.POST('/api/billing/cancel', { body: request });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to cancel subscription');
			}
			return data.data;
		},
		onError: (error: Error) => {
			pushError(billing_errorCancellingSubscription({ message: error.message }));
		}
	}));
}

/**
 * Query hook for the live save-offer coupon terms.
 * Returns null when STRIPE_SAVE_OFFER_COUPON_ID is unset — the cancel
 * modal hides the discount panel in that case.
 */
export function useSaveOfferCouponQuery(enabled: () => boolean = () => true) {
	return createQuery(() => ({
		queryKey: queryKeys.billing.saveOfferCoupon(),
		enabled: enabled(),
		queryFn: async () => {
			const { data } = await apiClient.GET('/api/billing/save-offer-coupon', {});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to read save-offer coupon');
			}
			return data.data ?? null;
		}
	}));
}

/**
 * Mutation hook for the discount save offer.
 * Server returns 400 with a clear message when STRIPE_SAVE_OFFER_COUPON_ID
 * is unset; the auto-toast pipeline surfaces the error.
 */
export function useApplyDiscountSaveOfferMutation() {
	return createMutation(() => ({
		mutationFn: async () => {
			const { data } = await apiClient.POST('/api/billing/cancel/apply-discount', {});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to apply discount');
			}
			return data.data;
		},
		// No onSuccess toast — call site fires it after waitForOrgUpdate confirms
		// `org.last_discount_at` is populated, so success is tied to the actual
		// downstream write rather than the Stripe acknowledgement.
		onError: (error: Error) => {
			pushError(billing_errorApplyingDiscount({ message: error.message }));
		}
	}));
}

/**
 * Query hook for the key this org has issued right now, with its type. The
 * License tab reads the key through this instead of minting on mount, so
 * opening Settings neither issues a key nor 403s before the plan lands.
 */
export function useCurrentLicenseKeyQuery(enabled: () => boolean = () => true) {
	return createQuery(() => ({
		queryKey: queryKeys.licenses.currentKey(),
		enabled: enabled(),
		queryFn: async (): Promise<LicenseKeyResponse> => {
			const { data } = await apiClient.GET('/api/v1/licenses/keys/current', {});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to read license key');
			}
			return data.data;
		}
	}));
}

/**
 * Mutation hook that sets which key type this org uses. A different type retires
 * the previous key and mints the new one; the same type returns the current key.
 * A plan without the key type returns 403; the API client toasts it.
 */
export function useCreateLicenseKeyMutation() {
	return createMutation(() => ({
		mutationFn: async (key_type: LicenseKeyType): Promise<LicenseKeyResponse> => {
			const { data } = await apiClient.POST('/api/v1/licenses/keys', {
				body: { key_type }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to create license key');
			}
			return data.data;
		},
		// The response is the org's current key, so the tab shows the new key
		// without a second round trip.
		onSuccess: (data: LicenseKeyResponse) => {
			queryClient.setQueryData(queryKeys.licenses.currentKey(), data);
		}
	}));
}

/**
 * Mutation hook that ends a trial immediately and charges the card on file.
 * No onError: the API client already toasts, and a handler here double-toasts.
 */
export function useEndTrialMutation() {
	return createMutation(() => ({
		mutationFn: async () => {
			const { data } = await apiClient.POST('/api/billing/end-trial', {});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to end trial');
			}
			return data.data;
		}
	}));
}

/**
 * Mutation hook that retires every online key issued so far. Servers still on an
 * old key stop receiving entitlements until they're given a newly copied one.
 */
export function useRotateLicenseKeyMutation() {
	return createMutation(() => ({
		mutationFn: async () => {
			const { data } = await apiClient.POST('/api/v1/licenses/keys/rotate', {});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to rotate license key');
			}
			return true;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: queryKeys.organizations.current() });
			queryClient.invalidateQueries({ queryKey: queryKeys.licenses.currentKey() });
		}
	}));
}

/**
 * Query hook for previewing plan change overage
 */
export function useChangePlanPreviewQuery(plan: () => BillingPlan | null) {
	return createQuery(() => ({
		queryKey: [...queryKeys.billing.plans(), 'preview', plan()],
		queryFn: async () => {
			const planValue = plan();
			if (!planValue) return null;
			const { data } = await apiClient.GET('/api/billing/change-plan/preview', {
				params: { query: { plan: JSON.stringify(planValue) } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to get plan preview');
			}
			return data.data;
		},
		enabled: !!plan()
	}));
}

type InvoiceBillingRequest = components['schemas']['InvoiceBillingRequest'];
type InvoiceBillingStatus = components['schemas']['InvoiceBillingStatus'];

/**
 * Query hook for invoice billing state on a self-hosted plan: whether the
 * org pays by invoice, its PO number, the unpaid invoice, and an open quote.
 */
export function useInvoiceBillingStatusQuery(enabled: () => boolean = () => true) {
	return createQuery(() => ({
		queryKey: queryKeys.billing.invoiceBilling(),
		enabled: enabled(),
		queryFn: async (): Promise<InvoiceBillingStatus> => {
			const { data } = await apiClient.GET('/api/billing/invoice-billing', {});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to read invoice billing');
			}
			return data.data;
		}
	}));
}

function invalidateInvoiceBilling() {
	queryClient.invalidateQueries({ queryKey: queryKeys.billing.invoiceBilling() });
	queryClient.invalidateQueries({ queryKey: queryKeys.organizations.current() });
}

/**
 * Mutation hook that switches the org to paying by invoice, either sending
 * the first invoice now or opening a quote. The API client toasts failures.
 */
export function useSetUpInvoiceBillingMutation() {
	return createMutation(() => ({
		mutationFn: async (request: InvoiceBillingRequest): Promise<string> => {
			const { data } = await apiClient.POST('/api/billing/invoice-billing', { body: request });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to set up invoice billing');
			}
			return data.data;
		},
		onSuccess: invalidateInvoiceBilling
	}));
}

/**
 * Mutation hook that accepts the open quote. The invoice carries whatever PO
 * number the billing account holds; `useUpdatePoNumberMutation` is the one
 * place that sets it.
 */
export function useAcceptQuoteMutation() {
	return createMutation(() => ({
		mutationFn: async (): Promise<string> => {
			const { data } = await apiClient.POST('/api/billing/quote/accept', {});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to accept quote');
			}
			return data.data;
		},
		onSuccess: invalidateInvoiceBilling
	}));
}

/** Mutation hook that cancels the open quote. */
export function useCancelQuoteMutation() {
	return createMutation(() => ({
		mutationFn: async () => {
			const { data } = await apiClient.DELETE('/api/billing/quote', {});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to cancel quote');
			}
		},
		onSuccess: invalidateInvoiceBilling
	}));
}

/** Mutation hook that replaces the PO number printed on future invoices. */
export function useUpdatePoNumberMutation() {
	return createMutation(() => ({
		mutationFn: async (poNumber: string | null) => {
			const { data } = await apiClient.PUT('/api/billing/po-number', {
				body: { po_number: poNumber }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to update PO number');
			}
		},
		onSuccess: invalidateInvoiceBilling
	}));
}

/** Download the open quote as a PDF through the backend. */
export async function downloadQuotePdf(quoteNumber: string | null): Promise<void> {
	const { data } = await apiClient.GET('/api/billing/quote/pdf', { parseAs: 'blob' });
	if (!data) return;
	const url = URL.createObjectURL(data as Blob);
	const link = document.createElement('a');
	link.href = url;
	link.download = `scanopy-quote-${quoteNumber ?? 'draft'}.pdf`;
	document.body.appendChild(link);
	link.click();
	document.body.removeChild(link);
	URL.revokeObjectURL(url);
}
