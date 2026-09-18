import type { Organization } from '$lib/features/organizations/types';
import { billingPlans } from '$lib/shared/stores/metadata';

export function getTrialEndDate(org: Organization | null | undefined): Date | null {
	return org?.trial_end_date ? new Date(org.trial_end_date) : null;
}

export function getTrialDaysLeft(org: Organization | null | undefined): number | null {
	const end = getTrialEndDate(org);
	if (!end) return null;
	const diff = end.getTime() - Date.now();
	return Math.max(0, Math.ceil(diff / (1000 * 60 * 60 * 24)));
}

/**
 * True when the org is mid-trial with no card on file, on a deployment that
 * actually bills. `billingEnabled` comes from `/api/config` and is required
 * rather than optional: with Stripe unconfigured, `plan_status` and
 * `has_payment_method` are frozen at whatever a previous Stripe-enabled run
 * left behind (both are written only by Stripe webhooks), so acting on them
 * nags about a subscription the deployment cannot have.
 */
export function isTrialingWithoutPayment(
	org: Organization | null | undefined,
	billingEnabled: boolean
): boolean {
	return billingEnabled && org?.plan_status === 'trialing' && !canPay(org);
}

/**
 * Whether the org has a way to pay its next invoice: a card on file, or a
 * subscription billed by sent invoice against a purchase order. Mirrors
 * `Organization::can_pay` on the backend; every payment nag reads it so an
 * invoice buyer is never asked for a card.
 */
export function canPay(org: Organization | null | undefined): boolean {
	return (org?.has_payment_method ?? false) || (org?.bills_by_invoice ?? false);
}

/**
 * True when the org is on a Stripe-managed plan that requires a card on file
 * but has none. Single source of truth for every payment-method nag (banner,
 * sidebar pill, BillingTab card) so they show/hide together. `is_stripe_managed
 * === true` fails safe: missing/stale plan metadata hides the nag rather than
 * showing it. `has_payment_method` is authoritative — it only flips on Stripe
 * `payment_method.attached`/`detached` webhooks, not on plan changes — and an
 * org billed by invoice has a way to pay without one.
 */
export function isMissingPaymentMethod(
	org: Organization | null | undefined,
	billingEnabled: boolean
): boolean {
	if (!org || !billingEnabled) return false;
	const meta = billingPlans.getMetadata(org.plan?.type ?? null);
	return (
		meta.is_stripe_managed === true &&
		(org.plan_status === 'trialing' ||
			org.plan_status === 'active' ||
			org.plan_status === 'past_due') &&
		!canPay(org)
	);
}

export function getDaysIntoTrial(org: Organization | null | undefined): number | null {
	if (!org?.created_at) return null;
	const created = new Date(org.created_at).getTime();
	if (Number.isNaN(created)) return null;
	const diff = Date.now() - created;
	return Math.max(0, Math.floor(diff / (1000 * 60 * 60 * 24)));
}
