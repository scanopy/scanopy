import type { components } from '$lib/api/schema';

// Re-export generated types
export type BillingPlan = components['schemas']['BillingPlan'];
export type BillingRate = components['schemas']['BillingRate'];

/** Which tab of the plan picker is shown (UI state, not a backend enum). */
export type PlanPickerHosting = 'cloud' | 'self_hosted';

export function formatPrice(cents: number, rate: string): string {
	return `${cents / 100} per ${rate}`;
}
