import type { Organization } from '$lib/features/organizations/types';
import { billingPlans } from '$lib/shared/stores/metadata';
import pickablePlans from '$lib/data/billing-plans.json';

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
 * actually bills (Scanopy Cloud). `isCloudDeployment` comes from `/api/config` and is required
 * rather than optional: with Stripe unconfigured, `plan_status` and
 * `has_payment_method` are frozen at whatever a previous Stripe-enabled run
 * left behind (both are written only by Stripe webhooks), so acting on them
 * nags about a subscription the deployment cannot have.
 */
export function isTrialingWithoutPayment(
	org: Organization | null | undefined,
	isCloudDeployment: boolean
): boolean {
	return isCloudDeployment && org?.plan_status === 'trialing' && !canPay(org);
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
	isCloudDeployment: boolean
): boolean {
	if (!org || !isCloudDeployment) return false;
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

/**
 * Whether the org is inside its trial. A trial with a cancellation scheduled
 * is still one (Stripe keeps it trialing; the org reads `pending_cancellation`),
 * so a plan switch keeps it running. Mirrors `Organization::is_in_trial`.
 */
export function isInTrial(org: Organization | null | undefined, now: number = Date.now()): boolean {
	if (org?.plan_status !== 'trialing' && org?.plan_status !== 'pending_cancellation') return false;
	return org.trial_end_date != null && new Date(org.trial_end_date).getTime() > now;
}

/**
 * When the card being added for `pendingPlan` (or for the org's current plan,
 * when null) is first charged, if a trial defers that charge: the end of the
 * trial the org is on (a switch keeps it), or the end of the trial the picked
 * plan starts. Null when the card is charged straight away.
 */
export function trialChargeDate(
	org: Organization | null | undefined,
	pendingPlan: Organization['plan'] | null,
	now: number = Date.now()
): Date | null {
	if (org == null) return null;
	if (isInTrial(org, now) && org.trial_end_date) return new Date(org.trial_end_date);
	if (pendingPlan != null && pendingPlan.trial_days > 0 && org.trial_end_date == null) {
		return new Date(now + pendingPlan.trial_days * 24 * 60 * 60 * 1000);
	}
	return null;
}

/**
 * Whether the org has had a paid subscription or a trial, so it gets no trial offer. Mirrors the
 * backend's checkout rule (`billing/service/checkout.rs`); `trial_end_date` is set only by a
 * subscription with a trial period. An org mid-trial is not returning: it sees trial-aware UI
 * instead. A paid plan counts only once a subscription started on it: a license signup whose trial
 * never started is on Self-Hosted Standard with no status.
 */
export function isReturningCustomer(org: Organization | null | undefined): boolean {
	if (!org || isInTrial(org)) return false;
	return (
		(org.plan_status != null && billingPlans.getMetadata(org.plan.type)?.is_free !== true) ||
		!!org.trial_end_date
	);
}

/**
 * The longest free trial a cloud plan with the topology view offers, in days; 0 when none does.
 * Read from the plans the picker sells (`billing-plans.json`): the metadata store's
 * `billing-plans-all.json` describes every variant at its default config, which has no trial.
 */
export function topologyTrialDays(): number {
	const days = pickablePlans
		.map((plan) => plan.metadata)
		.filter((meta) => meta.license_plan == null && !meta.is_free && meta.includes_topology)
		.map((meta) => meta.trial_days);
	return Math.max(0, ...days);
}

/** Whether the org would get a trial of the topology view if it picked a plan now. */
export function canTrialTopology(org: Organization | null | undefined): boolean {
	return !isReturningCustomer(org) && topologyTrialDays() > 0;
}

/**
 * Whether the org can take its one self-serve trial extension now: trialing,
 * in the last three days, and not extended before.
 */
export function canExtendTrial(
	org: Organization | null | undefined,
	trialDaysLeft: number | null
): boolean {
	return (
		org?.plan_status === 'trialing' &&
		trialDaysLeft !== null &&
		trialDaysLeft <= 3 &&
		!org.trial_extended_used
	);
}
