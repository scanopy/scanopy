import type { components } from '#lib/api/schema';

// Re-export generated types
export type BillingPlan = components['schemas']['BillingPlan'];
export type BillingRate = components['schemas']['BillingRate'];
export type PlanRecommendations = components['schemas']['PlanRecommendations'];
export type PlanRecommendation = components['schemas']['PlanRecommendation'];

/** The recommendation that names `planType`, on whichever hosting tab it sits. */
export function recommendationFor(
	recommendations: PlanRecommendations | null | undefined,
	planType: string
): PlanRecommendation | null {
	return (
		[recommendations?.cloud, recommendations?.self_hosted].find((r) => r?.plan === planType) ?? null
	);
}

/** Which tab of the plan picker is shown (UI state, not a backend enum). */
export type PlanPickerHosting = 'cloud' | 'self_hosted';

export function formatPrice(cents: number, rate: string): string {
	return `${cents / 100} per ${rate}`;
}
