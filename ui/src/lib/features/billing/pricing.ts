/**
 * What the org's plan costs right now.
 *
 * An active save-offer coupon changes what the card is actually charged, so
 * anything that quotes an amount reads it through here rather than off
 * `plan.base_cents`.
 */
import { billingPlans } from '$lib/shared/stores/metadata';
import type { BillingRate } from './types';
import type { Organization } from '$lib/features/organizations/types';

export interface SaveOfferDiscount {
	percentOff: number;
	rate: BillingRate;
	expiresAt: string;
}

/**
 * The live save-offer discount, or null. The discount columns can still be
 * populated on a non-Stripe plan (an org that took a discount on Pro and then
 * downgraded to Free), so the plan and the expiry are both checked.
 */
export function saveOfferDiscount(org: Organization | null | undefined): SaveOfferDiscount | null {
	if (!org) return null;
	if (billingPlans.getMetadata(org.plan?.type ?? null).is_stripe_managed !== true) return null;
	const until = org.discount_save_offer_active_until;
	const percent = org.discount_save_offer_percent_off;
	if (!until || percent == null) return null;
	const expiresAt = new Date(until);
	if (expiresAt.getTime() <= Date.now()) return null;
	return {
		percentOff: percent,
		rate: org.plan?.rate ?? 'Month',
		expiresAt: expiresAt.toLocaleDateString(undefined, {
			month: 'long',
			day: 'numeric',
			year: 'numeric'
		})
	};
}

/** The discounted price, or null when no discount is live. */
export function discountedPrice(org: Organization | null | undefined): string | null {
	const discount = saveOfferDiscount(org);
	if (!org?.plan || !discount) return null;
	return ((org.plan.base_cents * (100 - discount.percentOff)) / 100 / 100).toFixed(2);
}

/** The amount the card is charged on the next invoice. */
export function priceToCharge(org: Organization | null | undefined): string | null {
	if (!org?.plan) return null;
	return discountedPrice(org) ?? (org.plan.base_cents / 100).toFixed(2);
}
