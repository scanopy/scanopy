<script lang="ts">
	import { usePreloadStripe } from '$lib/features/billing/stripe.svelte';
	import { AlertTriangle } from '@lucide/svelte';
	import AppBanner from './AppBanner.svelte';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useHasPendingQuote } from '$lib/features/billing/queries';
	import { useExtendTrial } from '$lib/features/billing/extend-trial';
	import { useCan } from '$lib/features/auth/access';
	import { startSetupPayment } from '$lib/shared/billing/setup-payment';
	import {
		canExtendTrial,
		getTrialDaysLeft,
		isTrialingWithoutPayment
	} from '$lib/shared/utils/trial';
	import { hasLicensedPlan } from '$lib/features/organizations/types';
	import { trackOncePerSession } from '$lib/shared/utils/analytics';
	import { isCloud, useConfigQuery } from '$lib/shared/stores/config-query';
	import {
		billing_addPaymentMethod,
		billing_trialBannerBody,
		billing_trialBannerBodyOneDay,
		billing_trialBannerBodyToday,
		billing_trialBannerBodyLicense,
		billing_trialBannerBodyLicenseOneDay,
		billing_trialBannerBodyLicenseToday,
		billing_trialBannerExtend
	} from '$lib/paraglide/messages';

	const organizationQuery = useOrganizationQuery();
	const configQuery = useConfigQuery();
	const can = useCan();
	const extendTrial = useExtendTrial();

	let org = $derived(organizationQuery.data);
	let isCloudDeployment = $derived(configQuery.data ? isCloud(configQuery.data) : false);
	let trialDaysLeft = $derived(getTrialDaysLeft(org));
	// A quote out means payment is already being arranged; accepting it is what
	// ends the trial, and that lives on the Billing tab.
	const hasPendingQuote = useHasPendingQuote();
	let shouldShow = $derived(
		isTrialingWithoutPayment(org, isCloudDeployment) &&
			!hasPendingQuote.current &&
			trialDaysLeft !== null &&
			trialDaysLeft <= 3
	);

	// The one self-serve extension, offered beside the card prompt to whoever
	// can act on billing.
	let showExtend = $derived(can('Billing', 'Write') && canExtendTrial(org, trialDaysLeft));

	// A licensed trial ends in a stopped license key, not a downgraded cloud account.
	let isLicensed = $derived(org != null && hasLicensedPlan(org));
	let body = $derived.by(() => {
		if (trialDaysLeft === null) return '';
		if (trialDaysLeft <= 0)
			return isLicensed ? billing_trialBannerBodyLicenseToday() : billing_trialBannerBodyToday();
		if (trialDaysLeft === 1)
			return isLicensed ? billing_trialBannerBodyLicenseOneDay() : billing_trialBannerBodyOneDay();
		return isLicensed
			? billing_trialBannerBodyLicense({ days: trialDaysLeft })
			: billing_trialBannerBody({ days: trialDaysLeft });
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

	// Stripe.js is ready before anyone opens the payment dialog from here.
	usePreloadStripe(() => shouldShow);
</script>

{#if shouldShow}
	<AppBanner variant="warning" icon={AlertTriangle} {body}>
		{#snippet actions()}
			<button
				onclick={handleCta}
				class="ml-2 rounded-sm px-2 py-0.5 text-xs font-medium underline hover:no-underline disabled:opacity-50"
			>
				{billing_addPaymentMethod()}
			</button>
			{#if showExtend}
				<button
					onclick={extendTrial.extend}
					disabled={extendTrial.isPending}
					class="ml-2 rounded-sm px-2 py-0.5 text-xs underline opacity-80 hover:no-underline hover:opacity-100 disabled:opacity-50"
				>
					{billing_trialBannerExtend()}
				</button>
			{/if}
		{/snippet}
	</AppBanner>
{/if}
