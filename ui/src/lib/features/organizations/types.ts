import type { components } from '#lib/api/schema';
import { billingPlans } from '#lib/shared/stores/metadata.ts';

// Re-export generated types
// The signed-in user's organization, as `/api/v1/organizations/current` serves it.
export type Organization = components['schemas']['OrganizationResponse'];
/** The stored organization, which the plan predicates below read. */
type OrganizationEntity = components['schemas']['Organization'];
export type OrganizationInvite = components['schemas']['Invite'];
export type CreateInviteRequest = components['schemas']['CreateInviteRequest'];

export function isBillingPlanActive(organization: OrganizationEntity): boolean {
	const type = organization.plan?.type;
	// No plan selected yet: the plan picker opens and the app locks to Settings.
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
export function isPlanLapsed(organization: OrganizationEntity): boolean {
	const type = organization.plan?.type;
	if (type == null) return false;
	if (billingPlans.getMetadata(type).is_stripe_managed !== true) return false;
	return organization.plan_status == 'cancelled';
}

/**
 * The org is on a licensed self-hosted plan (SelfHostedStandard / SelfHostedPlus).
 * On Scanopy Cloud these orgs get license keys, not the main app.
 */
/**
 * The org's plan includes the topology view. Free doesn't: the topology tab
 * shows a plan-required overlay and nothing fetches topology data, which the
 * backend refuses for it.
 */
export function hasTopologyAccess(organization: OrganizationEntity | null | undefined): boolean {
	const type = organization?.plan?.type;
	if (type == null) return false;
	return billingPlans.getMetadata(type).includes_topology === true;
}

export function hasLicensedPlan(organization: OrganizationEntity): boolean {
	const type = organization.plan?.type;
	if (type == null) return false;
	return billingPlans.getMetadata(type).license_plan != null;
}

/**
 * The org is locked out of this app: it holds a licensed self-hosted plan on a
 * cloud server, so it runs Scanopy on its own server and gets
 * only Settings here. Mirrors `EmailService::self_hosted_plan_locked`.
 */
export function isSelfHostedPlanLocked(
	organization: OrganizationEntity | null | undefined,
	isCloudDeployment: boolean
): boolean {
	return isCloudDeployment && organization != null && hasLicensedPlan(organization);
}

/**
 * The org signed up for a self-hosted license, but its trial never started (it
 * signed up before signup started it, or the start failed), so it has a
 * license plan and no subscription. It stays on the plan picker over Settings
 * until it chooses. A cloud org always has a plan: Free, if nothing else.
 */
export function isPlanUnchosen(
	organization: OrganizationEntity | null | undefined,
	isCloudDeployment: boolean
): boolean {
	return (
		isCloudDeployment &&
		organization != null &&
		hasLicensedPlan(organization) &&
		organization.plan_status == null
	);
}

/**
 * A genuine paid subscription is live. Stricter than {@link isBillingPlanActive}
 * (which is the permissive hard-gate predicate): this gates the
 * "Subscription activated successfully!" toast so it doesn't fire for
 * downgrade-to-Free, pause, past_due, or pending_cancellation.
 */
export function isPaidSubscriptionActive(organization: OrganizationEntity): boolean {
	const type = organization.plan?.type;
	if (type == null) return false;
	if (billingPlans.getMetadata(type).is_stripe_managed !== true) return false;
	return organization.plan_status == 'active' || organization.plan_status == 'trialing';
}
