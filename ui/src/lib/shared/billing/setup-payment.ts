import { trackEvent } from '$lib/shared/utils/analytics';
import { openModal } from '$lib/shared/stores/modal-registry';
import { reopenSettingsTabAfterPayment } from '$lib/features/billing/stores';
import { ApiError } from '$lib/api/client';
import type { Organization } from '$lib/features/organizations/types';

/** The "Add/Update payment method" nudges that funnel through `startSetupPayment`. */
export type SetupPaymentSource =
	| 'trial_card'
	| 'trial_banner'
	| 'trial_modal'
	| 'no_payment_banner'
	| 'sidebar_trial_pill'
	| 'billing_tab'
	| 'license_tab';

/**
 * Every surface that opens the payment-method dialog. The plan picker opens it
 * directly rather than through `startSetupPayment`.
 */
export type PaymentFormSource = SetupPaymentSource | 'plan_picker';

interface StartSetupPaymentArgs {
	org: Organization | null | undefined;
	/**
	 * Plan to subscribe to once the card is on file. A lapsed org has no live
	 * subscription for the card to attach to, so the dialog buys this plan
	 * right after collecting it (the same handoff the plan picker uses).
	 */
	plan?: Organization['plan'];
	source: SetupPaymentSource;
	trialDaysLeft: number | null;
}

/**
 * Open the in-app payment-method dialog (Stripe Elements). Every "Add/Update
 * payment method" nudge funnels through here, so card collection happens in a
 * modal instead of redirecting out to a Stripe-hosted page.
 */
export function startSetupPayment({
	org,
	plan,
	source,
	trialDaysLeft
}: StartSetupPaymentArgs): void {
	trackEvent('add_payment_cta_clicked', {
		source,
		plan_status: org?.plan_status,
		trial_days_left: trialDaysLeft,
		has_payment_method: org?.has_payment_method ?? false
	});
	// The License tab lives inside Settings, which this modal replaces in the
	// registry. Record where to go back to so the two match again afterwards.
	reopenSettingsTabAfterPayment.set(source === 'license_tab' ? 'license' : null);
	openModal('payment-method', { entityData: plan ? { plan, source } : { source } });
}

/** Where in the payment-method dialog a failure happened. */
export type PaymentFormStage = 'setup_intent' | 'confirm' | 'finalize';

/** What a failure reports, apart from the stage and the source. */
export interface PaymentFormFailure {
	error_type: string | null;
	error_code: string | null;
	decline_code: string | null;
}

/**
 * A failed backend call (creating or finalizing the SetupIntent), reduced to
 * the fields `payment_form_failed` reports. Only an `ApiError` carries the
 * backend's error code; anything else (a timeout, a network failure) reports
 * its class name alone.
 */
export function apiFailure(err: unknown): PaymentFormFailure {
	return {
		error_type: err instanceof Error ? err.name : null,
		error_code: err instanceof ApiError ? err.code : null,
		decline_code: null
	};
}

/** Report a failure in the payment-method dialog. Failures only, never success. */
export function trackPaymentFormFailed(
	stage: PaymentFormStage,
	source: PaymentFormSource | null,
	failure: PaymentFormFailure
): void {
	trackEvent('payment_form_failed', { stage, source, ...failure });
}
