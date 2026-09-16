<script lang="ts">
	/**
	 * BillingPlanForm Component
	 *
	 * Card-based layout with pricing simulator, incremental feature highlights,
	 * and expandable full comparison grid. Responsive: 1 col mobile, 2 col tablet,
	 * auto-fit desktop.
	 */
	import { untrack } from 'svelte';
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';
	import { Check, X, ChevronDown, ChevronUp, Loader2, Minus, Plus } from 'lucide-svelte';
	import {
		billing_compareAllFeatures,
		billing_dayFreeTrial,
		billing_daysCount,
		billing_everythingInPlanPlus,
		billing_firstInvoiceOn,
		billing_networkUnit,
		billing_networkUnitPlural,
		billing_notIncluded,
		billing_perHostMonthly,
		billing_perNetworkMonthly,
		billing_perSeatMonthly,
		billing_priceBase,
		billing_rateMonthly,
		billing_rateMonthlyEquivalent,
		billing_rateYearly,
		billing_rateYearlyBilled,
		billing_seatUnit,
		billing_seatUnitPlural,
		billing_selfHosted,
		billing_showFeatures,
		billing_hideFeatures,
		billing_snapshotRetention,
		billing_startTrialNoCreditCard,
		billing_switchPlan,
		billing_trialContinues,
		billing_viewOnGithub,
		billing_yourCurrentPlan,
		common_cloud,
		common_comingSoon,
		common_custom,
		common_feature,
		common_getStarted,
		common_hide,
		common_hosts,
		common_monthly,
		common_networks,
		common_recommended,
		common_seats,
		common_unlimited,
		common_yearly
	} from '$lib/paraglide/messages';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import ToggleGroup from './ToggleGroup.svelte';
	import type { BillingPlan, PlanPickerHosting } from './types';
	import type { BillingPlanMetadata, FeatureMetadata } from '$lib/shared/stores/metadata';
	import type { ColorStyle } from '$lib/shared/utils/styling';
	import type { IconComponent } from '$lib/shared/utils/types';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import { useConfigQuery } from '$lib/shared/stores/config-query';

	const configQuery = useConfigQuery();

	/**
	 * Interface for metadata helpers props.
	 * Both app store helpers and website fixture helpers satisfy this interface.
	 */
	interface MetadataHelpers<T> {
		getMetadata: (id: string | null) => T;
		getDescription: (id: string | null) => string;
		getName: (id: string | null) => string;
		getCategory: (id: string | null) => string;
		getIconComponent: (id: string | null) => IconComponent;
		getColorHelper: (id: string | null) => ColorStyle;
	}

	// ============================================================================
	// Props
	// ============================================================================

	interface Props {
		plans: BillingPlan[];
		billingPlanHelpers: MetadataHelpers<BillingPlanMetadata>;
		featureHelpers: MetadataHelpers<FeatureMetadata>;
		onPlanSelect: (plan: BillingPlan) => void | Promise<void>;
		showGithubStars?: boolean;
		/** Show the Cloud / Self-Hosted toggle (and hosting tags on each card). */
		showHosting?: boolean;
		/** Tab the Cloud / Self-Hosted toggle opens on (only used with showHosting). */
		initialHosting?: PlanPickerHosting;
		class?: string;
		recommendedPlan?: string | null;
		/** If true, user is a returning customer and should not see trial offers */
		isReturningCustomer?: boolean;
		/** If true, user is currently on an active trial */
		isCurrentlyTrialing?: boolean;
		/** The org's current plan type — shown as non-selectable "Your current plan". */
		currentPlanType?: string | null;
	}

	// eslint-disable-next-line svelte/no-unused-props
	let {
		plans,
		billingPlanHelpers,
		featureHelpers,
		onPlanSelect,
		showGithubStars = true,
		class: className = '',
		showHosting = false,
		initialHosting = 'cloud',
		recommendedPlan = null,
		isReturningCustomer = false,
		isCurrentlyTrialing = false,
		currentPlanType = null
	}: Props = $props();

	let loadingPlanType = $state<string | null>(null);
	let showFullComparison = $state(false);

	function firstInvoiceCaption(plan: BillingPlan): string {
		const ms = Date.now() + plan.trial_days * 24 * 60 * 60 * 1000;
		const dateStr = new Date(ms).toLocaleDateString(undefined, {
			month: 'long',
			day: 'numeric',
			year: 'numeric'
		});
		return billing_firstInvoiceOn({ date: dateStr });
	}

	type BillingPeriod = 'monthly' | 'yearly';
	let billingPeriod = $state<BillingPeriod>('yearly');

	const billingPeriodOptions = [
		{ value: 'monthly', label: common_monthly() },
		{ value: 'yearly', label: common_yearly(), badge: '-20%' }
	];

	// The modal content remounts on every open, so the initial tab is read once.
	let hostingFilter = $state<PlanPickerHosting>(untrack(() => initialHosting));

	const hostingOptions = [
		{ value: 'cloud', label: common_cloud() },
		{ value: 'self_hosted', label: billing_selfHosted() }
	];

	// Self-hosted is annual-only (the paid tiers ship yearly), so the Self-Hosted tab
	// always renders yearly rows and locks the Monthly/Yearly toggle to Yearly (disabled).
	let selfHostedActive = $derived(showHosting && hostingFilter === 'self_hosted');

	let filteredPlans = $derived.by(() => {
		let result = plans;
		if (showHosting) {
			result = result.filter((plan) => {
				const hosting = getHosting(plan);
				// Enterprise is hosting-agnostic ('Any') and tops both ladders.
				if (hostingFilter === 'cloud') return hosting === 'Cloud' || hosting === 'Any';
				return hosting === 'SelfHosted' || hosting === 'Any';
			});
		}
		const period: BillingPeriod = selfHostedActive ? 'yearly' : billingPeriod;
		result = result.filter((plan) => {
			// Free plan is always monthly (no yearly variant)
			if (billingPlanHelpers.getMetadata(plan.type)?.is_free) return true;
			if (period === 'monthly') return plan.rate === 'Month';
			if (period === 'yearly') return plan.rate === 'Year';
			return true;
		});
		// Sort Free first; everything else keeps fixture order (stable sort).
		result = [...result].sort((a, b) => {
			if (billingPlanHelpers.getMetadata(a.type)?.is_free) return -1;
			if (billingPlanHelpers.getMetadata(b.type)?.is_free) return 1;
			return 0;
		});
		return result;
	});

	// ============================================================================
	// Pricing simulator state
	// ============================================================================

	let extraSeats = $state<Record<string, number>>({});
	let extraNetworks = $state<Record<string, number>>({});

	function adjustExtra(
		store: Record<string, number>,
		planType: string,
		delta: number
	): Record<string, number> {
		const current = store[planType] ?? 0;
		const next = Math.max(0, current + delta);
		return { ...store, [planType]: next };
	}

	function getExtraSeats(planType: string): number {
		return extraSeats[planType] ?? 0;
	}

	function getExtraNetworks(planType: string): number {
		return extraNetworks[planType] ?? 0;
	}

	function hasExtras(plan: BillingPlan): boolean {
		return getExtraSeats(plan.type) > 0 || getExtraNetworks(plan.type) > 0;
	}

	function getEstimatedTotal(plan: BillingPlan): number {
		const seatExtra = getExtraSeats(plan.type) * (plan.seat_cents ?? 0);
		const netExtra = getExtraNetworks(plan.type) * (plan.network_cents ?? 0);
		return plan.base_cents + seatExtra + netExtra;
	}

	function formatCents(cents: number): string {
		const dollars = cents / 100;
		return dollars % 1 === 0 ? `$${dollars}` : `$${dollars.toFixed(2)}`;
	}

	// Reset extras when billing period changes
	let prevBillingPeriod = $state(untrack(() => billingPeriod));
	$effect(() => {
		if (billingPeriod !== prevBillingPeriod) {
			prevBillingPeriod = billingPeriod;
			extraSeats = {};
			extraNetworks = {};
		}
	});

	// ============================================================================
	// Full comparison data
	// ============================================================================

	function getFeatureValue(planType: string, featureKey: string): boolean | string | number | null {
		// `snapshot_retention_days` is a per-plan fixture value with a universal
		// env-var escape hatch (`SCANOPY_SNAPSHOT_RETENTION_DAYS_OVERRIDE`).
		// When the override is set on this deployment, it wins for every plan
		// — mirrors the backend's `BillingPlan::snapshot_retention_days`.
		if (featureKey === 'snapshot_retention_days') {
			const override = configQuery.data?.snapshot_retention_days_override;
			if (override != null) return override;
		}
		const metadata = billingPlanHelpers.getMetadata(planType);
		const features = metadata?.features as unknown as
			| Record<string, boolean | string | number | null>
			| undefined;
		return features?.[featureKey] ?? null;
	}

	function isComingSoon(featureKey: string): boolean {
		return featureHelpers.getMetadata(featureKey)?.is_coming_soon === true;
	}

	let featureKeys = $derived(
		filteredPlans.length > 0
			? Object.keys(billingPlanHelpers.getMetadata(filteredPlans[0].type)?.features || {})
			: []
	);

	// Group features by category for the full comparison
	let groupedFeatures = $derived.by(() => {
		const groups = new SvelteMap<string, string[]>();
		for (const featureKey of featureKeys) {
			const category = featureHelpers.getCategory(featureKey) || 'Other';
			if (!groups.has(category)) groups.set(category, []);
			groups.get(category)!.push(featureKey);
		}
		// Sort features within each category by how many plans enable them (most first)
		for (const [, features] of groups) {
			features.sort((a, b) => {
				const countA = filteredPlans.filter((p) => getFeatureValue(p.type, a)).length;
				const countB = filteredPlans.filter((p) => getFeatureValue(p.type, b)).length;
				return countB - countA;
			});
		}
		// Sort categories: Core first, Support/Enterprise/Licensing last
		const sortedEntries = [...groups.entries()].sort(([a], [b]) => {
			const order = [
				'Discovery',
				'Visualization',
				'Integrations',
				'Support',
				'Enterprise',
				'Licensing & Billing'
			];
			const aIdx = order.indexOf(a);
			const bIdx = order.indexOf(b);
			if (aIdx === -1 && bIdx === -1) return a.localeCompare(b);
			if (aIdx === -1) return 1;
			if (bIdx === -1) return -1;
			return aIdx - bIdx;
		});
		return new SvelteMap(sortedEntries);
	});

	// Grid column template for full comparison
	let gridColumns = $derived.by(() => {
		const planCount = filteredPlans.length;
		if (planCount === 0) return '120px 1fr';
		return `minmax(100px, 20%) repeat(${planCount}, minmax(100px, 1fr))`;
	});

	// ============================================================================
	// Helper functions
	// ============================================================================

	// Self-hosted commercial tiers publish a real annual price (custom_price is null,
	// rate is Year). They're priced annual-first ("$4,000/yr") with a monthly whisper
	// under it; cloud plans stay monthly-first.
	function isSelfHostedAnnual(plan: BillingPlan): boolean {
		const metadata = billingPlanHelpers.getMetadata(plan.type);
		return metadata?.hosting === 'SelfHosted' && !metadata?.custom_price && plan.rate === 'Year';
	}

	function formatDollars(cents: number): string {
		return `$${(cents / 100).toLocaleString('en-US')}`;
	}

	function formatBasePricing(plan: BillingPlan): string {
		const metadata = billingPlanHelpers.getMetadata(plan.type);
		if (metadata?.custom_price) return metadata.custom_price;
		if (isSelfHostedAnnual(plan)) return formatDollars(plan.base_cents);
		if (plan.rate === 'Year') return `$${plan.base_cents / 12 / 100}`;
		return `$${plan.base_cents / 100}`;
	}

	function formatRate(plan: BillingPlan): string {
		const metadata = billingPlanHelpers.getMetadata(plan.type);
		if (metadata?.custom_price) return '';
		if (isSelfHostedAnnual(plan)) return billing_rateYearly();
		if (plan.rate === 'Year') return billing_rateYearlyBilled();
		return billing_rateMonthly();
	}

	function formatSeatAddonPricing(plan: BillingPlan): string {
		if (plan.seat_cents) {
			const monthly = plan.rate === 'Year' ? plan.seat_cents / 12 : plan.seat_cents;
			return billing_perSeatMonthly({ amount: String(monthly / 100) });
		}
		return '';
	}

	function formatNetworkAddonPricing(plan: BillingPlan): string {
		if (plan.network_cents) {
			const monthly = plan.rate === 'Year' ? plan.network_cents / 12 : plan.network_cents;
			return billing_perNetworkMonthly({ amount: String(monthly / 100) });
		}
		return '';
	}

	function formatHostAddonPricing(plan: BillingPlan): string {
		if (plan.host_cents) {
			const monthly = plan.rate === 'Year' ? plan.host_cents / 12 : plan.host_cents;
			return billing_perHostMonthly({ amount: String(monthly / 100) });
		}
		return '';
	}

	function getHosting(plan: BillingPlan): string {
		return billingPlanHelpers.getMetadata(plan.type)?.hosting ?? '';
	}

	function hasTrial(plan: BillingPlan): boolean {
		return !isReturningCustomer && plan.trial_days > 0;
	}

	function hasCustomPrice(plan: BillingPlan): boolean {
		return billingPlanHelpers.getMetadata(plan.type)?.custom_price !== null;
	}

	async function handlePlanSelect(plan: BillingPlan) {
		loadingPlanType = plan.type;
		try {
			await onPlanSelect(plan);
		} finally {
			loadingPlanType = null;
		}
	}

	function formatIncludedValue(value: number | null | undefined, plan?: BillingPlan): string {
		if (value == null && plan && hasCustomPrice(plan)) return common_custom();
		return value == null ? common_unlimited() : String(value);
	}

	function formatSnapshotRetention(plan: BillingPlan): string {
		// Self-hosted deployments set their own retention window, so no fixed number is
		// published for them — unless this deployment carries the universal override,
		// which getFeatureValue already applies and which wins for every plan.
		const override = configQuery.data?.snapshot_retention_days_override;
		if (override == null && getHosting(plan) === 'SelfHosted') return common_custom();
		const value = getFeatureValue(plan.type, 'snapshot_retention_days');
		if (value === 0) return billing_notIncluded();
		if (typeof value === 'number') return billing_daysCount({ count: value });
		return '—';
	}

	function sortFeaturesByCategory(features: string[]): string[] {
		const order = ['Discovery', 'Visualization', 'Integrations', 'Support', 'Enterprise'];
		return [...features].sort((a, b) => {
			// Coming-soon features sort to end
			const soonA = isComingSoon(a) ? 1 : 0;
			const soonB = isComingSoon(b) ? 1 : 0;
			if (soonA !== soonB) return soonA - soonB;
			const catA = order.indexOf(featureHelpers.getCategory(a));
			const catB = order.indexOf(featureHelpers.getCategory(b));
			return (catA === -1 ? 99 : catA) - (catB === -1 ? 99 : catB);
		});
	}

	// ============================================================================
	// Mobile feature list toggle
	// ============================================================================

	let expandedFeatures = $state(new Set<string>());

	function toggleFeatures(planType: string) {
		const next = new SvelteSet(expandedFeatures);
		if (next.has(planType)) {
			next.delete(planType);
		} else {
			next.add(planType);
		}
		expandedFeatures = next;
	}
</script>

<div class="flex min-h-0 flex-1 flex-col {className}">
	<!-- Header with Toggles (fixed, does not scroll) -->
	<div class="flex shrink-0 flex-wrap items-center justify-center gap-2 px-4 py-1 lg:px-6">
		{#if showGithubStars}
			<!-- <GithubStars /> -->
		{/if}

		{#if showHosting}
			<ToggleGroup
				options={hostingOptions}
				selected={hostingFilter}
				onchange={(value) => (hostingFilter = value as PlanPickerHosting)}
			/>
		{/if}

		<ToggleGroup
			options={billingPeriodOptions}
			selected={selfHostedActive ? 'yearly' : billingPeriod}
			onchange={(value) => (billingPeriod = value as BillingPeriod)}
			disabled={selfHostedActive}
		/>
	</div>

	<!-- Scrollable content -->
	<div class="min-h-0 flex-1 overflow-y-auto">
		<!-- Plan Cards -->
		<div class="plan-cards-container px-4 lg:px-6">
			<div class="plan-cards-grid">
				{#each filteredPlans as plan (plan.type + plan.rate)}
					{@const IconComponent = billingPlanHelpers.getIconComponent(plan.type)}
					{@const colorHelper = billingPlanHelpers.getColorHelper(plan.type)}
					{@const isRecommended = recommendedPlan === plan.type}
					{@const description = billingPlanHelpers.getDescription(plan.type)}
					{@const trial = hasTrial(plan)}
					{@const metadata = billingPlanHelpers.getMetadata(plan.type)}
					{@const incrementalFeatures = metadata?.incremental_features ?? []}
					{@const prevTier = metadata?.previous_tier}
					{@const prevTierVisible = prevTier
						? filteredPlans.some((p) => p.type === prevTier)
						: false}
					{@const prevTierFeatures =
						prevTier && !prevTierVisible
							? (billingPlanHelpers.getMetadata(prevTier)?.incremental_features ?? [])
							: []}
					{@const displayFeatures = sortFeaturesByCategory(
						prevTierFeatures.length > 0
							? [...new Set([...prevTierFeatures, ...incrementalFeatures])]
							: incrementalFeatures
					)}

					<div
						class="plan-card card card-static flex flex-col {isRecommended
							? 'plan-card-recommended'
							: ''}"
					>
						<!-- Recommended Badge -->
						{#if isRecommended}
							<div class="-mt-3 mb-1 flex justify-center">
								<Tag label={common_recommended()} color="Yellow" />
							</div>
						{/if}

						<!-- Plan Header -->
						<div class="flex flex-col items-center gap-2 pb-4">
							<div class="flex items-center gap-2">
								<IconComponent class="{colorHelper.icon} h-5 w-5 lg:h-6 lg:w-6" />
								<span class="text-primary text-base font-semibold lg:text-lg">
									{billingPlanHelpers.getName(plan.type)}
								</span>
							</div>
						</div>

						<!-- Pricing -->
						<div class="flex flex-col items-center gap-1 pb-4">
							<div class="flex items-baseline gap-1">
								<span class="text-primary text-2xl font-bold lg:text-3xl">
									{hasExtras(plan)
										? formatCents(
												plan.rate === 'Year'
													? getEstimatedTotal(plan) / 12
													: getEstimatedTotal(plan)
											)
										: formatBasePricing(plan)}
								</span>
								{#if formatRate(plan)}
									<span class="text-tertiary text-sm">{formatRate(plan)}</span>
								{/if}
							</div>
							{#if hasExtras(plan)}
								<div class="text-tertiary text-center text-xs">
									{billing_priceBase()}
									{formatCents(plan.rate === 'Year' ? plan.base_cents / 12 : plan.base_cents)}
									{#if getExtraSeats(plan.type) > 0}
										{@const seatCost = getExtraSeats(plan.type) * (plan.seat_cents ?? 0)}
										+ {getExtraSeats(plan.type)}
										{getExtraSeats(plan.type) === 1 ? billing_seatUnit() : billing_seatUnitPlural()}
										({formatCents(plan.rate === 'Year' ? seatCost / 12 : seatCost)})
									{/if}
									{#if getExtraNetworks(plan.type) > 0}
										{@const netCost = getExtraNetworks(plan.type) * (plan.network_cents ?? 0)}
										+ {getExtraNetworks(plan.type)}
										{getExtraNetworks(plan.type) === 1
											? billing_networkUnit()
											: billing_networkUnitPlural()} ({formatCents(
											plan.rate === 'Year' ? netCost / 12 : netCost
										)})
									{/if}
								</div>
							{/if}
							{#if selfHostedActive}
								<!-- Annual price on the card, monthly equivalent under it. The
								     invisible copy keeps card rows aligned for plans with no annual price. -->
								<div
									class={`text-tertiary text-center text-xs ${isSelfHostedAnnual(plan) && !hasExtras(plan) ? 'opacity-100' : 'opacity-0'}`}
								>
									{billing_rateMonthlyEquivalent({
										amount: isSelfHostedAnnual(plan) ? formatCents(plan.base_cents / 12) : ''
									})}
								</div>
							{/if}
							<div
								class={`text-xs font-medium text-success ${(hasTrial(plan) || (isCurrentlyTrialing && plan.trial_days > 0)) && !hasCustomPrice(plan) ? 'opacity-100' : 'opacity-0'}`}
							>
								{isCurrentlyTrialing
									? billing_trialContinues()
									: billing_dayFreeTrial({ days: plan.trial_days })}
							</div>
						</div>

						<!-- Description -->
						{#if description}
							<p class="text-tertiary pb-4 text-center text-xs leading-relaxed lg:text-sm">
								{description}
							</p>
						{/if}

						<!-- CTA Button -->
						<div class="py-4" style="border-color: var(--color-border)">
							{#if metadata?.purchase_flow === 'stripe' || metadata?.is_free}
								<!-- Free has purchase_flow 'none' but activates in-app like a Stripe plan -->
								{#if plan.type === currentPlanType}
									<InlineInfo title="" body={billing_yourCurrentPlan()} />
								{:else}
									<button
										type="button"
										onclick={() => handlePlanSelect(plan)}
										disabled={loadingPlanType !== null}
										class="btn-primary w-full text-sm"
									>
										{#if loadingPlanType === plan.type}
											<Loader2 class="mx-auto h-4 w-4 animate-spin" />
										{:else if isCurrentlyTrialing}
											{billing_switchPlan()}
										{:else}
											{trial ? billing_startTrialNoCreditCard() : common_getStarted()}
										{/if}
									</button>
									{#if trial && !isCurrentlyTrialing}
										<div class="text-tertiary pt-2 text-center text-xs">
											{firstInvoiceCaption(plan)}
										</div>
									{/if}
								{/if}
							{:else}
								<a
									href="https://github.com/scanopy/scanopy"
									target="_blank"
									rel="noopener noreferrer"
									class="btn-secondary inline-block w-full text-center text-sm"
								>
									{billing_viewOnGithub()}
								</a>
							{/if}
						</div>

						<!-- Included Resources with Stepper Controls -->
						<div class="space-y-2 border-b pb-4" style="border-color: var(--color-border)">
							<!-- Seats -->
							<div class="flex items-center justify-between text-sm">
								<div class="flex flex-col">
									<span class="text-secondary">{common_seats()}</span>
									{#if plan.seat_cents}
										<span class="text-tertiary text-xs">{formatSeatAddonPricing(plan)}</span>
									{/if}
								</div>
								{#if plan.seat_cents && plan.included_seats !== null}
									<div class="stepper">
										<button
											type="button"
											class="stepper-btn"
											disabled={getExtraSeats(plan.type) === 0}
											onclick={() => (extraSeats = adjustExtra(extraSeats, plan.type, -1))}
										>
											<Minus class="h-3 w-3" />
										</button>
										<span class="text-primary w-8 text-center text-sm font-medium">
											{(plan.included_seats ?? 0) + getExtraSeats(plan.type)}
										</span>
										<button
											type="button"
											class="stepper-btn"
											onclick={() => (extraSeats = adjustExtra(extraSeats, plan.type, 1))}
										>
											<Plus class="h-3 w-3" />
										</button>
									</div>
								{:else}
									<span class="text-primary font-medium">
										{formatIncludedValue(plan.included_seats, plan)}
									</span>
								{/if}
							</div>

							<!-- Networks -->
							<div class="flex items-center justify-between text-sm">
								<div class="flex flex-col">
									<span class="text-secondary">{common_networks()}</span>
									{#if plan.network_cents}
										<span class="text-tertiary text-xs">{formatNetworkAddonPricing(plan)}</span>
									{/if}
								</div>
								{#if plan.network_cents && plan.included_networks !== null}
									<div class="stepper">
										<button
											type="button"
											class="stepper-btn"
											disabled={getExtraNetworks(plan.type) === 0}
											onclick={() => (extraNetworks = adjustExtra(extraNetworks, plan.type, -1))}
										>
											<Minus class="h-3 w-3" />
										</button>
										<span class="text-primary w-8 text-center text-sm font-medium">
											{(plan.included_networks ?? 0) + getExtraNetworks(plan.type)}
										</span>
										<button
											type="button"
											class="stepper-btn"
											onclick={() => (extraNetworks = adjustExtra(extraNetworks, plan.type, 1))}
										>
											<Plus class="h-3 w-3" />
										</button>
									</div>
								{:else}
									<span class="text-primary font-medium">
										{formatIncludedValue(plan.included_networks, plan)}
									</span>
								{/if}
							</div>

							<!-- Hosts -->
							<div class="flex items-center justify-between text-sm">
								<div class="flex flex-col">
									<span class="text-secondary">{common_hosts()}</span>
									{#if plan.host_cents}
										<span class="text-tertiary text-xs">{formatHostAddonPricing(plan)}</span>
									{/if}
								</div>
								<span class="text-primary font-medium">
									{formatIncludedValue(plan.included_hosts, plan)}
								</span>
							</div>

							<!-- Snapshot Retention -->
							<div class="flex items-center justify-between text-sm">
								<span class="text-secondary">{billing_snapshotRetention()}</span>
								<span class="text-primary font-medium">
									{formatSnapshotRetention(plan)}
								</span>
							</div>
						</div>

						<!-- Incremental Features -->
						<div class="flex-1 py-4">
							{#if prevTier && prevTierVisible}
								<p class="text-secondary mb-2 text-xs font-medium">
									{billing_everythingInPlanPlus({ planName: billingPlanHelpers.getName(prevTier) })}
								</p>
							{/if}

							<!-- Mobile: collapsible feature list -->
							{#if displayFeatures.length > 0}
								<button
									type="button"
									class="text-tertiary mb-2 flex items-center gap-1 text-xs font-medium sm:hidden"
									onclick={() => toggleFeatures(plan.type)}
								>
									{expandedFeatures.has(plan.type)
										? billing_hideFeatures()
										: billing_showFeatures()}
									{#if expandedFeatures.has(plan.type)}
										<ChevronUp class="h-3 w-3" />
									{:else}
										<ChevronDown class="h-3 w-3" />
									{/if}
								</button>
							{/if}

							<ul class="space-y-1.5 {expandedFeatures.has(plan.type) ? '' : 'hidden sm:block'}">
								{#each displayFeatures as featureKey (featureKey)}
									{@const comingSoon = isComingSoon(featureKey)}
									<li class="flex items-start gap-2 text-sm">
										<Check
											class="mt-0.5 h-4 w-4 flex-shrink-0 {comingSoon
												? 'text-gray-500'
												: 'text-success'}"
										/>
										<span
											class={comingSoon ? 'text-tertiary' : 'text-secondary'}
											data-tooltip={featureHelpers.getDescription(featureKey)}
											use:tooltip>{featureHelpers.getName(featureKey)}</span
										>
										{#if comingSoon}
											<Tag label={common_comingSoon()} color="Gray" />
										{/if}
									</li>
								{/each}
							</ul>
						</div>
					</div>
				{/each}
			</div>
		</div>

		<!-- Compare All Features Toggle -->
		<div class="flex justify-center py-4">
			<button
				type="button"
				class="btn-primary flex items-center gap-2 text-sm"
				onclick={() => (showFullComparison = !showFullComparison)}
			>
				{showFullComparison ? common_hide() : billing_compareAllFeatures()}
				{#if showFullComparison}
					<ChevronUp class="h-4 w-4" />
				{:else}
					<ChevronDown class="h-4 w-4" />
				{/if}
			</button>
		</div>

		<!-- Full Comparison Grid (expandable) -->
		{#if showFullComparison}
			<div class="card mx-4 overflow-auto p-0 lg:mx-10">
				<!-- Plan Name Headers -->
				<div
					class="comparison-row comparison-header-row"
					style="grid-template-columns: {gridColumns}"
				>
					<div class="comparison-label-cell">
						<div class="text-xs font-medium lg:text-sm">{common_feature()}</div>
					</div>
					{#each filteredPlans as plan (plan.type)}
						<div class="comparison-value-cell">
							<span class="text-primary text-xs font-semibold lg:text-sm"
								>{billingPlanHelpers.getName(plan.type)}</span
							>
						</div>
					{/each}
				</div>

				{#each [...groupedFeatures.entries()] as [category, categoryFeatures] (category)}
					<!-- Category Header -->
					<div class="comparison-category-row">
						<span
							class="text-secondary p-2 text-xs font-semibold uppercase tracking-wide lg:p-3 lg:text-sm"
						>
							{category}
						</span>
					</div>

					{#each categoryFeatures as featureKey (featureKey)}
						{@const comingSoon = isComingSoon(featureKey)}
						<div class="comparison-row" style="grid-template-columns: {gridColumns}">
							<div class="comparison-label-cell">
								<div
									class="text-xs font-medium lg:text-sm"
									data-tooltip={featureHelpers.getDescription(featureKey)}
									use:tooltip
								>
									{featureHelpers.getName(featureKey)}
								</div>
							</div>
							{#each filteredPlans as plan (plan.type)}
								{@const value = getFeatureValue(plan.type, featureKey)}
								<div class="comparison-value-cell">
									{#if comingSoon && value}
										<Tag label={common_comingSoon()} color="Gray" />
									{:else if typeof value === 'boolean'}
										{#if value}
											<Check class="mx-auto h-4 w-4 text-success lg:h-5 lg:w-5" />
										{:else}
											<X class="text-muted mx-auto h-4 w-4 lg:h-5 lg:w-5" />
										{/if}
									{:else if typeof value === 'number' && value === 0}
										<X class="text-muted mx-auto h-4 w-4 lg:h-5 lg:w-5" />
									{:else if value === null}
										<span class="text-tertiary">&mdash;</span>
									{:else}
										<span class="text-secondary text-xs lg:text-sm">{value}</span>
									{/if}
								</div>
							{/each}
						</div>
					{/each}
				{/each}
			</div>
		{/if}
	</div>
	<!-- end scrollable content -->
</div>

<style>
	/* Card grid layout */
	.plan-cards-grid {
		display: grid;
		gap: 1rem;
		/* Mobile: single column vertical stack */
		grid-template-columns: 1fr;
	}

	@media (min-width: 640px) {
		.plan-cards-grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	@media (min-width: 1024px) {
		.plan-cards-grid {
			grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
			gap: 0.75rem;
		}
	}

	/* Individual plan card */
	.plan-card {
		padding: 1.25rem;
		position: relative;
	}

	.plan-card-recommended {
		outline: 2px solid rgb(234 179 8);
		outline-offset: -2px;
	}

	/* Stepper controls for pricing simulator */
	.stepper {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
	}

	.stepper-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 1.5rem;
		height: 1.5rem;
		border-radius: 0.25rem;
		border: 1px solid var(--color-border-input);
		color: var(--color-text-secondary);
		background: transparent;
		cursor: pointer;
		transition:
			background-color 150ms,
			border-color 150ms;
	}

	.stepper-btn:hover:not(:disabled) {
		background: var(--color-bg-surface-hover);
		border-color: var(--color-border-input);
	}

	.stepper-btn:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	/* ============================================ */
	/* Full comparison grid                         */
	/* ============================================ */

	.comparison-header-row {
		background: var(--color-bg-surface);
		position: sticky;
		top: 0;
		z-index: 11;
	}

	.comparison-category-row {
		border-bottom: 1px solid var(--color-border);
	}

	.comparison-row {
		display: grid;
		min-width: 500px;
		border-bottom: 1px solid var(--color-border);
	}

	.comparison-row:last-child {
		border-bottom: none;
	}

	.comparison-label-cell {
		padding: 0.5rem;
		color: var(--color-text-tertiary);
		text-align: left;
		display: flex;
		align-items: center;
		position: sticky;
		left: 0;
		z-index: 10;
		background: var(--color-bg-surface);
		border-right: 1px solid var(--color-border);
	}

	.comparison-value-cell {
		padding: 0.5rem;
		display: flex;
		align-items: center;
		justify-content: center;
		text-align: center;
		border-right: 1px solid var(--color-border);
	}

	.comparison-value-cell:last-child {
		border-right: none;
	}

	@media (min-width: 1024px) {
		.comparison-label-cell,
		.comparison-value-cell {
			padding: 0.75rem;
		}
	}

	/* Feature tooltips */
	[data-tooltip] {
		position: relative;
		cursor: help;
		text-decoration: underline dotted;
		text-decoration-color: var(--color-border-input);
		text-underline-offset: 2px;
	}
</style>
