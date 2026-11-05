/** The conditions that decide what the app shows when a page loads. */

interface QueryOutcome {
	data: unknown;
	isError: boolean;
}

function isSettled(query: QueryOutcome): boolean {
	return query.data !== undefined || query.isError;
}

/**
 * Whether the app shell can paint. The org and config decide which modal opens on load, so the
 * shell and that modal paint together. A failed query still paints the shell, which has its own
 * handling for missing data.
 */
export function isShellReady(organization: QueryOutcome, config: QueryOutcome): boolean {
	return isSettled(organization) && isSettled(config);
}

/** Whether Settings opens for a licensed self-hosted plan it hasn't opened for yet. */
export function shouldOpenLicenseSettings(state: {
	locked: boolean;
	planType: string | null;
	openedFor: string | null;
	appInitialized: boolean;
	planPickerShowing: boolean;
	modalName: string | null;
}): boolean {
	return (
		state.locked &&
		state.planType != null &&
		state.openedFor !== state.planType &&
		state.appInitialized &&
		!state.planPickerShowing &&
		(state.modalName === null || state.modalName === 'settings')
	);
}

/** Whether a `billing-plan` deep link has to close. Only a loaded config can rule out Cloud. */
export function shouldCloseBillingPlanLink(state: {
	modalName: string | null;
	configLoaded: boolean;
	isCloudDeployment: boolean;
}): boolean {
	return state.modalName === 'billing-plan' && state.configLoaded && !state.isCloudDeployment;
}

/** Whether the trial expiry modal shows. It holds until the quote query has answered. */
export function shouldShowTrialExpiryModal(state: {
	trialingWithoutPayment: boolean;
	hasPendingQuote: boolean;
	pendingQuoteLoading: boolean;
	trialDaysLeft: number | null;
	dismissedToday: boolean;
	blocked: boolean;
}): boolean {
	return (
		state.trialingWithoutPayment &&
		!state.hasPendingQuote &&
		!state.pendingQuoteLoading &&
		state.trialDaysLeft !== null &&
		state.trialDaysLeft <= 1 &&
		!state.dismissedToday &&
		!state.blocked
	);
}
