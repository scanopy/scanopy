import { trackEvent } from '$lib/shared/utils/analytics';
import { openModal } from '$lib/shared/stores/modal-registry';
import { reopenSettingsTabAfterPayment } from '$lib/features/billing/stores';
import type { Organization } from '$lib/features/organizations/types';

interface StartSetupPaymentArgs {
	org: Organization | null | undefined;
	/**
	 * Plan to subscribe to once the card is on file. A lapsed org has no live
	 * subscription for the card to attach to, so the dialog buys this plan
	 * right after collecting it (the same handoff the plan picker uses).
	 */
	plan?: Organization['plan'];
	source:
		| 'trial_card'
		| 'trial_banner'
		| 'trial_modal'
		| 'sidebar_trial_pill'
		| 'billing_tab'
		| 'license_tab';
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
	openModal('payment-method', plan ? { entityData: { plan } } : undefined);
}
