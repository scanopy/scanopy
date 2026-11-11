import * as m from '#lib/paraglide/messages.js';
import type { components } from '#lib/api/schema';
import { formatDate } from '#lib/shared/utils/formatting.ts';

type OrgBase = components['schemas']['OrganizationBase'];

/**
 * "Next renewal on ..." for an active subscription on the org's current plan.
 * Renders in BillingTab under the plan name when no plan-state line is shown.
 *
 * Returns null for every other status. Trialing, pending cancellation, paused
 * and lapsed each get their own status line in BillingTab, and Free /
 * past_due have either no renewal concept or a stale stored value.
 *
 * Forward-looking first-invoice copy on plans the user is *picking* lives
 * on `BillingPlanForm` (it computes `now + trial_days` predictively) — this
 * helper is for the user's *current* plan only.
 */
export function renewalLabel(org: OrgBase | undefined | null): string | null {
	if (!org?.next_renewal_at || org.plan_status !== 'active') return null;
	return m.billing_nextRenewalOn({ date: formatDate(org.next_renewal_at) });
}
