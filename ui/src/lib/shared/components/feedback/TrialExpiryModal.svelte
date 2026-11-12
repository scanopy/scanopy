<script lang="ts">
	import { usePreloadStripe } from '#lib/features/billing/stripe.svelte.ts';
	import { AlertTriangle, CreditCard } from '@lucide/svelte';
	import GenericModal from '#lib/shared/components/layout/GenericModal.svelte';
	import { useOrganizationQuery } from '#lib/features/organizations/queries.ts';
	import { hasLicensedPlan } from '#lib/features/organizations/types.ts';
	import { shouldShowTrialExpiryModal } from '#lib/features/organizations/onload-gates.ts';
	import { useHasPendingQuote } from '#lib/features/billing/queries.ts';
	import { startSetupPayment } from '#lib/shared/billing/setup-payment.ts';
	import { getTrialDaysLeft, isTrialingWithoutPayment } from '#lib/shared/utils/trial.ts';
	import { isCloud, useConfigQuery } from '#lib/shared/stores/config-query.ts';
	import { wasDismissedToday, markDismissedToday } from '#lib/shared/utils/dismissed-today.ts';
	import { trackEvent } from '#lib/shared/utils/analytics.ts';
	import type { ModalAction } from '#lib/shared/components/layout/modalActions.ts';
	import {
		billing_addPaymentMethod,
		billing_remindMeLater,
		billing_trialModalBody,
		billing_trialModalBodyLicense,
		billing_trialModalTitleToday,
		billing_trialModalTitleTomorrow
	} from '#lib/paraglide/messages.js';

	const DISMISS_KEY = 'trial_expiry_modal';

	// `blocked` holds the modal back while another modal is up, so it never
	// stacks on one; `open` reports whether it is showing, so the page's other
	// app-load modals can wait for it.
	// eslint-disable-next-line no-useless-assignment -- `open` is bound by the parent, which reads it
	let { blocked = false, open = $bindable(false) }: { blocked?: boolean; open?: boolean } =
		$props();

	const organizationQuery = useOrganizationQuery();
	const configQuery = useConfigQuery();

	let org = $derived(organizationQuery.data);
	let isCloudDeployment = $derived(configQuery.data ? isCloud(configQuery.data) : false);
	let trialDaysLeft = $derived(getTrialDaysLeft(org));

	let dismissedTick = $state(0);

	let dismissedTodayState = $derived.by(() => {
		// Re-read on each tick so handleDismiss triggers a recompute.
		void dismissedTick;
		return wasDismissedToday(DISMISS_KEY);
	});

	// An org waiting on its own procurement to accept a quote should not be
	// stopped by a modal asking for a card it has chosen not to use.
	const hasPendingQuote = useHasPendingQuote();

	let isOpen = $derived(
		shouldShowTrialExpiryModal({
			trialingWithoutPayment: isTrialingWithoutPayment(org, isCloudDeployment),
			hasPendingQuote: hasPendingQuote.current,
			pendingQuoteLoading: hasPendingQuote.loading,
			trialDaysLeft,
			dismissedToday: dismissedTodayState,
			blocked
		})
	);
	$effect(() => {
		open = isOpen;
	});

	let title = $derived(
		trialDaysLeft !== null && trialDaysLeft <= 0
			? billing_trialModalTitleToday()
			: billing_trialModalTitleTomorrow()
	);

	let alreadyShown = $state(false);
	$effect(() => {
		if (isOpen && !alreadyShown) {
			alreadyShown = true;
			trackEvent('trial_modal_shown', { trial_days_left: trialDaysLeft });
		}
	});

	function handleDismiss() {
		markDismissedToday(DISMISS_KEY);
		dismissedTick++;
		trackEvent('trial_modal_dismissed_today', { trial_days_left: trialDaysLeft });
	}

	function handleCta() {
		trackEvent('trial_modal_cta_clicked', { trial_days_left: trialDaysLeft });
		startSetupPayment({ org, source: 'trial_modal', trialDaysLeft });
	}

	let actions = $derived<ModalAction[]>([
		{ role: 'cancel', label: billing_remindMeLater(), onClick: handleDismiss },
		{ role: 'primary', label: billing_addPaymentMethod(), icon: CreditCard, onClick: handleCta }
	]);

	// Stripe.js is ready before anyone opens the payment dialog from here.
	usePreloadStripe(() => isOpen);
</script>

<GenericModal
	{title}
	{isOpen}
	onClose={handleDismiss}
	size="md"
	preventCloseOnClickOutside={true}
	{actions}
>
	{#snippet headerIcon()}
		<AlertTriangle class="h-5 w-5 text-amber-500" />
	{/snippet}
	<div class="px-6 py-4">
		<p class="text-sm text-secondary">
			{org && hasLicensedPlan(org) ? billing_trialModalBodyLicense() : billing_trialModalBody()}
		</p>
	</div>
</GenericModal>
