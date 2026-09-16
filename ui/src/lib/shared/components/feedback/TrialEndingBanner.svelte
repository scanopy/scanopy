<script lang="ts">
	import { AlertTriangle } from 'lucide-svelte';
	import AppBanner from './AppBanner.svelte';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { startSetupPayment } from '$lib/shared/billing/setup-payment';
	import { getTrialDaysLeft, isTrialingWithoutPayment } from '$lib/shared/utils/trial';
	import { trackOncePerSession } from '$lib/shared/utils/analytics';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import {
		billing_addPaymentMethod,
		billing_trialBannerBody,
		billing_trialBannerBodyOneDay,
		billing_trialBannerBodyToday
	} from '$lib/paraglide/messages';

	const organizationQuery = useOrganizationQuery();
	const configQuery = useConfigQuery();

	let org = $derived(organizationQuery.data);
	let billingEnabled = $derived(configQuery.data?.billing_enabled ?? false);
	let trialDaysLeft = $derived(getTrialDaysLeft(org));
	let shouldShow = $derived(
		isTrialingWithoutPayment(org, billingEnabled) && trialDaysLeft !== null && trialDaysLeft <= 3
	);

	let body = $derived.by(() => {
		if (trialDaysLeft === null) return '';
		if (trialDaysLeft <= 0) return billing_trialBannerBodyToday();
		if (trialDaysLeft === 1) return billing_trialBannerBodyOneDay();
		return billing_trialBannerBody({ days: trialDaysLeft });
	});

	$effect(() => {
		if (shouldShow && trialDaysLeft !== null) {
			trackOncePerSession('trial_banner_shown', 'trial_banner_shown', {
				trial_days_left: trialDaysLeft
			});
		}
	});

	function handleCta() {
		startSetupPayment({ org, source: 'trial_banner', trialDaysLeft });
	}
</script>

{#if shouldShow}
	<AppBanner variant="warning" icon={AlertTriangle} {body}>
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
