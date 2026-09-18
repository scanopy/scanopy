<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import TextArea from '$lib/shared/components/forms/input/TextArea.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import {
		usePauseSubscriptionMutation,
		useApplyDiscountSaveOfferMutation,
		useCancelSubscriptionMutation,
		useSaveOfferCouponQuery
	} from '$lib/features/billing/queries';
	import cancelReasons from '$lib/data/cancel-reasons.json';
	import saveOffers from '$lib/data/save-offers.json';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import { useCurrentLicenseKeyQuery } from '$lib/features/billing/queries';
	import { metaName } from '$lib/i18n/metadata';
	import { pushSuccess, pushWarning } from '$lib/shared/stores/feedback';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import { waitForOrgUpdate } from '$lib/shared/billing/wait-for-org-update';
	import type { components } from '$lib/api/schema';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import {
		common_back,
		settings_billing_cancelSubscription,
		settings_billing_cancelModal_reasonHeading,
		settings_billing_cancelModal_reasonHelp,
		settings_billing_cancelModal_commentLabel,
		settings_billing_cancelModal_commentPlaceholder,
		settings_billing_cancelModal_continueCancel,
		settings_billing_cancelModal_keepSubscription,
		settings_billing_cancelModal_airGappedNoRefund,
		settings_billing_cancelModal_confirmDisclosure,
		settings_billing_cancelModal_confirmHeading,
		settings_billing_cancelModal_confirmCta,
		settings_billing_cancelModal_doneSummary,
		settings_billing_saveOffer_pauseTitle,
		settings_billing_saveOffer_pauseSubtitle,
		settings_billing_saveOffer_pauseDuration30,
		settings_billing_saveOffer_pauseDuration60,
		settings_billing_saveOffer_pauseDuration90,
		settings_billing_saveOffer_pausePreview,
		settings_billing_saveOffer_pauseCta,
		settings_billing_saveOffer_pauseCooldown,
		settings_billing_saveOffer_pauseSpansRenewal_title,
		settings_billing_saveOffer_pauseSpansRenewal_body,
		settings_billing_saveOffer_discountTitle,
		settings_billing_saveOffer_discountSubtitleMonthly,
		settings_billing_saveOffer_discountSubtitleYearly,
		settings_billing_saveOffer_discountCta,
		billing_discountApplied,
		billing_requestAccepted,
		billing_subscriptionPausedUntil
	} from '$lib/paraglide/messages';

	type CancelReason = components['schemas']['CancelReason'];
	type PauseDuration = components['schemas']['PauseDuration'];

	let {
		isOpen = false,
		onClose,
		lastPausedAt = null,
		lastDiscountAt = null,
		planStatus = null,
		planType = null,
		planRate = null,
		nextRenewalAt = null,
		onSubscriptionChanged
	}: {
		isOpen?: boolean;
		onClose: () => void;
		/** Org's `last_paused_at` — used for 6-month rolling pause cooldown messaging. */
		lastPausedAt?: string | null;
		/** Org's `last_discount_at` — once-ever flag; non-null hides the Discount panel. */
		lastDiscountAt?: string | null;
		/** Org's `plan_status` — pause/discount save offers are suppressed while trialing. */
		planStatus?: string | null;
		/** Org's `plan.type` — save offers only apply to Stripe-managed plans. */
		planType?: string | null;
		/** Org's `plan.rate` — Month or Year. Used for yearly span-renewal info copy. */
		planRate?: 'Month' | 'Year' | null;
		/** Org's `next_renewal_at` — yearly span-renewal info copy uses this. */
		nextRenewalAt?: string | null;
		/** Called after pause/discount/cancel succeed so the caller can refresh the org payload. */
		onSubscriptionChanged?: () => void;
	} = $props();

	// A trial isn't charging yet, so pause (which freezes an active billing
	// cycle) is meaningless and is suppressed below. The discount, however,
	// applies to the first invoice at trial-end, so it survives the trial — a
	// trialing user picking "Too expensive" should still see the offer.
	let isTrialing = $derived(planStatus === 'trialing');

	// Save offers (pause + discount) only apply to Stripe-managed plans —
	// pausing or discounting a non-Stripe sub is nonsensical and the backend
	// would 4xx anyway. This just hides the dead-end UI.
	let canReceiveSaveOffer = $derived(
		billingPlans.getMetadata(planType ?? null).is_stripe_managed === true
	);

	// An air-gapped key validates without ever reaching us, so it runs to its
	// own expiry after a cancellation and there is nothing to refund. Say so
	// before the buyer commits, not after. The key type comes from the same
	// query the License tab uses, so this costs no extra round trip there.
	let airGappedPlan = $derived(
		billingPlans.getMetadata(planType ?? null).features?.air_gapped_deployment === true
	);
	const licenseKeyQuery = useCurrentLicenseKeyQuery(() => airGappedPlan);
	let holdsAirGappedKey = $derived(licenseKeyQuery.data?.key_type === 'Offline');

	// Two internal steps. Step 1 picks the reason; step 2 shows any save offers
	// AND hosts the Confirm Cancellation action in the footer. No stepper UI:
	// the existence of a save offer should not be telegraphed by a visible
	// breadcrumb labelled "Save Offer".
	type Step = 1 | 2;
	let currentStep = $state<Step>(1);

	// Mirror state for the reason value. TanStack's `form.state.values` is not
	// tracked by Svelte 5 `$derived`, so we mirror the form's reason_code into
	// plain `$state` via a store subscription and read from this for any
	// reactive UI (button enable/disable, save-offer lookup, etc).
	let selectedReason = $state<CancelReason | ''>('');
	let selectedPauseDuration = $state<PauseDuration>('days30');

	const cancelMutation = useCancelSubscriptionMutation();
	const pauseMutation = usePauseSubscriptionMutation();
	const discountMutation = useApplyDiscountSaveOfferMutation();
	const configQuery = useConfigQuery();
	const discountAvailable = $derived(configQuery.data?.discount_save_offer_available ?? false);
	// Fetch live coupon terms only while the modal is open and the deployment
	// has a coupon configured — otherwise the GET would return null anyway and
	// the panel won't render.
	const saveOfferCouponQuery = useSaveOfferCouponQuery(() => isOpen && discountAvailable);
	const saveOfferCoupon = $derived(saveOfferCouponQuery.data ?? null);

	const form = createForm(() => ({
		defaultValues: {
			reason_code: '' as CancelReason | '',
			comment: ''
		},
		onSubmit: () => {
			// Step transitions are imperative — submit handler is unused.
		}
	}));

	$effect(() => {
		// Mirror the form's reason_code into $state so $derived expressions
		// (offersForReason, button gating) react to changes.
		// `form.state.values` is NOT tracked by $derived — read it from inside
		// a `form.store.subscribe` callback (CLAUDE.md TanStack reactivity gap).
		return form.store.subscribe(() => {
			const v = form.state.values.reason_code as CancelReason | '';
			if (v !== selectedReason) {
				selectedReason = v;
			}
		});
	});

	const reasonOptions = $derived([
		{ value: '', label: '—', disabled: true },
		...cancelReasons.map((r) => ({
			value: r.id,
			label: metaName('cancel_reasons', r.id, r.name ?? r.id)
		}))
	]);

	const offersForReason = $derived.by<string[]>(() => {
		if (!canReceiveSaveOffer || !selectedReason) return [];
		const reason = cancelReasons.find((r) => r.id === selectedReason);
		const offers = (reason?.metadata as { save_offers?: string[] } | null | undefined)?.save_offers;
		return (offers ?? []).filter((o) => {
			if (o === 'discount') {
				// Hide the discount panel until the backend confirms the coupon
				// is applicable to this org's next renewal. saveOfferCoupon ===
				// null can mean (a) the env var isn't configured, (b) the next
				// renewal falls outside the coupon's duration window, or (c) the
				// query is still loading. In any of those cases the panel would
				// have nothing useful to show — better to not render it.
				return !lastDiscountAt && saveOfferCoupon != null;
			}
			// Non-discount offers (pause) freeze an active billing cycle; a
			// trial isn't charging yet, so suppress them while trialing.
			return !isTrialing;
		});
	});

	const offerMeta = (offerId: string) => saveOffers.find((o) => o.id === offerId);

	const pauseCooldownEnd = $derived.by<Date | null>(() => {
		if (!lastPausedAt) return null;
		const last = new Date(lastPausedAt);
		const eligible = new Date(
			last.getFullYear(),
			last.getMonth() + 6,
			last.getDate(),
			last.getHours(),
			last.getMinutes(),
			last.getSeconds()
		);
		return eligible.getTime() > Date.now() ? eligible : null;
	});

	const pauseResumesAt = $derived.by<Date>(() => {
		const days =
			selectedPauseDuration === 'days30' ? 30 : selectedPauseDuration === 'days60' ? 60 : 90;
		return new Date(Date.now() + days * 24 * 60 * 60 * 1000);
	});

	const pauseDurationOptions = $derived([
		{
			value: 'days30' as PauseDuration,
			label: settings_billing_saveOffer_pauseDuration30(),
			days: 30
		},
		{
			value: 'days60' as PauseDuration,
			label: settings_billing_saveOffer_pauseDuration60(),
			days: 60
		},
		{
			value: 'days90' as PauseDuration,
			label: settings_billing_saveOffer_pauseDuration90(),
			days: 90
		}
	]);

	// When a yearly pause extends past the renewal date, Stripe generates the
	// next year's invoice as a draft mid-pause and finalizes it at resume.
	// The pause credit partially offsets it, but the customer still gets a
	// net charge for "next year minus pause credit." Surface this in an
	// InlineInfo so they aren't surprised. Monthly subs never hit this —
	// their drafts and credits are in the same denomination (full periods).
	const renewalSpanWarning = $derived.by(() => {
		if (planRate !== 'Year' || !nextRenewalAt) return null;
		const renewal = new Date(nextRenewalAt);
		const pauseDays = pauseDurationOptions.find((o) => o.value === selectedPauseDuration)?.days;
		if (!pauseDays) return null;
		const resumeAt = new Date(Date.now() + pauseDays * 24 * 60 * 60 * 1000);
		if (resumeAt.getTime() <= renewal.getTime()) return null;
		return { renewalDate: renewal };
	});

	function fmtDate(d: Date | string): string {
		const dt = typeof d === 'string' ? new Date(d) : d;
		return dt.toLocaleDateString(undefined, {
			month: 'long',
			day: 'numeric',
			year: 'numeric'
		});
	}

	function reset() {
		currentStep = 1;
		selectedReason = '';
		selectedPauseDuration = 'days30';
		form.reset();
	}

	function handleClose() {
		onClose();
		// Defer reset until after close animation so step 1 doesn't flicker.
		setTimeout(reset, 200);
	}

	function goToStep2() {
		currentStep = 2;
	}

	async function handlePauseRedeem() {
		try {
			await pauseMutation.mutateAsync(selectedPauseDuration);
			const flipped = await waitForOrgUpdate((o) => o.plan_status === 'paused');
			if (flipped) {
				pushSuccess(billing_subscriptionPausedUntil({ date: fmtDate(pauseResumesAt) }));
			} else {
				pushWarning(billing_requestAccepted());
			}
			onSubscriptionChanged?.();
			handleClose();
		} catch {
			// Mutation onError surfaces toast.
		}
	}

	async function handleDiscountRedeem() {
		try {
			await discountMutation.mutateAsync();
			const flipped = await waitForOrgUpdate((o) => o.last_discount_at != null);
			if (flipped) {
				pushSuccess(billing_discountApplied());
			} else {
				pushWarning(billing_requestAccepted());
			}
			onSubscriptionChanged?.();
			handleClose();
		} catch {
			// Mutation onError surfaces toast.
		}
	}

	async function handleConfirmCancel() {
		if (!selectedReason) return;
		const shownOffers = offersForReason as Array<components['schemas']['SaveOffer']>;
		try {
			const response = await cancelMutation.mutateAsync({
				reason_code: selectedReason,
				comment: form.state.values.comment || null,
				save_offer_shown: shownOffers,
				save_offer_redeemed: null
			});
			const flipped = await waitForOrgUpdate((o) => o.plan_status === 'pending_cancellation');
			if (flipped) {
				pushSuccess(
					settings_billing_cancelModal_doneSummary({
						periodEnd: fmtDate(response.period_end)
					})
				);
			} else {
				pushWarning(
					'Cancellation request accepted. It may take a moment to reflect across your account.'
				);
			}
			onSubscriptionChanged?.();
			handleClose();
		} catch {
			// Mutation onError surfaces toast; modal stays open.
		}
	}
</script>

<GenericModal
	{isOpen}
	title={settings_billing_cancelSubscription()}
	size="md"
	onClose={handleClose}
>
	<div class="flex min-h-0 flex-1 flex-col gap-6 overflow-y-auto p-6">
		{#if currentStep === 1}
			<div class="space-y-4">
				<div>
					<h3 class="text-primary text-lg font-semibold">
						{settings_billing_cancelModal_reasonHeading()}
					</h3>
					<p class="text-secondary mt-1 text-sm">
						{settings_billing_cancelModal_reasonHelp()}
					</p>
				</div>
				<form.Field name="reason_code">
					{#snippet children(field: AnyFieldApi)}
						<SelectInput
							id="cancel-reason"
							label={settings_billing_cancelModal_reasonHeading()}
							{field}
							required={true}
							options={reasonOptions}
						/>
					{/snippet}
				</form.Field>
				<form.Field name="comment">
					{#snippet children(field: AnyFieldApi)}
						<TextArea
							id="cancel-comment"
							label={settings_billing_cancelModal_commentLabel()}
							placeholder={settings_billing_cancelModal_commentPlaceholder()}
							rows={3}
							{field}
						/>
					{/snippet}
				</form.Field>
			</div>
		{:else}
			<div class="space-y-4">
				{#each offersForReason as offerId (offerId)}
					{#if offerId === 'pause'}
						<div class="card card-static space-y-3 p-4">
							<div>
								<h4 class="text-primary text-base font-semibold">
									{metaName(
										'save_offers',
										'pause',
										offerMeta('pause')?.name ?? settings_billing_saveOffer_pauseTitle()
									)}
								</h4>
								<p class="text-secondary mt-1 text-sm">
									{settings_billing_saveOffer_pauseSubtitle()}
								</p>
							</div>
							{#if pauseCooldownEnd && lastPausedAt}
								<InlineWarning
									title={settings_billing_saveOffer_pauseCooldown({
										lastPausedDate: fmtDate(new Date(lastPausedAt)),
										nextEligibleDate: fmtDate(pauseCooldownEnd)
									})}
								/>
							{:else}
								<div class="grid grid-cols-3 gap-2">
									{#each pauseDurationOptions as d (d.value)}
										<button
											type="button"
											class="card-static rounded-md border p-2 text-sm {selectedPauseDuration ===
											d.value
												? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
												: ''}"
											onclick={() => (selectedPauseDuration = d.value)}
										>
											{d.label}
										</button>
									{/each}
								</div>
								<p class="text-tertiary text-sm">
									{settings_billing_saveOffer_pausePreview({
										resumesAt: fmtDate(pauseResumesAt)
									})}
								</p>
								{#if renewalSpanWarning}
									<InlineInfo
										title={settings_billing_saveOffer_pauseSpansRenewal_title({
											renewalDate: fmtDate(renewalSpanWarning.renewalDate)
										})}
										body={settings_billing_saveOffer_pauseSpansRenewal_body()}
									/>
								{/if}
								<button
									type="button"
									class="btn-primary w-full"
									disabled={pauseMutation.isPending}
									onclick={handlePauseRedeem}
								>
									{settings_billing_saveOffer_pauseCta()}
								</button>
							{/if}
						</div>
					{:else if offerId === 'discount'}
						<div class="card card-static space-y-3 p-4">
							<div>
								<h4 class="text-primary text-base font-semibold">
									{metaName(
										'save_offers',
										'discount',
										offerMeta('discount')?.name ?? settings_billing_saveOffer_discountTitle()
									)}
								</h4>
								<p class="text-secondary mt-1 text-sm">
									{#if saveOfferCoupon?.billing_rate === 'Year'}
										{settings_billing_saveOffer_discountSubtitleYearly({
											percentOff: saveOfferCoupon.percent_off,
											nextRenewalDate: fmtDate(new Date(saveOfferCoupon.next_renewal_at))
										})}
									{:else if saveOfferCoupon}
										{settings_billing_saveOffer_discountSubtitleMonthly({
											percentOff: saveOfferCoupon.percent_off,
											durationInMonths: saveOfferCoupon.duration_in_months
										})}
									{/if}
								</p>
							</div>
							<button
								type="button"
								class="btn-primary w-full"
								disabled={discountMutation.isPending}
								onclick={handleDiscountRedeem}
							>
								{settings_billing_saveOffer_discountCta()}
							</button>
						</div>
					{/if}
				{/each}
				<div class="card card-static space-y-3 p-4">
					<div>
						<h4 class="text-primary text-base font-semibold">
							{settings_billing_cancelModal_confirmHeading()}
						</h4>
						<p class="text-secondary mt-1 text-sm">
							{settings_billing_cancelModal_confirmDisclosure({
								periodEnd: 'the end of your current billing cycle'
							})}
						</p>
						{#if holdsAirGappedKey}
							<div class="mt-3">
								<InlineWarning title={settings_billing_cancelModal_airGappedNoRefund()} />
							</div>
						{/if}
					</div>
					<button
						type="button"
						class="btn-danger w-full"
						disabled={cancelMutation.isPending}
						onclick={handleConfirmCancel}
					>
						{settings_billing_cancelModal_confirmCta()}
					</button>
				</div>
			</div>
		{/if}
	</div>

	{#snippet footer()}
		<div class="modal-footer flex justify-between gap-2">
			{#if currentStep === 1}
				<button type="button" class="btn-secondary" onclick={handleClose}>
					{settings_billing_cancelModal_keepSubscription()}
				</button>
				<button type="button" class="btn-primary" disabled={!selectedReason} onclick={goToStep2}>
					{settings_billing_cancelModal_continueCancel()}
				</button>
			{:else}
				<button type="button" class="btn-secondary" onclick={() => (currentStep = 1)}>
					{common_back()}
				</button>
			{/if}
		</div>
	{/snippet}
</GenericModal>
