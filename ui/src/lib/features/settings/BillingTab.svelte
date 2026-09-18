<script lang="ts">
	import { CreditCard } from 'lucide-svelte';
	import ProgressTrack from '$lib/shared/components/data/ProgressTrack.svelte';
	import { triggerUpgrade } from '$lib/features/billing/trigger-upgrade';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { hasLicensedPlan } from '$lib/features/organizations/types';
	import { billingPlans, planStatuses } from '$lib/shared/stores/metadata';
	import { canPay, isMissingPaymentMethod } from '$lib/shared/utils/trial';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import { trackEvent, trackOncePerSession } from '$lib/shared/utils/analytics';
	import {
		useCustomerPortalMutation,
		useResumeSubscriptionMutation,
		useReactivateSubscriptionMutation,
		useExtendTrialMutation,
		useInvoiceBillingStatusQuery
	} from '$lib/features/billing/queries';
	import CancelSubscriptionModal from '$lib/features/billing/CancelSubscriptionModal.svelte';
	import InvoiceBillingCard from '$lib/features/billing/InvoiceBillingCard.svelte';
	import { renewalLabel } from '$lib/features/billing/renewal';
	import { discountedPrice, saveOfferDiscount } from '$lib/features/billing/pricing';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import { useDashboardQuery } from '$lib/features/home/queries';
	import {
		common_atLimit,
		common_billingExtra,
		common_billingUsage,
		common_close,
		common_hosts,
		common_included,
		common_networks,
		common_seats,
		common_tryAgainLater,
		common_usage,
		settings_billing_billingQuestions,
		settings_billing_canceled,
		settings_billing_contactUs,
		settings_billing_currentPlan,
		settings_billing_discount_active,
		settings_billing_discount_active_yearly,
		settings_billing_downgrade_pending,
		settings_billing_paymentAndInvoices,
		settings_billing_needHelp,
		settings_billing_pastDue,
		settings_billing_cancelSubscription,
		settings_billing_per,
		settings_billing_unableToLoad,
		settings_billing_updatePaymentMethod,
		settings_billing_upgradePlan,
		settings_billing_usageAddOn,
		settings_billing_usageUpgradeToAddMore,
		settings_billing_changePlan,
		settings_billing_resume_button,
		settings_billing_resume_confirmBody,
		settings_billing_reactivateSubscription,
		settings_billing_extendTrial_link,
		settings_billing_extendTrial_confirmBody,
		settings_billing_addPaymentMethodSubtitle,
		settings_billing_license_addPaymentMethodSubtitle,
		settings_billing_trialCountdown,
		settings_billing_trialEndsOn,
		billing_addPaymentMethod,
		billing_noPaymentMethodBannerBody,
		billing_requestAccepted,
		billing_subscriptionReactivated,
		billing_trialExtended
	} from '$lib/paraglide/messages';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import InlineDanger from '$lib/shared/components/feedback/InlineDanger.svelte';
	import ButtonMenu, { type ButtonMenuItem } from '$lib/shared/components/ButtonMenu.svelte';
	import { pushSuccess, pushWarning } from '$lib/shared/stores/feedback';
	import { startSetupPayment } from '$lib/shared/billing/setup-payment';
	import { waitForOrgUpdate } from '$lib/shared/billing/wait-for-org-update';

	let {
		isOpen = false,
		onClose,
		dismissible = true
	}: {
		isOpen?: boolean;
		onClose: () => void;
		dismissible?: boolean;
	} = $props();

	// TanStack Query for organization
	const organizationQuery = useOrganizationQuery();
	let org = $derived(organizationQuery.data);
	// Licensed self-hosted plans get license keys here instead of usage: the
	// dashboard route is locked for them server-side.
	let isLicensedPlan = $derived(org != null && hasLicensedPlan(org));

	// Dashboard summary aggregates host/network/seat counts into one query —
	// reuse it here instead of re-counting users/networks/hosts independently.
	const dashboardQuery = useDashboardQuery({ enabled: () => org != null && !isLicensedPlan });
	let planUsage = $derived(dashboardQuery.data?.plan_usage);

	// PO number, an open quote, and the link to an unpaid invoice. Self-hosted
	// only: the endpoint refuses other plans, and a cloud org would pay for a
	// Stripe round trip on every visit to this tab.
	const invoiceBillingQuery = useInvoiceBillingStatusQuery(() => isLicensedPlan);
	let invoiceBilling = $derived(invoiceBillingQuery.data ?? null);

	// Customer portal mutation
	const customerPortalMutation = useCustomerPortalMutation();
	const resumeMutation = useResumeSubscriptionMutation();
	const reactivateMutation = useReactivateSubscriptionMutation();
	const extendTrialMutation = useExtendTrialMutation();

	// Cancel modal state. Replaces the legacy Stripe-Portal handoff.
	let showCancelModal = $state(false);

	let seatCount = $derived(planUsage?.seat_count ?? 0);
	let networkCount = $derived(planUsage?.network_count ?? 0);
	let hostCount = $derived(planUsage?.host_count ?? 0);

	// Status badge color + icon come from PlanStatus metadata (backend
	// EntityMetadataProvider). `StatusIcon` is capitalized so it renders as a
	// component in the markup.
	let StatusIcon = $derived(planStatuses.getIconComponent(org?.plan_status ?? null));
	let planStatusColor = $derived(planStatuses.getColorHelper(org?.plan_status ?? null).text);

	let isFree = $derived(billingPlans.getMetadata(org?.plan?.type ?? null).is_free === true);

	// Plan-status shorthands used across the banner + CTA section.
	let isTrialing = $derived(org?.plan_status === 'trialing');
	let isActive = $derived(org?.plan_status === 'active');
	let isPastDue = $derived(org?.plan_status === 'past_due');
	let isPaused = $derived(org?.plan_status === 'paused');
	let isPendingCancellation = $derived(org?.plan_status === 'pending_cancellation');

	// Live Stripe subscription whose lifecycle these CTAs can act on. Non-Stripe
	// plans (Free/Community/Demo/SelfHosted/Enterprise) have plan_status === null;
	// a fully-cancelled subscription is 'cancelled' (org.plan still holds the old
	// paid plan) — neither is manageable here, so neither should show
	// Manage/Cancel/Resume/Reactivate.
	let hasManageableSubscription = $derived(
		!isFree && (isActive || isTrialing || isPastDue || isPaused || isPendingCancellation)
	);

	// "Can this org pay?", so an invoice buyer sees no card warning.
	let hasPaymentMethod = $derived(canPay(org));
	// Stripe-managed plan that needs a card on file but has none.
	const configQuery = useConfigQuery();
	let missingCard = $derived(
		isMissingPaymentMethod(org, configQuery.data?.billing_enabled ?? false)
	);
	let trialEndDate = $derived(org?.trial_end_date ? new Date(org.trial_end_date) : null);

	// Renewal / subscription-ends label for the current plan; null when not
	// applicable (trialing has its own dedicated trial-ends-on line above;
	// Free / paused / past_due / cancelled don't surface a date here).
	let currentPlanRenewalLine = $derived(renewalLabel(org));
	let trialDaysLeft = $derived.by(() => {
		if (!trialEndDate) return null;
		const now = new Date();
		const diff = trialEndDate.getTime() - now.getTime();
		return Math.max(0, Math.ceil(diff / (1000 * 60 * 60 * 24)));
	});

	// Extend trial is offered to any trialing org in the final stretch that
	// hasn't already used its one-time extension — independent of card status.
	let canExtendTrial = $derived(
		isTrialing && trialDaysLeft !== null && trialDaysLeft <= 3 && !org?.trial_extended_used
	);

	// Change Plan moves into the overflow menu (rather than the primary) when the
	// primary slot is taken by Add Payment Method, i.e. an active/trialing org
	// missing a card.
	let showChangePlanItem = $derived(missingCard && (isActive || isTrialing));
	// Cancel is available while on a live, manageable active/trial subscription.
	let showCancelItem = $derived(hasManageableSubscription && (isActive || isTrialing));
	// Stripe portal (invoices + card management) for any manageable sub except
	// the states whose primary already opens the portal / has no portal value.
	let showPortalItem = $derived(hasManageableSubscription && !isPastDue && !isPaused);

	// The single primary action for the current state.
	let primaryAction = $derived.by(() => {
		if (missingCard)
			return { label: billing_addPaymentMethod(), onclick: handleSetupPayment, icon: CreditCard };
		if (isPastDue)
			return { label: settings_billing_updatePaymentMethod(), onclick: handleManageSubscription };
		if (isPaused)
			return {
				label: settings_billing_resume_button(),
				onclick: handleResume,
				disabled: resumeMutation.isPending
			};
		if (isPendingCancellation)
			return {
				label: settings_billing_reactivateSubscription(),
				onclick: handleReactivate,
				disabled: reactivateMutation.isPending
			};
		return {
			label: hasManageableSubscription
				? settings_billing_changePlan()
				: settings_billing_upgradePlan(),
			onclick: openPlanPicker
		};
	});

	// Extend trial gets its own prominent secondary CTA under the primary
	// (rather than being buried in the menu) when it's available.
	let secondaryAction = $derived(
		canExtendTrial
			? {
					label: settings_billing_extendTrial_link(),
					onclick: handleExtendTrial,
					disabled: extendTrialMutation.isPending
				}
			: undefined
	);

	// Ancillary actions tucked behind the "More Actions" menu.
	let menuItems = $derived.by(() => {
		const items: ButtonMenuItem[] = [];
		if (showChangePlanItem)
			items.push({ label: settings_billing_changePlan(), onclick: openPlanPicker });
		if (showPortalItem)
			items.push({
				label: settings_billing_paymentAndInvoices(),
				onclick: handleManageSubscription
			});
		// Cancel is the destructive action — always last in the list.
		if (showCancelItem)
			items.push({
				label: settings_billing_cancelSubscription(),
				onclick: openCancelModal,
				tone: 'danger'
			});
		return items;
	});

	// Single source of status messaging for the whole tab. One button-less
	// banner replaces the former trial/no-payment cards and the in-card
	// Inline* chain so each state shows exactly one status message.
	let statusBanner = $derived.by(() => {
		if (!org) return null;
		if (isTrialing && !hasPaymentMethod) {
			const date =
				trialEndDate?.toLocaleDateString(undefined, {
					month: 'long',
					day: 'numeric',
					year: 'numeric'
				}) ?? '';
			// A licensed org's trial ends in a dead license key, not a downgraded cloud
			// account, so it gets the license wording.
			return {
				kind: 'warning' as const,
				message: `${settings_billing_trialCountdown({ days: trialDaysLeft ?? 0, date })} ${
					isLicensedPlan
						? settings_billing_license_addPaymentMethodSubtitle()
						: settings_billing_addPaymentMethodSubtitle()
				}`
			};
		}
		if (isPastDue) return { kind: 'danger' as const, message: settings_billing_pastDue() };
		if (missingCard)
			return { kind: 'warning' as const, message: billing_noPaymentMethodBannerBody() };
		if (org.plan_status === 'cancelled')
			return { kind: 'warning' as const, message: settings_billing_canceled() };
		if (isPendingCancellation)
			return { kind: 'warning' as const, message: settings_billing_downgrade_pending() };
		return null;
	});

	// Render the active save-offer discount chip only while the discount window is
	// still in the future, and only on Stripe-managed plans — a coupon needs a
	// Stripe sub to attach to. The License tab quotes the same numbers when it
	// charges a trial out, so the math lives in one place.
	let activeDiscount = $derived(saveOfferDiscount(org));
	let discountedPriceLabel = $derived(discountedPrice(org));

	// Show the Usage card only when the plan defines at least one metered
	// resource (Free plans may define none).
	let hasAnyUsageRow = $derived(
		!!org?.plan &&
			(org.plan.included_seats !== null ||
				org.plan.included_networks !== null ||
				org.plan.included_hosts !== null)
	);

	// Track billing tab view
	$effect(() => {
		if (isOpen && org) {
			trackEvent('billing_tab_viewed', {
				plan_type: org.plan?.type,
				plan_status: org.plan_status
			});
		}
	});

	// Trial-card impression: once per browser tab session
	$effect(() => {
		if (isTrialing && trialDaysLeft !== null && !hasPaymentMethod) {
			trackOncePerSession('trial_card_impression', 'trial_card_impression', {
				trial_days_left: trialDaysLeft,
				has_payment_method: hasPaymentMethod
			});
		}
	});

	function openPlanPicker() {
		triggerUpgrade({
			source: 'settings_billing',
			surface: 'billing_tab',
			reopenSettings: true,
			beforeModal: () => onClose()
		});
	}

	async function handleManageSubscription() {
		// New tab — this tab stays put, so track immediately rather than stashing
		// the event for a post-redirect flush.
		trackEvent('billing_portal_opened', { plan_type: org?.plan?.type });
		// Snapshot the billing-relevant fields so the poller can detect whatever the
		// user changes in the (open-ended) Stripe portal.
		const before = {
			plan_status: org?.plan_status,
			plan_type: org?.plan?.type,
			has_payment_method: org?.has_payment_method
		};
		// Open synchronously so popup blockers allow it; point it at Stripe once ready.
		// (No 'noopener' — that makes window.open return null, losing the handle.)
		const stripeTab = window.open('', '_blank');
		try {
			const url = await customerPortalMutation.mutateAsync();
			if (!url) {
				stripeTab?.close();
				return;
			}
			if (stripeTab) {
				stripeTab.location.href = url;
				void waitForOrgUpdate(
					(o) =>
						o.plan_status !== before.plan_status ||
						o.plan?.type !== before.plan_type ||
						(o.has_payment_method ?? false) !== (before.has_payment_method ?? false)
				);
			} else {
				window.location.href = url;
			}
		} catch {
			stripeTab?.close();
			// Error handling is done by the mutation's onError
		}
	}

	function openCancelModal() {
		trackEvent('cancel_modal_opened', { plan_type: org?.plan?.type });
		showCancelModal = true;
	}

	async function handleResume() {
		if (!confirm(settings_billing_resume_confirmBody())) return;
		try {
			await resumeMutation.mutateAsync();
			const flipped = await waitForOrgUpdate((o) => o.plan_status === 'active');
			if (flipped) {
				pushSuccess(
					'Subscription resumed. A credit for the days you paused will be applied to your next invoice.'
				);
			} else {
				pushWarning(
					'Resume request accepted. It may take a moment to reflect across your account.'
				);
			}
			organizationQuery.refetch();
		} catch {
			// Mutation onError handles toast.
		}
	}

	async function handleReactivate() {
		try {
			await reactivateMutation.mutateAsync();
			const flipped = await waitForOrgUpdate((o) => o.plan_status === 'active');
			if (flipped) {
				pushSuccess(billing_subscriptionReactivated());
			} else {
				pushWarning(billing_requestAccepted());
			}
			organizationQuery.refetch();
		} catch {
			// Mutation onError handles toast.
		}
	}

	async function handleExtendTrial() {
		if (!confirm(settings_billing_extendTrial_confirmBody())) return;
		try {
			await extendTrialMutation.mutateAsync();
			const flipped = await waitForOrgUpdate((o) => o.trial_extended_used === true);
			if (flipped) {
				pushSuccess(billing_trialExtended());
			} else {
				pushWarning(billing_requestAccepted());
			}
			organizationQuery.refetch();
		} catch {
			// Mutation onError handles toast.
		}
	}

	function handleSetupPayment() {
		startSetupPayment({ org, source: 'billing_tab', trialDaysLeft });
	}
</script>

{#snippet usageRow(label: string, used: number, included: number, overageCents: number | null)}
	{@const expandable = overageCents != null && overageCents > 0}
	{@const over = used > included}
	{@const atCap = !expandable && used >= included}
	<div class="border-t pt-3" style="border-color: var(--color-border)">
		<div class="flex items-baseline justify-between gap-3">
			<div>
				<p class="text-primary font-medium">{label}</p>
				<p class="text-secondary text-sm">
					{common_billingUsage({ count: used, included })}
					{#if overageCents != null && overageCents > 0 && over}
						{common_billingExtra({ extra: used - included, price: overageCents / 100 })}
					{:else})
					{/if}
				</p>
			</div>
			{#if overageCents != null && overageCents > 0}
				<!-- Expandable: surface the add-on price so it's clear more can be
				     bought; exceeding the included count is normal paid usage, not a
				     warning, so it stays neutral (never amber). -->
				{#if over}
					<div class="text-right">
						<p class="text-primary text-xl font-bold">
							+${((used - included) * overageCents) / 100}
						</p>
						<p class="text-secondary text-xs">
							{settings_billing_per({ rate: org?.plan?.rate ?? 'Month' })}
						</p>
					</div>
				{:else}
					<p class="text-secondary text-sm">
						{settings_billing_usageAddOn({ price: overageCents / 100 })}
					</p>
				{/if}
			{:else if atCap}
				<!-- Hard cap: can't buy more on this plan — flag it and point to the
				     only way to get more. -->
				<div class="text-right">
					<p class="text-secondary text-sm">{common_atLimit()}</p>
					<button type="button" onclick={openPlanPicker} class="text-link text-xs hover:underline">
						{settings_billing_usageUpgradeToAddMore()}
					</button>
				</div>
			{:else}
				<p class="text-tertiary text-sm">{common_included()}</p>
			{/if}
		</div>
		{#if used > 0}
			<ProgressTrack
				class="mt-2 w-full"
				progress={Math.min(100, (used / (included || 1)) * 100)}
				color="bg-blue-500"
			/>
		{/if}
	</div>
{/snippet}

<div class="flex min-h-0 flex-1 flex-col">
	<div class="flex-1 overflow-auto p-6">
		{#if org}
			<div class="space-y-6">
				<!-- Single status banner: one message per state, no embedded action. -->
				{#if statusBanner}
					{#if statusBanner.kind === 'danger'}
						<InlineDanger title={statusBanner.message} />
					{:else}
						<InlineWarning title={statusBanner.message} />
					{/if}
				{/if}

				<!-- Current Plan: status, price, and the single CTA section. -->
				<InfoCard>
					<svelte:fragment slot="default">
						<div class="mb-3 flex items-center justify-between">
							<h3 class="text-primary text-sm font-semibold">{settings_billing_currentPlan()}</h3>
							<div class="flex items-center gap-2">
								<StatusIcon class={`h-4 w-4 ${planStatusColor}`} />
								<span class={`text-sm font-medium ${planStatusColor}`}>
									{planStatuses.getName(org.plan_status ?? null)}
								</span>
							</div>
						</div>

						<div class="space-y-4">
							{#if org.plan}
								<!-- Base Plan -->
								<div class="flex items-baseline justify-between">
									<div>
										<p class="text-primary text-lg font-semibold">
											{billingPlans.getName(org.plan.type || null)}
										</p>
										{#if isTrialing && trialEndDate}
											<p class="text-secondary mt-1 text-xs">
												{settings_billing_trialEndsOn({
													date: trialEndDate.toLocaleDateString(undefined, {
														month: 'long',
														day: 'numeric',
														year: 'numeric'
													})
												})}
											</p>
										{:else if currentPlanRenewalLine}
											<p class="text-secondary mt-1 text-xs">{currentPlanRenewalLine}</p>
										{/if}
										{#if activeDiscount}
											<p
												class="mt-1 inline-block rounded-md bg-green-100 px-2 py-0.5 text-xs font-medium text-green-700 dark:bg-green-900/30 dark:text-green-300"
											>
												{#if activeDiscount.rate === 'Year'}
													{settings_billing_discount_active_yearly({
														percentOff: activeDiscount.percentOff
													})}
												{:else}
													{settings_billing_discount_active({
														percentOff: activeDiscount.percentOff,
														expiresAt: activeDiscount.expiresAt
													})}
												{/if}
											</p>
										{/if}
									</div>
									<div class="text-right">
										{#if activeDiscount && discountedPriceLabel}
											<p class="text-tertiary text-sm line-through">
												${org.plan.base_cents / 100}
											</p>
											<p class="text-primary text-2xl font-bold">
												${discountedPriceLabel}
											</p>
										{:else}
											<p class="text-primary text-2xl font-bold">
												${org.plan.base_cents / 100}
											</p>
										{/if}
										<p class="text-secondary text-xs">
											{settings_billing_per({ rate: org.plan.rate })}
										</p>
									</div>
								</div>
							{/if}

							{#if invoiceBilling}
								<InvoiceBillingCard status={invoiceBilling} />
							{/if}

							<!-- CTA section: one primary action; every ancillary action
							     collapses into the caret menu so the section stays a single
							     control. Add Payment Method / Update Payment Method / Resume /
							     Reactivate take the primary slot ahead of plan changes. -->
							<ButtonMenu
								label={primaryAction.label}
								onclick={primaryAction.onclick}
								icon={primaryAction.icon}
								disabled={primaryAction.disabled ?? false}
								{secondaryAction}
								items={menuItems}
							/>
						</div>
					</svelte:fragment>
				</InfoCard>

				<!-- Usage -->
				{#if hasAnyUsageRow && org.plan && !isLicensedPlan}
					<InfoCard title={common_usage()}>
						<div class="space-y-4">
							{#if org.plan.included_seats !== null}
								{@render usageRow(
									common_seats(),
									seatCount,
									org.plan.included_seats ?? 0,
									org.plan.seat_cents ?? null
								)}
							{/if}
							{#if org.plan.included_networks !== null}
								{@render usageRow(
									common_networks(),
									networkCount,
									org.plan.included_networks ?? 0,
									org.plan.network_cents ?? null
								)}
							{/if}
							{#if org.plan.included_hosts !== null}
								{@render usageRow(
									common_hosts(),
									hostCount,
									org.plan.included_hosts ?? 0,
									org.plan.host_cents ?? null
								)}
							{/if}
						</div>
					</InfoCard>
				{/if}

				<!-- Additional Info -->
				<InfoCard title={settings_billing_needHelp()}>
					<p class="text-secondary text-sm">
						{settings_billing_contactUs()}
						<a href="mailto:billing@scanopy.net" class="text-link hover:underline"
							>billing@scanopy.net</a
						>
						{settings_billing_billingQuestions()}
					</p>
				</InfoCard>
			</div>
		{:else}
			<div class="text-secondary py-8 text-center">
				<p>{settings_billing_unableToLoad()}</p>
				<p class="text-tertiary mt-2 text-sm">{common_tryAgainLater()}</p>
			</div>
		{/if}
	</div>

	<!-- Footer -->
	{#if dismissible}
		<div class="modal-footer">
			<div class="flex justify-end">
				<button type="button" onclick={onClose} class="btn-secondary">{common_close()}</button>
			</div>
		</div>
	{/if}
</div>

<CancelSubscriptionModal
	isOpen={showCancelModal}
	onClose={() => (showCancelModal = false)}
	lastPausedAt={org?.last_paused_at ?? null}
	lastDiscountAt={org?.last_discount_at ?? null}
	planStatus={org?.plan_status ?? null}
	planType={org?.plan?.type ?? null}
	planRate={org?.plan?.rate ?? null}
	nextRenewalAt={org?.next_renewal_at ?? null}
	onSubscriptionChanged={() => organizationQuery.refetch()}
/>
