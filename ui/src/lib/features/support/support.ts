import type { BillingPlanFeatures } from '$lib/shared/stores/metadata';

/** Booking page for demos and plan onboarding calls. */
export const BOOK_CALL_URL = 'https://cal.com/mferrandiz/scanopy-demo';

/** A mailto link to billing@ with the subject and body filled in. */
export function billingMailto(subject: string, body: string): string {
	return `mailto:billing@scanopy.net?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;
}

/**
 * The support address a plan's email card sends to, or `null` when the plan
 * has no email support. Priority-support plans get their own alias.
 */
export function supportEmailAddress(
	features: Pick<BillingPlanFeatures, 'email_support' | 'priority_support'> | undefined
): string | null {
	if (!features?.email_support) return null;
	return features.priority_support ? 'priority@scanopy.net' : 'support@scanopy.net';
}
