<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import type { Snippet } from 'svelte';
	import { queryClient, queryKeys } from '#lib/api/query-client.ts';
	import { useCurrentUserQuery } from '#lib/features/auth/queries.ts';
	import { fetchOrganization, useOrganizationQuery } from '#lib/features/organizations/queries.ts';
	import {
		identifyUser,
		trackEvent,
		flushEventQueue,
		flushStoredEvents
	} from '#lib/shared/utils/analytics.ts';
	import Loading from '#lib/shared/components/feedback/Loading.svelte';
	import { resolve } from '$app/paths';
	import { resetTopologyOptions } from '#lib/features/topology/queries.ts';
	import { pushError, pushSuccess } from '#lib/shared/stores/feedback.ts';
	import { useConfigQuery } from '#lib/shared/stores/config-query.ts';
	import { displaySettings } from '#lib/shared/stores/display-settings.svelte.ts';
	import {
		hasLicensedPlan,
		isBillingPlanActive,
		isPaidSubscriptionActive
	} from '#lib/features/organizations/types.ts';
	import { getRoute } from '#lib/shared/utils/navigation.ts';
	import type { PostHog } from 'posthog-js';
	import { browser } from '$app/env';
	import CookieConsent, {
		hasAnalyticsConsent
	} from '#lib/shared/components/feedback/CookieConsent.svelte';
	import {
		applyAttributionToPosthog,
		markLandingPageviewDropped
	} from '#lib/shared/utils/first-touch.ts';
	import PaymentMethodModal from '#lib/features/billing/PaymentMethodModal.svelte';
	import LoadFailedState from './LoadFailedState.svelte';
	import ServerUnreachableBanner from '#lib/shared/components/feedback/ServerUnreachableBanner.svelte';
	import { serverUnreachable } from '#lib/api/server-reachability.ts';
	import {
		billing_paymentMethodAdded,
		billing_subscriptionActivated,
		billing_subscriptionDelayed
	} from '#lib/paraglide/messages.js';
	import { markPlanActivated } from '#lib/shared/billing/plan-activation-marker.ts';
	import { waitForOrgUpdate } from '#lib/shared/billing/wait-for-org-update.ts';

	let { children }: { children: Snippet } = $props();

	// TanStack Query for current user
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);
	let isAuthenticated = $derived(currentUser != null);
	let isCheckingAuth = $derived(currentUserQuery.isPending);
	// Only an answer from the server settles who is signed in: `null` is its 401. A first load
	// that couldn't reach the server has no answer, and must neither route to the login page nor
	// count as a logout.
	let authCheckComplete = $derived(currentUser !== undefined);
	let authUnreachable = $derived(currentUserQuery.isError && currentUser === undefined);
	// While the server is down the unreachable banner shows above a spinner, and the recovery
	// refetch brings the app up by itself. Only an error the server sent shows the failed state.
	let authWaitingForServer = $derived(
		authUnreachable && ($serverUnreachable || currentUserQuery.isFetching)
	);

	// Dates everywhere follow the signed-in user's display settings; defaults when signed out.
	$effect(() => {
		displaySettings.set(currentUser?.display_settings);
	});

	// TanStack Query for organization
	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	// Only a license plan has a key. When the org is on any other plan, at load or
	// after a plan change, drop any key still cached from an earlier license plan.
	$effect(() => {
		if (organization && !hasLicensedPlan(organization)) {
			queryClient.removeQueries({ queryKey: queryKeys.licenses.all });
		}
	});

	// TanStack Query for config
	const configQuery = useConfigQuery();
	let configData = $derived(configQuery.data);

	// Track if we've done initial setup
	let hasInitialized = $state(false);
	let previouslyAuthenticated = $state<boolean | null>(null);
	let handlingStripeReturn = $state(false);

	// Effect to handle logout (clear data when user goes from authenticated to not)
	$effect(() => {
		if (authCheckComplete) {
			if (previouslyAuthenticated === true && !isAuthenticated) {
				// User logged out - clear data
				resetTopologyOptions();
				queryClient.clear();
				// Route again: a session that expired mid-use leaves the page behind it with no
				// data, and the routing below runs only once unless reset.
				hasInitialized = false;
			}
			previouslyAuthenticated = isAuthenticated;
		}
	});

	let posthogInstance = $state<PostHog | null>(null);
	let posthogInitStarted = false;

	$effect(() => {
		if (!configData) return;

		const posthogKey = configData.posthog_key;

		const isShareRoute = page.url.pathname.startsWith('/share/');

		if (browser && posthogKey && !posthogInitStarted && !isShareRoute) {
			posthogInitStarted = true;
			// Lazy-load posthog-js to avoid blocking initial bundle
			import('posthog-js').then(({ default: posthog }) => {
				posthog.init(posthogKey, {
					api_host: 'https://ph.scanopy.net',
					ui_host: 'https://us.posthog.com',
					defaults: '2025-11-30',
					secure_cookie: true,
					persistence: 'localStorage+cookie',
					opt_out_capturing_by_default: !hasAnalyticsConsent(),
					opt_out_capturing_persistence_type: 'localStorage', // Respect opt-out choice
					capture_pageview: true,
					capture_pageleave: true,

					// Don't auto-identify until consent
					person_profiles: 'identified_only', // Only create person profiles after identify

					loaded: () => {
						posthogInstance = posthog;
						// Close the consent race: the user may have accepted the banner
						// while posthog-js was still importing, in which case the panel's
						// opt_in_capturing() call was skipped.
						if (hasAnalyticsConsent() && posthog.has_opted_out_capturing()) {
							posthog.opt_in_capturing();
						}
						if (posthog.has_opted_out_capturing()) {
							// The landing $pageview was discarded; remember so it can be
							// replayed with the true landing URL if the user opts in.
							markLandingPageviewDropped();
						} else {
							applyAttributionToPosthog(posthog);
						}
						flushEventQueue();
						flushStoredEvents();
					}
				});
			});
		}
	});

	// Identify user in PostHog when authenticated (skipped in demo mode by identifyUser)
	$effect(() => {
		if (posthogInstance && currentUser && organization !== undefined) {
			identifyUser(currentUser.id, currentUser.email, organization);
		}
	});

	async function waitForBillingActivation(maxAttempts = 10) {
		const ok = await waitForOrgUpdate(isBillingPlanActive, { maxAttempts });

		if (ok) {
			const orgData = await fetchOrganization();
			trackEvent('billing_completed', {
				plan: orgData?.plan?.type ?? 'unknown',
				amount: orgData?.plan?.base_cents ?? 0,
				plan_status: orgData?.plan_status
			});
			markPlanActivated();
			// Only claim "activated" for a genuine paid subscription — not for
			// downgrade-to-Free, pause, past_due, etc. (isBillingPlanActive, the
			// poll target, is true for all of those).
			if (orgData && isPaidSubscriptionActive(orgData)) {
				pushSuccess(billing_subscriptionActivated());
			}
			return true;
		}

		pushError(billing_subscriptionDelayed());
		return false;
	}

	// Handle routing after auth check completes
	$effect(() => {
		if (!authCheckComplete || hasInitialized) return;
		if (!browser) return;

		hasInitialized = true;

		// Check for OIDC error in URL
		const error = page.url.searchParams.get('error');
		if (error) {
			pushError(decodeURIComponent(error));
		}

		// Store last login method from OIDC success redirect
		const loginMethod = page.url.searchParams.get('login_method');
		if (loginMethod) {
			localStorage.setItem('scanopy_last_login_method', loginMethod);
		}

		// Clean up query params
		if (error || loginMethod) {
			const cleanUrl = new URL(page.url.href);
			cleanUrl.searchParams.delete('error');
			cleanUrl.searchParams.delete('login_method');
			window.history.replaceState({}, '', cleanUrl.toString());
		}

		if (!isAuthenticated) {
			// Not authenticated - redirect to login/onboarding if not on public route
			const isPublicRoute =
				page.url.pathname === '/auth' ||
				page.url.pathname === '/login' ||
				page.url.pathname === '/onboarding' ||
				page.url.pathname === '/verify-email' ||
				page.url.pathname.startsWith('/share/');

			if (!isPublicRoute) {
				const token = page.url.searchParams.get('token');
				const isDemo = page.url.hostname === 'demo.scanopy.net';
				if (token) {
					goto(`${resolve('login')}?token=${token}`);
				} else if (isDemo) {
					goto(resolve('login'));
				} else if (typeof localStorage !== 'undefined' && localStorage.getItem('hasAccount')) {
					// Preserve the query string so UTM params survive for analytics
					goto(`${resolve('login')}${page.url.search}`);
				} else {
					goto(`${resolve('onboarding')}${page.url.search}`);
				}
			}
		}
	});

	// Handle organization-dependent routing (runs after org data loads)
	$effect(() => {
		if (!authCheckComplete || !isAuthenticated || !browser) return;
		if (!organization) return;
		if (handlingStripeReturn) return;

		const billingFlow = page.url.searchParams.get('billing_flow');

		// Handle Stripe checkout callback (new subscription activation)
		if (billingFlow === 'checkout') {
			const cleanUrl = new URL(page.url.href);
			cleanUrl.searchParams.delete('billing_flow');
			window.history.replaceState({}, '', cleanUrl.toString());

			if (isBillingPlanActive(organization)) {
				// Webhook already processed — fire event directly
				trackEvent('billing_completed', {
					plan: organization.plan?.type ?? 'unknown',
					amount: organization.plan?.base_cents ?? 0,
					plan_status: organization.plan_status
				});
				markPlanActivated();
				// Only claim "activated" for a genuine paid subscription — a
				// checkout return that lands on Free/paused/past_due shouldn't
				// say "Subscription activated successfully!".
				if (isPaidSubscriptionActive(organization)) {
					pushSuccess(billing_subscriptionActivated());
				}
			} else {
				// Webhook hasn't processed yet — poll until activation
				handlingStripeReturn = true;
				waitForBillingActivation()
					.then((activated) => {
						if (activated) {
							goto(getRoute());
						}
					})
					.finally(() => {
						handlingStripeReturn = false;
					});
				return;
			}
		} else if (billingFlow === 'payment_setup') {
			trackEvent('payment_method_setup_completed', {
				plan_type: organization.plan?.type,
				plan_status: organization.plan_status
			});

			const cleanUrl = new URL(page.url.href);
			cleanUrl.searchParams.delete('billing_flow');
			window.history.replaceState({}, '', cleanUrl.toString());

			markPlanActivated();
			pushSuccess(billing_paymentMethodAdded());

			// Refresh org data to update has_payment_method
			queryClient.invalidateQueries({ queryKey: queryKeys.organizations.current() });
		}

		// Check if current page matches where user should be
		// Skip routing check for share pages and onboarding (which handles its own navigation)
		const isSharePage = page.url.pathname.startsWith('/share/');
		const isOnboardingPage = page.url.pathname === '/onboarding';
		if (!isSharePage && !isOnboardingPage) {
			const correctRoute = getRoute();
			if (page.url.pathname !== correctRoute) {
				goto(correctRoute);
			}
		}
	});
</script>

{#if (isCheckingAuth && !page.url.pathname.startsWith('/onboarding')) || authWaitingForServer}
	<div class="flex min-h-screen flex-col bg-(--color-bg-elevated)">
		<ServerUnreachableBanner />
		<div class="flex flex-1 items-center justify-center">
			<Loading />
		</div>
	</div>
{:else if authUnreachable}
	<div class="flex min-h-screen items-center justify-center bg-(--color-bg-elevated)">
		<LoadFailedState onRetry={() => currentUserQuery.refetch()} />
	</div>
{:else}
	{@render children()}
{/if}

{#if configData && configData.needs_cookie_consent && !page.url.pathname.startsWith('/share/')}
	<CookieConsent />
{/if}

<!-- Global in-app card dialog, opened by any "Add/Update payment method" nudge -->
<PaymentMethodModal />
