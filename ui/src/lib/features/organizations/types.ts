import type { components } from '$lib/api/schema';
import { billingPlans } from '$lib/shared/stores/metadata';

// Re-export generated types
export type Organization = components['schemas']['Organization'];
export type OrganizationInvite = components['schemas']['Invite'];
export type CreateInviteRequest = components['schemas']['CreateInviteRequest'];

export function isBillingPlanActive(organization: Organization): boolean {
	const type = organization.plan?.type;
	// No plan selected yet — the hard-gate must trigger.
	if (type == null) return false;
	// Non-Stripe plans (Demo / Community / CommercialSelfHosted) have no
	// Stripe lifecycle, so they're considered active by definition.
	if (billingPlans.getMetadata(type).is_stripe_managed === false) return true;
	// A lapsed org keeps its plan and browses read-only, so the hard-gate
	// stays out of its way; PlanLapsedBanner carries the recovery CTA.
	return (
		organization.plan_status == 'active' ||
		organization.plan_status == 'trialing' ||
		organization.plan_status == 'pending_cancellation' ||
		organization.plan_status == 'past_due' ||
		organization.plan_status == 'paused' ||
		organization.plan_status == 'cancelled'
	);
}

/**
 * The org's subscription ended (unpaid trial, or a cancellation that reached
 * its period end) and it has not chosen a paid plan since. It keeps its plan;
 * the backend refuses mutating requests until it does. Mirrors
 * `Organization::is_lapsed` on the backend.
 */
export function isPlanLapsed(organization: Organization): boolean {
	const type = organization.plan?.type;
	if (type == null) return false;
	if (billingPlans.getMetadata(type).is_stripe_managed !== true) return false;
	return organization.plan_status == 'cancelled';
}

/**
 * The org is on a licensed self-hosted plan (SelfHostedStandard / SelfHostedPlus).
 * On a billing-enabled server these orgs get license keys, not the main app.
 */
export function hasLicensedPlan(organization: Organization): boolean {
	const type = organization.plan?.type;
	if (type == null) return false;
	return billingPlans.getMetadata(type).license_plan != null;
}

/**
 * A genuine paid subscription is live. Stricter than {@link isBillingPlanActive}
 * (which is the permissive hard-gate predicate): this gates the
 * "Subscription activated successfully!" toast so it doesn't fire for
 * downgrade-to-Free, pause, past_due, or pending_cancellation.
 */
export function isPaidSubscriptionActive(organization: Organization): boolean {
	const type = organization.plan?.type;
	if (type == null) return false;
	if (billingPlans.getMetadata(type).is_stripe_managed !== true) return false;
	return organization.plan_status == 'active' || organization.plan_status == 'trialing';
}
