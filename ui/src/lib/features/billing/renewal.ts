import * as m from '$lib/paraglide/messages';
import type { components } from '$lib/api/schema';
import { formatDate } from '$lib/shared/utils/formatting';

type OrgBase = components['schemas']['OrganizationBase'];

/**
 * Human-facing label for the next renewal / period-end date on the org's
 * current plan. Renders in BillingTab next to the plan name so users can
 * contextualise "next renewal" copy in surrounding UI.
 *
 * Switches phrasing on `plan_status`:
 *   - active → "Next renewal on ..."
 *   - pending_cancellation → "Subscription ends on ..."
 *
 * Returns null for everything else, including `trialing` — that state has
 * its own dedicated "Trial ends on ..." line in BillingTab that frames the
 * date as the trial expiry rather than the first invoice. Free / paused /
 * past_due / cancelled have either no renewal concept or a stale stored
 * value; the line is suppressed.
 *
 * Forward-looking first-invoice copy on plans the user is *picking* lives
 * on `BillingPlanForm` (it computes `now + trial_days` predictively) — this
 * helper is for the user's *current* plan only.
 */
export function renewalLabel(org: OrgBase | undefined | null): string | null {
	if (!org?.next_renewal_at) return null;
	const formatted = formatDate(org.next_renewal_at);
	switch (org.plan_status) {
		case 'active':
			return m.billing_nextRenewalOn({ date: formatted });
		case 'pending_cancellation':
			return m.billing_subscriptionEndsOn({ date: formatted });
		default:
			return null;
	}
}
