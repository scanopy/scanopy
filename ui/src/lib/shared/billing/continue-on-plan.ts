import type { Organization } from '$lib/features/organizations/types';
import { canPay } from '$lib/shared/utils/trial';
import { startSetupPayment } from './setup-payment';
import { waitForOrgUpdate } from './wait-for-org-update';

interface ContinueOnLapsedPlanArgs {
	org: Organization | null | undefined;
	source: 'billing_tab' | 'license_tab';
	trialDaysLeft: number | null;
	/** The checkout mutation's `mutateAsync`; it toasts its own failures. */
	checkout: (plan: NonNullable<Organization['plan']>) => Promise<string>;
}

/**
 * Re-subscribe a lapsed org to the plan it lapsed from. With a card on file
 * the backend creates the subscription in place and the org is polled until
 * the webhook lands it; without one the payment dialog collects the card and
 * buys the plan on success. The Billing and License tabs both offer this.
 */
export async function continueOnLapsedPlan({
	org,
	source,
	trialDaysLeft,
	checkout
}: ContinueOnLapsedPlanArgs): Promise<void> {
	const plan = org?.plan;
	if (!org || !plan) return;
	if (!canPay(org)) {
		startSetupPayment({ org, plan, source, trialDaysLeft });
		return;
	}
	try {
		const result = await checkout(plan);
		if (result.startsWith('http')) {
			window.location.href = result;
			return;
		}
		await waitForOrgUpdate((o) => o.plan_status === 'active', { intervalMs: 500 });
	} catch {
		// The mutation toasts the failure.
	}
}
