import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import { queryClient, queryKeys } from '#lib/api/query-client.ts';
import type { Organization } from '#lib/features/organizations/types.ts';

/**
 * Determines the correct route for an authenticated user based on their state
 */
export function getRoute(): string {
	const organization = queryClient.getQueryData<Organization | null>(
		queryKeys.organizations.current()
	);

	if (!organization) {
		return resolve('onboarding');
	}

	// Org exists - go to main app
	// Billing plan selection is handled by the modal on the main page
	return resolve('/');
}

/**
 * Navigate to the appropriate route after authentication
 */
export async function navigate(): Promise<void> {
	const route = getRoute();
	await goto(route);
}

/**
 * Navigate to the appropriate route with a modal query param
 */
export async function navigateWithModal(
	modalName: string,
	opts?: { id?: string; tab?: string }
): Promise<void> {
	const route = getRoute();
	const url = new URL(route, window.location.origin);
	url.searchParams.set('modal', modalName);
	if (opts?.id) url.searchParams.set('id', opts.id);
	if (opts?.tab) url.searchParams.set('tab', opts.tab);
	await goto(url.pathname + url.search);
}
