/**
 * Whether Settings shows the License tab. SettingsModal filters its tab list with
 * this, and the billing lock in routes/+page.svelte uses it to pick the tab a
 * non-dismissible Settings modal opens on, so the two can't disagree.
 *
 * - `isOwner`: license keys and the card behind them are owner-only.
 * - `isDemoOrg`: a demo org never holds a license. Its owner can still open the
 *   plan picker, so the pending window below needs this gate too.
 * - `billingEnabled`: with billing off the server sells no licenses through
 *   Stripe (a self-hosted instance), so there is nothing to show.
 * - `signingAvailable`: no signing key means the mint endpoints fail, so the tab
 *   would only produce an error toast.
 * - `licensedPlanPending`: covers the gap between picking a licensed plan and the
 *   Stripe webhook landing, when `hasLicensedPlan` is still false.
 *
 * Takes booleans so it can be tested without the metadata store.
 */
export function licenseTabVisible(s: {
	isOwner: boolean;
	isDemoOrg: boolean;
	billingEnabled: boolean;
	signingAvailable: boolean;
	hasLicensedPlan: boolean;
	licensedPlanPending: boolean;
}): boolean {
	return (
		s.isOwner &&
		!s.isDemoOrg &&
		s.billingEnabled &&
		s.signingAvailable &&
		(s.licensedPlanPending || s.hasLicensedPlan)
	);
}
