<script lang="ts">
	import { onMount } from 'svelte';
	import { Copy, RefreshCw } from 'lucide-svelte';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import ConfirmationDialog from '$lib/shared/components/feedback/ConfirmationDialog.svelte';
	import StripeCardForm from '$lib/features/billing/StripeCardForm.svelte';
	import {
		useCreateLicenseKeyMutation,
		useCreateSetupIntentMutation,
		useFinalizePaymentMethodMutation,
		useRegenerateLicenseKeyMutation
	} from '$lib/features/billing/queries';
	import { triggerUpgrade } from '$lib/features/billing/trigger-upgrade';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import { isMissingPaymentMethod } from '$lib/shared/utils/trial';
	import { pushSuccess, pushWarning } from '$lib/shared/stores/feedback';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { copyViaSelection } from '$lib/shared/utils/clipboard';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import { waitForOrgUpdate } from '$lib/shared/billing/wait-for-org-update';
	import type { components } from '$lib/api/schema';
	import {
		billing_addPaymentMethod,
		billing_paymentMethodAdded,
		common_close,
		common_copied,
		common_copy,
		common_failedToCopy,
		common_license,
		common_never,
		common_save,
		common_tier,
		settings_billing_changePlan,
		settings_billing_license_addPaymentMethodSubtitle,
		settings_billing_license_copyOfflineKey,
		settings_billing_license_keyLabel,
		settings_billing_license_lastCheckIn,
		settings_billing_license_offlineKeyUpsell,
		settings_billing_license_paidThrough,
		settings_billing_license_regenerate,
		settings_billing_license_regenerateConfirm,
		settings_billing_license_regenerateTitle,
		settings_billing_license_regenerated,
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

	const currentUserQuery = useCurrentUserQuery();
	let userEmail = $derived(currentUserQuery.data?.email);

	const createKeyMutation = useCreateLicenseKeyMutation();
	const regenerateMutation = useRegenerateLicenseKeyMutation();
	const setupIntentMutation = useCreateSetupIntentMutation();
	const finalizeMutation = useFinalizePaymentMethodMutation();

	let planType = $derived(org?.plan?.type ?? null);
	let offlineKeysIncluded = $derived(
		billingPlans.getMetadata(planType).features?.air_gapped_deployment === true
	);

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

	// The online key is deterministic: the same claims re-signed every time, so
	// fetching it when the tab opens costs one request and shows the same string
	// the servers already hold.
	let onlineKey = $state<string | null>(null);
	let maskedKey = $derived(onlineKey ? `${onlineKey.slice(0, 8)}${'•'.repeat(32)}` : '');

	async function loadOnlineKey() {
		try {
			onlineKey = await createKeyMutation.mutateAsync('Online');
		} catch {
			// The API client toasts the failure.
		}
	}

	onMount(loadOnlineKey);

	let showRegenerateConfirm = $state(false);

	async function copyKey(key: string, keyType: LicenseKeyType) {
		try {
			if (window.isSecureContext && navigator.clipboard) {
				await navigator.clipboard.writeText(key);
			} else if (!copyViaSelection(key)) {
				// Plain-HTTP self-hosts have no navigator.clipboard; the selection copy is
				// the fallback, and it can still be refused.
				throw new Error('the browser refused the copy');
			}
			pushSuccess(common_copied());
			trackEvent('license_key_copied', { key_type: keyType });
		} catch (error) {
			pushWarning(common_failedToCopy({ error: String(error) }));
		}
	}

	async function copyOfflineKey() {
		try {
			const key = await createKeyMutation.mutateAsync('Offline');
			await copyKey(key, 'Offline');
		} catch {
			// The API client toasts a failed mint.
		}
	}

	async function handleRegenerate() {
		showRegenerateConfirm = false;
		try {
			await regenerateMutation.mutateAsync();
			onlineKey = null;
			await loadOnlineKey();
			pushSuccess(settings_billing_license_regenerated());
			trackEvent('license_key_regenerated');
		} catch {
			// The API client toasts the error.
		}
	}

	function openPlanPicker() {
		triggerUpgrade({
			source: 'settings_license',
			surface: 'billing_tab',
			reopenSettings: true,
			beforeModal: () => onClose()
		});
	}

	// Card collection runs inline here rather than through the payment-method modal,
	// which would open over the Settings modal this tab lives in.
	let missingCard = $derived(isMissingPaymentMethod(org));
	let clientSecret = $state<string | null>(null);
	let cardSetupStarted = false;

	$effect(() => {
		if (missingCard && !cardSetupStarted) {
			cardSetupStarted = true;
			void startCardSetup();
		}
	});

	async function startCardSetup() {
		try {
			clientSecret = await setupIntentMutation.mutateAsync();
		} catch {
			// The setup-intent error is toasted by the mutation.
		}
	}

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

	async function handleCardSuccess(setupIntentId: string) {
		await finalizeMutation.mutateAsync(setupIntentId);
		clientSecret = null;
		await waitForOrgUpdate((o) => o.has_payment_method ?? false);
		pushSuccess(billing_paymentMethodAdded());
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
								<label class="text-secondary text-sm" for="license-key">
									{settings_billing_license_keyLabel()}
								</label>
								<div class="mt-1 flex flex-wrap items-center gap-2">
									{#if onlineKey}
										<input
											id="license-key"
											class="input-field min-w-[16rem] flex-1 font-mono text-sm"
											value={maskedKey}
											readonly
										/>
									{:else}
										<div
											class="input-field flex min-w-[16rem] flex-1 items-center justify-center py-1"
										>
											<Loading />
										</div>
									{/if}
									<button
										type="button"
										class="btn-primary flex items-center gap-2"
										onclick={() => onlineKey && copyKey(onlineKey, 'Online')}
										disabled={!onlineKey}
									>
										<Copy class="h-4 w-4" />
										{common_copy()}
									</button>
									<button
										type="button"
										class="btn-secondary flex items-center gap-2"
										onclick={() => (showRegenerateConfirm = true)}
										disabled={!onlineKey || regenerateMutation.isPending}
									>
										<RefreshCw class="h-4 w-4" />
										{settings_billing_license_regenerate()}
									</button>
								</div>
							</div>

							{#if offlineKeysIncluded}
								<button
									type="button"
									class="btn-secondary flex items-center gap-2"
									onclick={copyOfflineKey}
									disabled={createKeyMutation.isPending}
								>
									<Copy class="h-4 w-4" />
									{settings_billing_license_copyOfflineKey()}
								</button>
							{:else}
								<p class="text-secondary text-sm">
									{settings_billing_license_offlineKeyUpsell()}
									<button type="button" onclick={openPlanPicker} class="text-link hover:underline">
										{settings_billing_changePlan()}
									</button>
								</p>
							{/if}
						</div>
					</div>
				</InfoCard>

				{#if missingCard}
					<div class="card card-static p-0">
						<h3 class="text-primary px-6 pt-6 text-sm font-semibold">
							{billing_addPaymentMethod()}
						</h3>
						{#if clientSecret}
							<StripeCardForm
								{clientSecret}
								email={userEmail}
								description={cardDescription}
								submitLabel={common_save()}
								onSuccess={handleCardSuccess}
							/>
						{:else}
							<div class="flex min-h-[12rem] items-center justify-center p-6">
								<Loading />
							</div>
						{/if}
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
