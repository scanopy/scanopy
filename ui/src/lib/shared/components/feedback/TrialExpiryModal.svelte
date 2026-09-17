<script lang="ts">
	import { AlertTriangle, CreditCard } from 'lucide-svelte';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { startSetupPayment } from '$lib/shared/billing/setup-payment';
	import { getTrialDaysLeft, isTrialingWithoutPayment } from '$lib/shared/utils/trial';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import { wasDismissedToday, markDismissedToday } from '$lib/shared/utils/dismissed-today';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import {
		billing_addPaymentMethod,
		billing_remindMeLater,
		billing_trialModalBody,
		billing_trialModalTitleToday,
		billing_trialModalTitleTomorrow
	} from '$lib/paraglide/messages';

	const DISMISS_KEY = 'trial_expiry_modal';

	const organizationQuery = useOrganizationQuery();
	const configQuery = useConfigQuery();

	let org = $derived(organizationQuery.data);
	let billingEnabled = $derived(configQuery.data?.billing_enabled ?? false);
	let trialDaysLeft = $derived(getTrialDaysLeft(org));

	let dismissedTick = $state(0);

	let dismissedTodayState = $derived.by(() => {
		// Re-read on each tick so handleDismiss triggers a recompute.
		void dismissedTick;
		return wasDismissedToday(DISMISS_KEY);
	});

	let isOpen = $derived(
		isTrialingWithoutPayment(org, billingEnabled) &&
			trialDaysLeft !== null &&
			trialDaysLeft <= 1 &&
			!dismissedTodayState
	);

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
</script>

<GenericModal {title} {isOpen} onClose={handleDismiss} size="md" preventCloseOnClickOutside={true}>
	{#snippet headerIcon()}
		<AlertTriangle class="h-5 w-5 text-amber-500" />
	{/snippet}
	<div class="px-6 py-4">
		<p class="text-secondary text-sm">{billing_trialModalBody()}</p>
	</div>
	{#snippet footer()}
		<div class="modal-footer flex justify-end gap-2">
			<button type="button" class="btn-secondary text-sm" onclick={handleDismiss}>
				{billing_remindMeLater()}
			</button>
			<button
				type="button"
				class="btn-primary flex items-center gap-1.5 text-sm"
				onclick={handleCta}
			>
				<CreditCard size={14} />
				{billing_addPaymentMethod()}
			</button>
		</div>
	{/snippet}
</GenericModal>
