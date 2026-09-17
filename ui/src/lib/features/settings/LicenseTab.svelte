<script lang="ts">
	import { Copy, RefreshCw } from 'lucide-svelte';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import ConfirmationDialog from '$lib/shared/components/feedback/ConfirmationDialog.svelte';
	import ToggleGroup from '$lib/features/billing/ToggleGroup.svelte';
	import {
		useCreateLicenseKeyMutation,
		useCurrentLicenseKeyQuery,
		useEndTrialMutation,
		useRegenerateLicenseKeyMutation
	} from '$lib/features/billing/queries';
	import { priceToCharge } from '$lib/features/billing/pricing';
	import { triggerUpgrade } from '$lib/features/billing/trigger-upgrade';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { hasLicensedPlan } from '$lib/features/organizations/types';
	import type { Organization } from '$lib/features/organizations/types';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import { getTrialDaysLeft, isMissingPaymentMethod } from '$lib/shared/utils/trial';
	import { pushSuccess, pushWarning } from '$lib/shared/stores/feedback';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { copyViaSelection } from '$lib/shared/utils/clipboard';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import { startSetupPayment } from '$lib/shared/billing/setup-payment';
	import { waitForOrgUpdate } from '$lib/shared/billing/wait-for-org-update';
	import type { components } from '$lib/api/schema';
	import {
		billing_addPaymentMethod,
		billing_requestAccepted,
		common_airGapped,
		common_close,
		common_continue,
		common_copied,
		common_copy,
		common_failedToCopy,
		common_license,
		common_never,
		common_online,
		common_tier,
		settings_billing_changePlan,
		settings_billing_license_addPaymentMethodSubtitle,
		settings_billing_license_airGappedNeedsCard,
		settings_billing_license_endTrialConfirm,
		settings_billing_license_keyLabel,
		settings_billing_license_keyTypeChanged,
		settings_billing_license_keyTypeLabel,
		settings_billing_license_lastCheckIn,
		settings_billing_license_offlineKeyUpsell,
		settings_billing_license_paidThrough,
		settings_billing_license_regenerate,
		settings_billing_license_regenerateConfirm,
		settings_billing_license_regenerateTitle,
		settings_billing_license_regenerated,
		settings_billing_license_switchConfirm,
		settings_billing_license_switchTitle,
		settings_billing_license_trialPaymentBody
	} from '$lib/paraglide/messages';

	type LicenseKeyType = components['schemas']['LicenseKeyType'];

	let {
		onClose,
		dismissible = true
	}: {
		onClose: () => void;
		dismissible?: boolean;
	} = $props();

	const organizationQuery = useOrganizationQuery();
	let org = $derived(organizationQuery.data);

	const createKeyMutation = useCreateLicenseKeyMutation();
	const regenerateMutation = useRegenerateLicenseKeyMutation();
	const endTrialMutation = useEndTrialMutation();

	// Wait for the real org to carry the plan: minting before the checkout webhook
	// lands returns 403 NotLicensed. The tab opens on a provisional signal from the
	// plan picker, so this can be false for the first second or two.
	const keyQuery = useCurrentLicenseKeyQuery(() => org != null && hasLicensedPlan(org));
	let currentKey = $derived(keyQuery.data?.key ?? null);
	let keyType = $derived<LicenseKeyType>(keyQuery.data?.key_type ?? 'Online');
	let maskedKey = $derived(currentKey ? `${currentKey.slice(0, 8)}${'•'.repeat(32)}` : '');

	let planType = $derived(org?.plan?.type ?? null);
	let airGappedIncluded = $derived(
		billingPlans.getMetadata(planType).features?.air_gapped_deployment === true
	);
	let hasCard = $derived(org?.has_payment_method ?? false);
	let isTrialing = $derived(org?.plan_status === 'trialing');
	// The server also refuses an air-gapped mint until the subscription is out of
	// trial, but choosing the option during a trial is what ends the trial. A card
	// on file is the one thing the user has to do first.
	let airGappedAvailable = $derived(airGappedIncluded && hasCard);
	let missingCard = $derived(isMissingPaymentMethod(org));
	let chargeAmount = $derived(priceToCharge(org));

	let keyTypeOptions = $derived([
		{ value: 'Online', label: common_online() },
		{
			value: 'Offline',
			label: common_airGapped(),
			disabled: !airGappedAvailable,
			tooltip:
				airGappedIncluded && !hasCard ? settings_billing_license_airGappedNeedsCard() : undefined
		}
	]);

	let paidThrough = $derived(
		org?.license_paid_through
			? new Date(org.license_paid_through).toLocaleDateString(undefined, {
					month: 'long',
					day: 'numeric',
					year: 'numeric'
				})
			: null
	);
	let lastCheckIn = $derived(
		org?.license_checkin_at ? formatTimestamp(org.license_checkin_at) : common_never()
	);

	let trialEndsOn = $derived(
		org?.trial_end_date
			? new Date(org.trial_end_date).toLocaleDateString(undefined, {
					month: 'long',
					day: 'numeric',
					year: 'numeric'
				})
			: null
	);
	let cardDescription = $derived(
		trialEndsOn
			? settings_billing_license_trialPaymentBody({ date: trialEndsOn })
			: settings_billing_license_addPaymentMethodSubtitle()
	);

	let showRegenerateConfirm = $state(false);
	let pendingType = $state<LicenseKeyType | null>(null);
	let switching = $state(false);

	let switchConfirmMessage = $derived.by(() => {
		const body = settings_billing_license_switchConfirm();
		if (pendingType === 'Offline' && isTrialing && chargeAmount != null) {
			return `${settings_billing_license_endTrialConfirm({ amount: chargeAmount })} ${body}`;
		}
		return body;
	});

	async function copyKey(key: string, type: LicenseKeyType) {
		try {
			if (window.isSecureContext && navigator.clipboard) {
				await navigator.clipboard.writeText(key);
			} else if (!copyViaSelection(key)) {
				// Plain-HTTP self-hosts have no navigator.clipboard; the selection copy is
				// the fallback, and it can still be refused.
				throw new Error('the browser refused the copy');
			}
			pushSuccess(common_copied());
			trackEvent('license_key_copied', { key_type: type });
		} catch (error) {
			pushWarning(common_failedToCopy({ error: String(error) }));
		}
	}

	async function handleRegenerate() {
		showRegenerateConfirm = false;
		try {
			await regenerateMutation.mutateAsync();
			pushSuccess(settings_billing_license_regenerated());
			trackEvent('license_key_regenerated');
		} catch {
			// The API client toasts the error.
		}
	}

	function handleTypeChange(value: string) {
		const next = value as LicenseKeyType;
		if (switching || next === keyType) return;
		pendingType = next;
	}

	/** The invoice for the first cycle has been paid, past whatever the trial covered. */
	function paidPastTrial(o: Organization): boolean {
		if (!o.license_paid_through) return false;
		if (!o.trial_end_date) return true;
		return new Date(o.license_paid_through).getTime() > new Date(o.trial_end_date).getTime();
	}

	async function handleConfirmSwitch() {
		const target = pendingType;
		pendingType = null;
		if (!target) return;
		switching = true;
		try {
			if (target === 'Offline' && isTrialing) {
				// The server refuses an air-gapped key until the subscription is paid, so
				// charge first and let the webhooks land before asking for the key.
				await endTrialMutation.mutateAsync();
				const converged = await waitForOrgUpdate(
					(o) => o.plan_status === 'active' && paidPastTrial(o)
				);
				if (!converged) {
					pushWarning(billing_requestAccepted());
					return;
				}
			}
			await createKeyMutation.mutateAsync(target);
			pushSuccess(settings_billing_license_keyTypeChanged());
			trackEvent('license_key_type_changed', { key_type: target });
		} catch {
			// The API client toasts the failure.
		} finally {
			switching = false;
		}
	}

	function handleAddPaymentMethod() {
		startSetupPayment({ org, source: 'license_tab', trialDaysLeft: getTrialDaysLeft(org) });
	}

	function openPlanPicker() {
		triggerUpgrade({
			source: 'settings_license',
			surface: 'billing_tab',
			reopenSettings: true,
			beforeModal: () => onClose()
		});
	}
</script>

<div class="flex min-h-0 flex-1 flex-col">
	<div class="flex-1 overflow-auto p-6">
		{#if org}
			<div class="space-y-6">
				<InfoCard title={common_license()}>
					<div class="space-y-4">
						<dl class="grid grid-cols-[auto_1fr] gap-x-6 gap-y-1 text-sm">
							<dt class="text-secondary">{common_tier()}</dt>
							<dd class="text-primary font-medium">{billingPlans.getName(planType)}</dd>
							{#if paidThrough}
								<dt class="text-secondary">{settings_billing_license_paidThrough()}</dt>
								<dd class="text-primary">{paidThrough}</dd>
							{/if}
							<dt class="text-secondary">{settings_billing_license_lastCheckIn()}</dt>
							<dd class="text-primary">{lastCheckIn}</dd>
						</dl>

						<div class="space-y-3 border-t pt-3" style="border-color: var(--color-border)">
							<div>
								<p class="text-secondary text-sm">{settings_billing_license_keyTypeLabel()}</p>
								<div class="mt-1">
									<ToggleGroup
										options={keyTypeOptions}
										selected={keyType}
										onchange={handleTypeChange}
										disabled={switching || createKeyMutation.isPending}
									/>
								</div>
							</div>

							{#if !airGappedIncluded}
								<p class="text-secondary text-sm">
									{settings_billing_license_offlineKeyUpsell()}
									<button type="button" onclick={openPlanPicker} class="text-link hover:underline">
										{settings_billing_changePlan()}
									</button>
								</p>
							{/if}

							<div>
								<label class="text-secondary text-sm" for="license-key">
									{settings_billing_license_keyLabel()}
								</label>
								<div class="mt-1 flex flex-wrap items-center gap-2">
									<input
										id="license-key"
										class="input-field min-w-[16rem] flex-1 font-mono text-sm"
										value={maskedKey}
										readonly
									/>
									<button
										type="button"
										class="btn-primary flex items-center gap-2"
										onclick={() => currentKey && copyKey(currentKey, keyType)}
										disabled={!currentKey}
									>
										<Copy class="h-4 w-4" />
										{common_copy()}
									</button>
									<button
										type="button"
										class="btn-secondary flex items-center gap-2"
										onclick={() => (showRegenerateConfirm = true)}
										disabled={!currentKey || regenerateMutation.isPending}
									>
										<RefreshCw class="h-4 w-4" />
										{settings_billing_license_regenerate()}
									</button>
								</div>
							</div>
						</div>
					</div>
				</InfoCard>

				{#if missingCard}
					<div class="card card-static space-y-3 p-6">
						<h3 class="text-primary text-sm font-semibold">{billing_addPaymentMethod()}</h3>
						<p class="text-secondary text-sm">{cardDescription}</p>
						<button type="button" class="btn-primary" onclick={handleAddPaymentMethod}>
							{billing_addPaymentMethod()}
						</button>
					</div>
				{/if}
			</div>
		{/if}
	</div>

	{#if dismissible}
		<div class="modal-footer">
			<div class="flex justify-end">
				<button type="button" onclick={onClose} class="btn-secondary">{common_close()}</button>
			</div>
		</div>
	{/if}
</div>

<ConfirmationDialog
	isOpen={showRegenerateConfirm}
	title={settings_billing_license_regenerateTitle()}
	message={settings_billing_license_regenerateConfirm()}
	confirmLabel={settings_billing_license_regenerate()}
	variant="danger"
	onConfirm={handleRegenerate}
	onCancel={() => (showRegenerateConfirm = false)}
	onClose={() => (showRegenerateConfirm = false)}
/>

<ConfirmationDialog
	isOpen={pendingType != null}
	title={settings_billing_license_switchTitle()}
	message={switchConfirmMessage}
	confirmLabel={common_continue()}
	variant="danger"
	onConfirm={handleConfirmSwitch}
	onCancel={() => (pendingType = null)}
	onClose={() => (pendingType = null)}
/>
