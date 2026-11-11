<script lang="ts">
	import { usePreloadStripe } from '#lib/features/billing/stripe.svelte.ts';
	import { CreditCard } from '@lucide/svelte';
	import AppBanner from './AppBanner.svelte';
	import { useOrganizationQuery } from '#lib/features/organizations/queries.ts';
	import { useHasPendingQuote } from '#lib/features/billing/queries.ts';
	import { startSetupPayment } from '#lib/shared/billing/setup-payment.ts';
	import {
		getTrialDaysLeft,
		isTrialingWithoutPayment,
		isMissingPaymentMethod
	} from '#lib/shared/utils/trial.ts';
	import { trackOncePerSession } from '#lib/shared/utils/analytics.ts';
	import { isCloud, useConfigQuery } from '#lib/shared/stores/config-query.ts';
	import {
		billing_addPaymentMethod,
		billing_noPaymentMethodBannerBody
	} from '#lib/paraglide/messages.js';

	const organizationQuery = useOrganizationQuery();
	const configQuery = useConfigQuery();

	let org = $derived(organizationQuery.data);
	let isCloudDeployment = $derived(configQuery.data ? isCloud(configQuery.data) : false);
	let trialDaysLeft = $derived(getTrialDaysLeft(org));

	// `isMissingPaymentMethod` is the shared predicate (Stripe-managed plan that
	// requires a card but has none) used by every payment-method nag so they
	// stay in sync. Two states are left to other surfaces:
	// - A trial without a card belongs to TrialEndingBanner, which shows in its
	//   last three days. Before then the trial date on the Billing and License
	//   tabs and the Billing tab dot are the ask.
	// - A past-due org sees the Billing tab's past-due notice, which says more.
	// An org with a quote out is already arranging payment, and accepting it is
	// the Billing tab's primary action, so this would talk it out of that.
	const hasPendingQuote = useHasPendingQuote();
	let shouldShow = $derived(
		isMissingPaymentMethod(org, isCloudDeployment) &&
			!hasPendingQuote.current &&
			!isTrialingWithoutPayment(org, isCloudDeployment) &&
			org?.plan_status !== 'past_due'
	);

	$effect(() => {
		if (shouldShow) {
			trackOncePerSession('no_payment_banner_shown', 'no_payment_banner_shown', {
				plan_status: org?.plan_status
			});
		}
	});

	function handleCta() {
		startSetupPayment({ org, source: 'no_payment_banner', trialDaysLeft });
	}

	// Stripe.js is ready before anyone opens the payment dialog from here.
	usePreloadStripe(() => shouldShow);
</script>

{#if shouldShow}
	<AppBanner variant="warning" icon={CreditCard} body={billing_noPaymentMethodBannerBody()}>
		{#snippet actions()}
			<button
				onclick={handleCta}
				class="ml-2 rounded-sm px-2 py-0.5 text-xs font-medium underline hover:no-underline disabled:opacity-50"
			>
				{billing_addPaymentMethod()}
			</button>
		{/snippet}
	</AppBanner>
{/if}
