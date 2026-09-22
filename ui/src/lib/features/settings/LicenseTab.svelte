<script lang="ts">
	import { Copy, RefreshCw } from 'lucide-svelte';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import ConfirmationDialog from '$lib/shared/components/feedback/ConfirmationDialog.svelte';
	import SegmentedControl from '$lib/shared/components/forms/SegmentedControl.svelte';
	import {
		useCreateLicenseKeyMutation,
		useCurrentLicenseKeyQuery,
		useEndTrialMutation,
		useInvoiceBillingStatusQuery,
		useRotateLicenseKeyMutation
	} from '$lib/features/billing/queries';
	import { priceToCharge } from '$lib/features/billing/pricing';
	import { triggerUpgrade } from '$lib/features/billing/trigger-upgrade';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { hasLicensedPlan } from '$lib/features/organizations/types';
	import type { Organization } from '$lib/features/organizations/types';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import {
		licenseKeyExpiry,
		licenseKeySwitchBackWindowDays,
		useConfigQuery
	} from '$lib/shared/stores/config-query';
	import { canPay, getTrialDaysLeft, isMissingPaymentMethod } from '$lib/shared/utils/trial';
	import { pushError, pushSuccess, pushWarning } from '$lib/shared/stores/feedback';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { copyText } from '$lib/shared/utils/clipboard';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import { startSetupPayment } from '$lib/shared/billing/setup-payment';
	import { waitForOrgUpdate } from '$lib/shared/billing/wait-for-org-update';
	import type { components } from '$lib/api/schema';
	import {
		apiKeys_rotateKey,
		billing_addPaymentMethod,
		billing_requestAccepted,
		common_airGapped,
		common_close,
		common_continue,
		common_copied,
		common_copy,
		common_expires,
		common_failedToCopy,
		common_license,
		common_never,
		common_online,
		common_tier,
		settings_billing_changePlan,
		settings_billing_license_addPaymentMethodSubtitle,
		settings_billing_license_airGappedCurrentUntil,
		settings_billing_license_airGappedNeedsCard,
		settings_billing_license_airGappedOpenInvoice,
		settings_billing_license_airGappedOpenInvoiceLink,
		settings_billing_license_airGappedPastDue,
		settings_billing_license_keyLabel,
		settings_billing_license_keyTypeChanged,
		settings_billing_license_keyTypeLabel,
		settings_billing_license_lastCheckIn,
		settings_billing_license_offlineKeyUpsell,
		settings_billing_license_onlineLockedUntil,
		settings_billing_license_paidThrough,
		settings_billing_license_paymentDeclined,
		settings_billing_license_rotateConfirm,
		settings_billing_license_rotateTitle,
		settings_billing_license_rotated,
		settings_billing_license_switchAirGappedConfirm,
		settings_billing_license_switchConfirm,
		settings_billing_license_switchOnlineConfirm,
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
	const configQuery = useConfigQuery();
	let billingEnabled = $derived(configQuery.data?.billing_enabled ?? false);

	const createKeyMutation = useCreateLicenseKeyMutation();
	const rotateMutation = useRotateLicenseKeyMutation();
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
	// Either way to pay: an air-gapped key needs a paid subscription, and a
	// buyer paying against a purchase order gets there without a card.
	let hasCard = $derived(canPay(org));
	let isTrialing = $derived(org?.plan_status === 'trialing');
	let isPastDue = $derived(org?.plan_status === 'past_due');
	// The server also refuses an air-gapped mint until the subscription is out of
	// trial, but choosing the option during a trial is what ends the trial. A card
	// on file is the one thing the user has to do first.
	//
	// A declined card stays attached, so `hasCard` on its own keeps the option
	// live for an org whose payment just failed. The server would then refuse the
	// mint after the switch had already retired their online key.
	//
	// An open invoice is the same story ahead of time: the server caps an
	// air-gapped key at what has been paid for, so while one is outstanding
	// there is nothing left to mint.
	const invoiceBillingQuery = useInvoiceBillingStatusQuery(
		() => billingEnabled && org != null && hasLicensedPlan(org)
	);
	let hasOpenInvoice = $derived(invoiceBillingQuery.data?.open_invoice != null);
	let openInvoiceUrl = $derived(invoiceBillingQuery.data?.open_invoice?.hosted_invoice_url ?? null);
	let airGappedAvailable = $derived(airGappedIncluded && hasCard && !isPastDue && !hasOpenInvoice);
	// The date an air-gapped key stays current until, which is also the date both
	// locks lift: the plan change and the switch back to an online key. Read from
	// the org rather than recomputed, so this line cannot disagree with the
	// server's refusal.
	let airGappedCurrentUntil = $derived(org?.air_gapped_key_current_until ?? null);
	let missingCard = $derived(isMissingPaymentMethod(org, billingEnabled));
	let chargeAmount = $derived(priceToCharge(org));

	function formatDate(value: string | Date): string {
		return new Date(value).toLocaleDateString(undefined, {
			month: 'long',
			day: 'numeric',
			year: 'numeric'
		});
	}

	// An air-gapped key carries its expiry, so an org running one stays on it until
	// the period it has paid for is over. The server enforces the refusal; the
	// disabled option says so before the click.
	let switchBackDate = $derived(
		keyType === 'Offline' &&
			org?.license_paid_through != null &&
			Date.now() < Date.parse(org.license_paid_through)
			? formatDate(org.license_paid_through)
			: null
	);

	let keyTypeOptions = $derived([
		{
			value: 'Online',
			label: common_online(),
			disabled: switchBackDate != null,
			tooltip: switchBackDate
				? settings_billing_license_onlineLockedUntil({ date: switchBackDate })
				: undefined
		},
		{ value: 'Offline', label: common_airGapped(), disabled: !airGappedAvailable }
	]);

	let paidThrough = $derived(
		org?.license_paid_through ? formatDate(org.license_paid_through) : null
	);
	// An air-gapped key outlives the paid-through date by the server's buffer, so
	// the date on the key is not the date in the billing row.
	let keyExpiresOn = $derived(
		keyType === 'Offline' && org?.license_paid_through && configQuery.data
			? formatDate(licenseKeyExpiry(configQuery.data, org.license_paid_through))
			: null
	);
	let switchBackWindowDays = $derived(
		configQuery.data ? licenseKeySwitchBackWindowDays(configQuery.data) : null
	);
	let lastCheckIn = $derived(
		org?.license_checkin_at ? formatTimestamp(org.license_checkin_at) : common_never()
	);

	let trialEndsOn = $derived(org?.trial_end_date ? formatDate(org.trial_end_date) : null);
	let cardDescription = $derived(
		trialEndsOn
			? settings_billing_license_trialPaymentBody({ date: trialEndsOn })
			: settings_billing_license_addPaymentMethodSubtitle()
	);

	let showRotateConfirm = $state(false);
	let pendingType = $state<LicenseKeyType | null>(null);
	let switching = $state(false);

	let switchConfirmMessage = $derived.by(() => {
		if (pendingType === 'Online') return settings_billing_license_switchOnlineConfirm();
		const days = switchBackWindowDays;
		// Every air-gapped message names the switch-back window, which only the
		// server knows. Until the config lands there is nothing accurate to show.
		if (days == null) return '';
		if (isTrialing && chargeAmount != null) {
			return settings_billing_license_switchConfirm({ amount: chargeAmount, days });
		}
		return settings_billing_license_switchAirGappedConfirm({ days });
	});

	async function copyKey(key: string, type: LicenseKeyType) {
		try {
			await copyText(key);
			pushSuccess(common_copied());
			trackEvent('license_key_copied', { key_type: type });
		} catch (error) {
			pushWarning(common_failedToCopy({ error: String(error) }));
		}
	}

	async function handleRotate() {
		showRotateConfirm = false;
		try {
			await rotateMutation.mutateAsync();
			pushSuccess(settings_billing_license_rotated());
			trackEvent('license_key_rotated');
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
				// Both outcomes end the wait. A decline advances neither paid-through
				// nor the status to active, so waiting only on success burns every
				// attempt and then reports the opposite of what happened.
				//
				// The flag rides on an object because a plain boolean would be
				// narrowed to its initial value: the assignment happens inside the
				// predicate, where TypeScript cannot see it.
				const outcome = { declined: false };
				const settled = await waitForOrgUpdate(
					(o) => {
						if (o.plan_status === 'past_due') {
							outcome.declined = true;
							return true;
						}
						return o.plan_status === 'active' && paidPastTrial(o);
					},
					{ intervalMs: 500 }
				);
				// Neither outcome landed inside the window. The charge may still go
				// through, so this cannot claim it either way.
				if (!settled) {
					pushWarning(billing_requestAccepted());
					return;
				}
				// Leave the key type alone. The switch retires the key they are
				// running, and nothing has been paid for the one they asked for.
				if (outcome.declined) {
					pushError(settings_billing_license_paymentDeclined());
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
							{#if keyExpiresOn}
								<dt class="text-secondary">{common_expires()}</dt>
								<dd class="text-primary">{keyExpiresOn}</dd>
							{/if}
							<dt class="text-secondary">{settings_billing_license_lastCheckIn()}</dt>
							<dd class="text-primary">{lastCheckIn}</dd>
						</dl>

						<div class="space-y-3 border-t pt-3" style="border-color: var(--color-border)">
							<div>
								<p class="text-secondary text-sm">{settings_billing_license_keyTypeLabel()}</p>
								<div class="mt-1">
									<SegmentedControl
										options={keyTypeOptions}
										selected={keyType}
										onchange={handleTypeChange}
										size="md"
										disabled={switching || createKeyMutation.isPending}
									/>
								</div>
							</div>

							<!-- One line, one slot: the plan upsell, or (on a plan that already
							     includes air-gapped) what makes the option selectable. A tooltip on
							     the disabled option can't carry this — the message has to be readable
							     without hovering something that isn't clickable. -->
							{#if airGappedCurrentUntil}
								<!-- The org already holds the key, so this slot says what is locked
								     rather than why air-gapped is unavailable. It has to come first:
								     such an org has a card and is not past due, so it falls through
								     every branch below to no line at all. -->
								<p class="text-secondary text-sm">
									{settings_billing_license_airGappedCurrentUntil({
										date: formatDate(airGappedCurrentUntil)
									})}
								</p>
							{:else if !airGappedIncluded}
								<p class="text-secondary text-sm">
									{settings_billing_license_offlineKeyUpsell()}
									<button type="button" onclick={openPlanPicker} class="text-link hover:underline">
										{settings_billing_changePlan()}
									</button>
								</p>
							{:else if !hasCard}
								<p class="text-secondary text-sm">
									{settings_billing_license_airGappedNeedsCard()}
								</p>
							{:else if isPastDue}
								<p class="text-secondary text-sm">
									{settings_billing_license_airGappedPastDue()}
								</p>
							{:else if hasOpenInvoice}
								<p class="text-secondary text-sm">
									{settings_billing_license_airGappedOpenInvoice()}
									{#if openInvoiceUrl}
										<!-- eslint-disable svelte/no-navigation-without-resolve -->
										<a
											href={openInvoiceUrl}
											target="_blank"
											rel="external noopener noreferrer"
											class="text-link hover:underline"
										>
											{settings_billing_license_airGappedOpenInvoiceLink()}
										</a>
										<!-- eslint-enable svelte/no-navigation-without-resolve -->
									{/if}
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
										onclick={() => (showRotateConfirm = true)}
										disabled={!currentKey || rotateMutation.isPending}
									>
										<RefreshCw class="h-4 w-4" />
										{apiKeys_rotateKey()}
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
	isOpen={showRotateConfirm}
	title={settings_billing_license_rotateTitle()}
	message={settings_billing_license_rotateConfirm()}
	confirmLabel={apiKeys_rotateKey()}
	variant="danger"
	onConfirm={handleRotate}
	onCancel={() => (showRotateConfirm = false)}
	onClose={() => (showRotateConfirm = false)}
/>

<ConfirmationDialog
	isOpen={pendingType != null && switchConfirmMessage !== ''}
	title={settings_billing_license_switchTitle()}
	message={switchConfirmMessage}
	confirmLabel={common_continue()}
	variant="info"
	onConfirm={handleConfirmSwitch}
	onCancel={() => (pendingType = null)}
	onClose={() => (pendingType = null)}
/>
