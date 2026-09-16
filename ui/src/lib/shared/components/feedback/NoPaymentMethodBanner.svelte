<script lang="ts">
	import { CreditCard } from 'lucide-svelte';
	import AppBanner from './AppBanner.svelte';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { startSetupPayment } from '$lib/shared/billing/setup-payment';
	import {
		getTrialDaysLeft,
		isTrialingWithoutPayment,
		isMissingPaymentMethod
	} from '$lib/shared/utils/trial';
	import { trackOncePerSession } from '$lib/shared/utils/analytics';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import {
		billing_addPaymentMethod,
		billing_noPaymentMethodBannerBody
	} from '$lib/paraglide/messages';

	const organizationQuery = useOrganizationQuery();
	const configQuery = useConfigQuery();

	let org = $derived(organizationQuery.data);
	let billingEnabled = $derived(configQuery.data?.billing_enabled ?? false);
	let trialDaysLeft = $derived(getTrialDaysLeft(org));

	// `isMissingPaymentMethod` is the shared predicate (Stripe-managed plan that
	// requires a card but has none) used by every payment-method nag so they
	// stay in sync. The final clause defers to TrialEndingBanner in its window
	// (trialing + no card + <= 3 days) so the two never render together.
	let shouldShow = $derived(
		isMissingPaymentMethod(org, billingEnabled) &&
			!(
				isTrialingWithoutPayment(org, billingEnabled) &&
				trialDaysLeft !== null &&
				trialDaysLeft <= 3
			)
	);

	$effect(() => {
		if (shouldShow) {
			trackOncePerSession('no_payment_banner_shown', 'no_payment_banner_shown', {
				plan_status: org?.plan_status
			});
		}
	});

	function handleCta() {
		startSetupPayment({ org, source: 'trial_banner', trialDaysLeft });
	}
</script>

{#if shouldShow}
	<AppBanner variant="warning" icon={CreditCard} body={billing_noPaymentMethodBannerBody()}>
		{#snippet actions()}
			<button
				onclick={handleCta}
				class="ml-2 rounded px-2 py-0.5 text-xs font-medium underline hover:no-underline disabled:opacity-50"
			>
				{billing_addPaymentMethod()}
			</button>
		{/snippet}
	</AppBanner>
{/if}
