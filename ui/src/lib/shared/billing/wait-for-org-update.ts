/**
 * Poll the organization payload until a predicate is satisfied, or give
 * up after a bounded number of attempts.
 *
 * Backend billing mutations are webhook-driven (Pattern A): the API call
 * returns before Stripe's `customer.subscription.updated` webhook lands
 * and our subscriber writes the new state. A single post-mutation
 * `refetch()` therefore races the webhook and reads stale data. Callers
 * pass a predicate describing the expected post-state (`o => o.plan_status
 * === 'active'`) and this helper invalidates + refetches every
 * `intervalMs` until the predicate matches.
 *
 * Returns `true` if the predicate matched within the window, `false` if
 * we hit `maxAttempts` without seeing the expected state. Callers decide
 * what to do on timeout — typically a quiet "may take a moment to
 * reflect" toast plus a final refetch, leaving the next manual refresh to
 * land the truth.
 */
import { queryClient, queryKeys } from '#lib/api/query-client.ts';
import { fetchOrganization } from '#lib/features/organizations/queries.ts';
import type { Organization } from '#lib/features/organizations/types.ts';

export interface WaitForOrgUpdateOptions {
	/** Max number of poll attempts. Defaults to 10. */
	maxAttempts?: number;
	/** Delay between attempts in milliseconds. Defaults to 2000. */
	intervalMs?: number;
}

export async function waitForOrgUpdate(
	predicate: (org: Organization) => boolean,
	opts: WaitForOrgUpdateOptions = {}
): Promise<boolean> {
	const { maxAttempts = 10, intervalMs = 2000 } = opts;
	for (let i = 0; i < maxAttempts; i++) {
		await queryClient.invalidateQueries({ queryKey: queryKeys.organizations.current() });
		try {
			const org = await fetchOrganization();
			if (predicate(org)) return true;
		} catch {
			// Transient fetch failure — retry on next tick.
		}
		await new Promise((r) => setTimeout(r, intervalMs));
	}
	return false;
}
