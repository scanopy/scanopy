<script lang="ts">
	/**
	 * BillingPlanForm Component
	 *
	 * Card-based layout with explained plan limits, incremental feature highlights,
	 * and expandable full comparison grid. Responsive: 1 col mobile, 2 col tablet,
	 * auto-fit desktop.
	 */
	import { untrack } from 'svelte';
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';
	import { Check, X, ChevronDown, ChevronUp, Info, Loader2 } from '@lucide/svelte';
	import {
		billing_compareAllFeatures,
		billing_dayFreeTrial,
		billing_dayRetention,
		billing_everythingInPlanPlus,
		billing_extraSeatsMonthly,
		billing_extraSitesMonthly,
		billing_includedCount,
		billing_nonProductionCount,
		billing_rateMonthly,
		billing_rateMonthlyEquivalent,
		billing_rateYearly,
		billing_rateYearlyBilled,
		billing_hostingCloudTooltip,
		billing_hostingSelfHostedTooltip,
		billing_selfHosted,
		billing_showFeatures,
		billing_hideFeatures,
		billing_betaFeatureTooltip,
		billing_startDayFreeTrial,
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
		billing_continueOnPlan,
		common_hide,
		common_servers,
		common_monthly,
		common_sites,
		common_seats,
		common_unlimited,
		common_yearly
	} from '$lib/paraglide/messages';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import ToggleGroup from './ToggleGroup.svelte';
	import type { BillingPlan, PlanPickerHosting } from './types';
	import type {
		BillingPlanMetadata,
		CredentialIntegrationMetadata,
		FeatureMetadata
	} from '$lib/shared/stores/metadata';
	import type { ColorStyle } from '$lib/shared/utils/styling';
	import type { IconComponent } from '$lib/shared/utils/types';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import { isCloud, useConfigQuery } from '$lib/shared/stores/config-query';
	import { getStabilityTagProps } from '$lib/shared/utils/stability';
	import { betaProgramUrl } from '$lib/features/support/support';

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
		/** Feature names and metadata; `getIds` gives the backend's declaration order. */
		featureHelpers: MetadataHelpers<FeatureMetadata> & { getIds: () => string[] };
		/** Names and icons of the category headings features are grouped under, in display order
		 * (`product-areas.json`). */
		productAreaHelpers: MetadataHelpers<object> & { getIds: () => string[] };
		/** Names the discovery integrations itemized on each card (`credential-integrations.json`). */
		integrationHelpers: MetadataHelpers<CredentialIntegrationMetadata>;
		/** Explains each counted limit on the cards (`plan-limits.json`). */
		limitHelpers: MetadataHelpers<object>;
		onPlanSelect: (plan: BillingPlan) => void | Promise<void>;
		showGithubStars?: boolean;
		/** Show the Cloud / Self-Hosted toggle (and hosting tags on each card). */
		showHosting?: boolean;
		/** Tab the Cloud / Self-Hosted toggle opens on (only used with showHosting). */
		initialHosting?: PlanPickerHosting;
		class?: string;
		/** If true, user is a returning customer and should not see trial offers */
		isReturningCustomer?: boolean;
		/** If true, user is currently on an active trial */
		isCurrentlyTrialing?: boolean;
		/** The org's current plan type — shown as non-selectable "Your current plan". */
		currentPlanType?: string | null;
		/**
		 * The org lapsed on `currentPlanType` (subscription ended, no paid plan
		 * chosen since), so that plan is selectable again: continuing on it is a
		 * new subscription, not a no-op.
		 */
		currentPlanLapsed?: boolean;
	}

	// eslint-disable-next-line svelte/no-unused-props
	let {
		plans,
		billingPlanHelpers,
		featureHelpers,
		productAreaHelpers,
		integrationHelpers,
		limitHelpers,
		onPlanSelect,
		showGithubStars = true,
		class: className = '',
		showHosting = false,
		initialHosting = 'cloud',
		isReturningCustomer = false,
		isCurrentlyTrialing = false,
		currentPlanType = null,
		currentPlanLapsed = false
	}: Props = $props();

	let loadingPlanType = $state<string | null>(null);
	let showFullComparison = $state(false);

	type BillingPeriod = 'monthly' | 'yearly';
	let billingPeriod = $state<BillingPeriod>('yearly');

	const billingPeriodOptions = [
		{ value: 'monthly', label: common_monthly() },
		{ value: 'yearly', label: common_yearly(), badge: '-20%' }
	];

	// The modal content remounts on every open, so the initial tab is read once.
	let hostingFilter = $state<PlanPickerHosting>(untrack(() => initialHosting));

	const hostingOptions = [
		{ value: 'cloud', label: common_cloud(), tooltip: billing_hostingCloudTooltip() },
		{
			value: 'self_hosted',
			label: billing_selfHosted(),
			tooltip: billing_hostingSelfHostedTooltip()
		}
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

	function formatCents(cents: number): string {
		const dollars = cents / 100;
		return dollars % 1 === 0 ? `$${dollars}` : `$${dollars.toFixed(2)}`;
	}

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
			Record<string, boolean | string | number | null> | undefined;
		return features?.[featureKey] ?? null;
	}

	function isComingSoon(featureKey: string): boolean {
		return featureHelpers.getMetadata(featureKey)?.is_coming_soon === true;
	}

	/**
	 * A beta feature's tag, linking to the beta program sign-up; `null` for a stable feature, and
	 * for one still coming soon, which carries the Coming Soon tag instead.
	 */
	function betaTag(featureKey: string) {
		if (isComingSoon(featureKey)) return null;
		const cloud = configQuery.data ? isCloud(configQuery.data) : false;
		return getStabilityTagProps(
			featureHelpers.getMetadata(featureKey)?.stability,
			billing_betaFeatureTooltip(),
			betaProgramUrl('app_plans', cloud)
		);
	}

	const DISCOVERY_INTEGRATIONS = 'discovery_integrations';

	/** Feature categories in display order; unknown ones sort last. */
	let categoryOrder = $derived(productAreaHelpers.getIds());

	/** The category the integrations sit under, on the cards and in the comparison. */
	let discoveryCategory = $derived(featureHelpers.getCategory(DISCOVERY_INTEGRATIONS));

	/**
	 * Features the cards and the comparison show as the itemized integrations instead: the
	 * general discovery-integrations flag, and each feature that gates one integration.
	 */
	function isItemizedAsIntegrations(featureKey: string): boolean {
		return (
			featureKey === DISCOVERY_INTEGRATIONS ||
			featureHelpers.getMetadata(featureKey)?.gates_integration === true
		);
	}

	/**
	 * Each plan lists its integrations in the Integrations page's order. A merged list sorts by the
	 * longest plan list, which holds every integration the others do.
	 */
	let integrationRank = $derived.by(() => {
		const lists = plans.map((p) => billingPlanHelpers.getMetadata(p.type)?.integrations ?? []);
		const longest = lists.reduce<string[]>((a, b) => (b.length > a.length ? b : a), []);
		return new SvelteMap(longest.map((id, i) => [id, i]));
	});

	function sortIntegrations(ids: string[]): string[] {
		const rank = (id: string) => integrationRank.get(id) ?? Number.MAX_SAFE_INTEGER;
		return [...ids].sort((a, b) => rank(a) - rank(b));
	}

	/** Every integration any shown plan includes, in the Integrations page's order. */
	let comparisonIntegrations = $derived(
		sortIntegrations([
			...new Set(
				filteredPlans.flatMap((p) => billingPlanHelpers.getMetadata(p.type)?.integrations ?? [])
			)
		])
	);

	function planHasIntegration(planType: string, integration: string): boolean {
		return (billingPlanHelpers.getMetadata(planType)?.integrations ?? []).some(
			(id) => id === integration
		);
	}

	/** Every feature in the backend's declaration order (`features.json` order). */
	let featureOrder = $derived(featureHelpers.getIds());

	let featureKeys = $derived(
		filteredPlans.length > 0
			? featureOrder.filter(
					(key) =>
						key in (billingPlanHelpers.getMetadata(filteredPlans[0].type)?.features ?? {}) &&
						!isItemizedAsIntegrations(key) &&
						isListed(key)
				)
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
		// Features keep the backend's declaration order within each category.
		// Sort categories into display order
		const sortedEntries = [...groups.entries()].sort(([a], [b]) => {
			const order: string[] = categoryOrder;
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
			return billing_extraSeatsMonthly({ amount: String(monthly / 100) });
		}
		return '';
	}

	function formatSiteAddonPricing(plan: BillingPlan): string {
		if (plan.site_cents) {
			const monthly = plan.rate === 'Year' ? plan.site_cents / 12 : plan.site_cents;
			return billing_extraSitesMonthly({ amount: String(monthly / 100) });
		}
		return '';
	}

	/** The included count, worded as such when more can be bought past it. */
	function formatIncludedLimit(
		included: number | null | undefined,
		unitCents: number | null | undefined,
		plan: BillingPlan
	): string {
		if (unitCents && included != null) return billing_includedCount({ count: included });
		return formatIncludedValue(included, plan);
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

	const SNAPSHOT_RETENTION = 'snapshot_retention_days';

	/** "30 day retention", beside the Snapshots feature. */
	function formatSnapshotRetention(planType: string): string {
		const value = getFeatureValue(planType, SNAPSHOT_RETENTION);
		return typeof value === 'number' ? billing_dayRetention({ count: value }) : '';
	}

	/**
	 * Whether a card lists snapshot retention under Track & share: a cloud plan that keeps
	 * snapshots, for a different window than the visible tier below it. A self-hosted deployment
	 * sets its own window.
	 */
	function listsSnapshotRetention(
		plan: BillingPlan,
		prevTier: string | null | undefined,
		prevTierVisible: boolean
	): boolean {
		if (getHosting(plan) === 'SelfHosted') return false;
		const days = getFeatureValue(plan.type, SNAPSHOT_RETENTION);
		if (typeof days !== 'number' || days === 0) return false;
		return !(prevTier && prevTierVisible && getFeatureValue(prevTier, SNAPSHOT_RETENTION) === days);
	}

	// A row or line that some visible card has renders on every card, invisible where it doesn't
	// apply, so the cards' rows line up across the grid.
	let anySeatAddon = $derived(filteredPlans.some((p) => p.seat_cents));
	let anySiteAddon = $derived(filteredPlans.some((p) => p.site_cents));
	let anySelfHosted = $derived(filteredPlans.some((p) => getHosting(p) === 'SelfHosted'));
	let anyEverythingIn = $derived(
		filteredPlans.some((p) => {
			const prev = billingPlanHelpers.getMetadata(p.type)?.previous_tier;
			return !!prev && filteredPlans.some((q) => q.type === prev);
		})
	);

	/** Whether the cards and the comparison list this feature (`listed_on_plans`). */
	function isListed(featureKey: string): boolean {
		return featureHelpers.getMetadata(featureKey)?.listed_on_plans !== false;
	}

	/** The format a diagram export writes; null for any other feature, CSV export included. */
	function exportFormat(featureKey: string): string | null {
		return featureHelpers.getMetadata(featureKey)?.export_format ?? null;
	}

	/** A category's place in display order; unknown ones sort last. */
	function categoryRank(category: string): number {
		const i = categoryOrder.indexOf(category);
		return i === -1 ? Number.MAX_SAFE_INTEGER : i;
	}

	/** The feature this one nests under (`parent` in features.json); null for a top-line feature. */
	function parentOf(featureKey: string): string | null {
		return featureHelpers.getMetadata(featureKey)?.parent ?? null;
	}

	type CardEntry =
		{ type: 'feature'; key: string } | { type: 'group'; key: string; members: string[] };

	/**
	 * One category's features on a card, in order: a feature on its own, or a parent with the
	 * features under it that the card lists, indented. A group sits where its first feature would.
	 */
	function cardEntries(features: string[]): CardEntry[] {
		const entries: CardEntry[] = [];
		const groups: Record<string, Extract<CardEntry, { type: 'group' }>> = {};
		const parents = new Set(features.map(parentOf).filter((p) => p !== null));
		for (const key of features) {
			const parent = parentOf(key) ?? (parents.has(key) ? key : null);
			if (!parent) {
				entries.push({ type: 'feature', key });
				continue;
			}
			if (!groups[parent]) {
				groups[parent] = { type: 'group', key: parent, members: [] };
				entries.push(groups[parent]);
			}
			if (key !== parent) groups[parent].members.push(key);
		}
		return entries;
	}

	/**
	 * A card's features under their category headings, in display order. The integrations sit in
	 * the Discovery group, which a card listing no other Discovery feature still gets.
	 */
	function groupCardFeatures(
		features: string[],
		hasIntegrations: boolean
	): { category: string; features: string[] }[] {
		const groups: { category: string; features: string[] }[] = [];
		for (const key of features) {
			const category = featureHelpers.getCategory(key);
			const last = groups[groups.length - 1];
			if (last?.category === category) last.features.push(key);
			else groups.push({ category, features: [key] });
		}
		if (hasIntegrations && !groups.some((g) => g.category === discoveryCategory)) {
			const at = groups.findIndex(
				(g) => categoryRank(g.category) > categoryRank(discoveryCategory)
			);
			groups.splice(at === -1 ? groups.length : at, 0, {
				category: discoveryCategory,
				features: []
			});
		}
		return groups;
	}

	function sortFeaturesByCategory(features: string[]): string[] {
		const order: string[] = categoryOrder;
		return [...features].sort((a, b) => {
			const catA = order.indexOf(featureHelpers.getCategory(a));
			const catB = order.indexOf(featureHelpers.getCategory(b));
			if (catA !== catB) return (catA === -1 ? 99 : catA) - (catB === -1 ? 99 : catB);
			// Within a category, coming-soon features sort to the end, the rest in declaration order
			const soon = (isComingSoon(a) ? 1 : 0) - (isComingSoon(b) ? 1 : 0);
			return soon !== 0 ? soon : featureOrder.indexOf(a) - featureOrder.indexOf(b);
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

<!-- A card's integrations under the discovery-integrations feature, one row each with its logo. -->
{#snippet integrationRows(integrations: string[])}
	<li class="flex items-start gap-2 text-sm">
		<Check class="mt-0.5 h-4 w-4 shrink-0 text-success" />
		<div class="flex flex-col gap-1">
			<span
				class="explainer self-start text-secondary"
				data-tooltip={featureHelpers.getDescription(DISCOVERY_INTEGRATIONS)}
				use:tooltip>{featureHelpers.getName(DISCOVERY_INTEGRATIONS)}</span
			>
			<ul class="space-y-1">
				{#each integrations as integration (integration)}
					{@const Logo = integrationHelpers.getIconComponent(integration)}
					<li class="flex items-center gap-2 text-xs text-tertiary">
						{#if integrationHelpers.getMetadata(integration)?.logo}
							<!-- A logo draws in its own round frame. -->
							<Logo size={20} />
						{:else}
							<!-- An integration without one gets the same frame around its icon. -->
							<span
								class="inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-full border border-gray-300 dark:border-gray-600"
								style="background: var(--color-topology-node-bg)"
							>
								<Logo class="h-3 w-3 {integrationHelpers.getColorHelper(integration).icon}" />
							</span>
						{/if}
						{integrationHelpers.getName(integration)}
					</li>
				{/each}
			</ul>
		</div>
	</li>
{/snippet}

<!-- A parent feature with the features under it that a card lists, one
     member per row, laid out like the integrations.
     An export reads as its format, any other feature as its name. -->
{#snippet subRows(parent: string, features: string[])}
	<li class="flex items-start gap-2 text-sm">
		<Check class="mt-0.5 h-4 w-4 shrink-0 text-success" />
		<div class="flex flex-col gap-1">
			<span
				class="explainer self-start text-secondary"
				data-tooltip={featureHelpers.getDescription(parent)}
				use:tooltip>{featureHelpers.getName(parent)}</span
			>
			<ul class="space-y-1">
				{#each features as featureKey (featureKey)}
					{@const beta = betaTag(featureKey)}
					<!-- pl-7: the integration logo (20px) and its gap, so the names line up -->
					<li class="flex items-center gap-2 pl-7 text-xs text-tertiary">
						<span
							class="explainer"
							data-tooltip={featureHelpers.getDescription(featureKey)}
							use:tooltip>{exportFormat(featureKey) ?? featureHelpers.getName(featureKey)}</span
						>
						{#if isComingSoon(featureKey)}
							<Tag label={common_comingSoon()} color="Gray" />
						{/if}
						{#if beta}
							<Tag {...beta} />
						{/if}
					</li>
				{/each}
			</ul>
		</div>
	</li>
{/snippet}

<!-- A feature category's heading on a card: its icon and name. -->
{#snippet categoryHeading(category: string)}
	{@const CategoryIcon = productAreaHelpers.getIconComponent(category)}
	<div
		class="flex items-center gap-1.5 text-[10px] font-medium tracking-wider text-tertiary uppercase"
	>
		<CategoryIcon class="h-3 w-3 shrink-0" />
		{productAreaHelpers.getName(category)}
	</div>
{/snippet}

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
					{@const displayFeatures = sortFeaturesByCategory([
						...(prevTierFeatures.length > 0
							? [...new Set([...prevTierFeatures, ...incrementalFeatures])]
							: incrementalFeatures
						).filter(
							(key) => !isItemizedAsIntegrations(key) && isListed(key) && key !== SNAPSHOT_RETENTION
						),
						...(listsSnapshotRetention(plan, prevTier, prevTierVisible) ? [SNAPSHOT_RETENTION] : [])
					])}
					{@const incrementalIntegrations = metadata?.incremental_integrations ?? []}
					{@const displayIntegrations =
						prevTier && !prevTierVisible
							? sortIntegrations([
									...new Set([
										...(billingPlanHelpers.getMetadata(prevTier)?.incremental_integrations ?? []),
										...incrementalIntegrations
									])
								])
							: incrementalIntegrations}

					<div class="plan-card card-static card">
						<!-- Plan Header -->
						<div class="flex flex-col items-center gap-2 pb-4">
							<div class="flex items-center gap-2">
								<IconComponent class="{colorHelper.icon} h-5 w-5 lg:h-6 lg:w-6" />
								<span class="text-base font-semibold text-primary lg:text-lg">
									{billingPlanHelpers.getName(plan.type)}
								</span>
							</div>
						</div>

						<!-- Pricing -->
						<div class="flex flex-col items-center gap-1 pb-4">
							<div class="flex items-baseline gap-1">
								<span class="text-2xl font-bold text-primary lg:text-3xl">
									{formatBasePricing(plan)}
								</span>
								{#if formatRate(plan)}
									<span class="text-sm text-tertiary">{formatRate(plan)}</span>
								{/if}
							</div>
							{#if selfHostedActive}
								<!-- Annual price on the card, monthly equivalent under it. The
								     invisible copy keeps card rows aligned for plans with no annual price. -->
								<div
									class={`text-center text-xs text-tertiary ${isSelfHostedAnnual(plan) ? 'opacity-100' : 'opacity-0'}`}
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

						<!-- Description: always rendered, so every card keeps the same grid rows -->
						<p class="pb-4 text-center text-xs leading-relaxed text-tertiary lg:text-sm">
							{description}
						</p>

						<!-- CTA Button -->
						<div class="py-4" style="border-color: var(--color-border)">
							{#if metadata?.purchase_flow === 'stripe' || metadata?.is_free}
								<!-- Free has purchase_flow 'none' but activates in-app like a Stripe plan -->
								{#if plan.type === currentPlanType && !currentPlanLapsed}
									<!-- Sized like the CTA buttons (border, py-2, text-sm) so every card's header is the same height. -->
									<div
										class="flex w-full items-center justify-center gap-2 rounded-md border border-blue-200 bg-blue-50 px-4 py-2 text-sm font-medium text-info dark:border-blue-600/30 dark:bg-blue-900/20"
									>
										<Info class="h-4 w-4 shrink-0" />
										{billing_yourCurrentPlan()}
									</div>
								{:else}
									<button
										type="button"
										onclick={() => handlePlanSelect(plan)}
										disabled={loadingPlanType !== null}
										class="btn-primary w-full text-sm"
									>
										{#if loadingPlanType === plan.type}
											<Loader2 class="mx-auto h-4 w-4 animate-spin" />
										{:else if plan.type === currentPlanType}
											{billing_continueOnPlan({ plan: billingPlanHelpers.getName(plan.type) })}
										{:else if isCurrentlyTrialing}
											{billing_switchPlan()}
										{:else if !trial}
											{common_getStarted()}
										{:else if getHosting(plan) === 'SelfHosted'}
											{billing_startTrialNoCreditCard()}
										{:else}
											<!-- A cloud trial collects a card first. -->
											{billing_startDayFreeTrial({ days: plan.trial_days })}
										{/if}
									</button>
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

						<!-- Included limits, striped so each value reads against its label. Each label explains its
						     limit and sits level with its value; extras bill at the unit price noted under the count. -->
						<div class="pb-4">
							<div class="-mx-5">
								<!-- Seats -->
								<div
									class="flex items-start justify-between gap-2 px-5 py-1.5 text-sm odd:bg-gray-100 dark:odd:bg-gray-700/25"
								>
									<span
										class="explainer text-secondary"
										data-tooltip={limitHelpers.getDescription('seats')}
										use:tooltip>{common_seats()}</span
									>
									<div class="flex flex-col items-end text-right">
										<span class="font-medium text-primary">
											{formatIncludedLimit(plan.included_seats, plan.seat_cents, plan)}
										</span>
										<!-- Invisible on a plan that sells no extras while another card does, so the rows
										     line up. -->
										{#if anySeatAddon}
											<span
												class={`text-xs text-tertiary ${plan.seat_cents ? 'opacity-100' : 'opacity-0'}`}
												aria-hidden={!plan.seat_cents}>{formatSeatAddonPricing(plan) || ' '}</span
											>
										{/if}
									</div>
								</div>

								<!-- Sites -->
								<div
									class="flex items-start justify-between gap-2 px-5 py-1.5 text-sm odd:bg-gray-100 dark:odd:bg-gray-700/25"
								>
									<span
										class="explainer text-secondary"
										data-tooltip={limitHelpers.getDescription('sites')}
										use:tooltip>{common_sites()}</span
									>
									<div class="flex flex-col items-end text-right">
										<span class="font-medium text-primary">
											{formatIncludedLimit(plan.included_sites, plan.site_cents, plan)}
										</span>
										{#if anySiteAddon}
											<span
												class={`text-xs text-tertiary ${plan.site_cents ? 'opacity-100' : 'opacity-0'}`}
												aria-hidden={!plan.site_cents}>{formatSiteAddonPricing(plan) || ' '}</span
											>
										{/if}
									</div>
								</div>

								<!-- Servers (self-hosted only: how many servers one license key covers).
								     Invisible on a cloud card beside a self-hosted one. -->
								{#if anySelfHosted}
									{@const selfHosted = getHosting(plan) === 'SelfHosted'}
									{@const production = metadata?.included_instances}
									{@const nonProduction = metadata?.included_non_production_instances}
									<div
										class="flex items-start justify-between gap-2 px-5 py-1.5 text-sm odd:bg-gray-100 dark:odd:bg-gray-700/25 {selfHosted
											? ''
											: 'invisible'}"
										aria-hidden={!selfHosted}
									>
										<span
											class="explainer text-secondary"
											data-tooltip={limitHelpers.getDescription('servers')}
											use:tooltip>{common_servers()}</span
										>
										<span class="text-right font-medium text-primary">
											{#if production == null || nonProduction == null}
												{formatIncludedValue(null, plan)}
											{:else}
												{production} +
												<span
													class="explainer"
													data-tooltip={limitHelpers.getDescription('non_production_servers')}
													use:tooltip>{billing_nonProductionCount({ count: nonProduction })}</span
												>
											{/if}
										</span>
									</div>
								{/if}
							</div>
						</div>

						<!-- Incremental Features -->
						<div class="flex-1 py-4">
							{#if anyEverythingIn}
								<!-- Invisible on a card with no visible tier below it, so feature lists start level. -->
								<p
									class="mb-2 text-xs font-medium text-secondary {prevTier && prevTierVisible
										? ''
										: 'invisible'}"
									aria-hidden={!(prevTier && prevTierVisible)}
								>
									{prevTier && prevTierVisible
										? billing_everythingInPlanPlus({
												planName: billingPlanHelpers.getName(prevTier)
											})
										: ' '}
								</p>
							{/if}

							<!-- Mobile: collapsible feature list -->
							{#if displayFeatures.length > 0 || displayIntegrations.length > 0}
								<button
									type="button"
									class="mb-2 flex items-center gap-1 text-xs font-medium text-tertiary sm:hidden"
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

							<!-- One block per category, striped like the limits so each group reads as one. The
							     integrations sit in the Discovery group. -->
							<ul class="-mx-5 {expandedFeatures.has(plan.type) ? '' : 'hidden sm:block'}">
								{#each groupCardFeatures(displayFeatures, displayIntegrations.length > 0) as group (group.category)}
									<li class="space-y-1.5 px-5 py-2 odd:bg-gray-100 dark:odd:bg-gray-700/25">
										{@render categoryHeading(group.category)}
										<ul class="space-y-1.5">
											{#each cardEntries(group.features) as entry (entry.key)}
												{#if entry.type === 'group'}
													{@render subRows(entry.key, entry.members)}
												{:else}
													{@const featureKey = entry.key}
													{@const comingSoon = isComingSoon(featureKey)}
													{@const beta = betaTag(featureKey)}
													<li class="flex items-start gap-2 text-sm">
														<Check
															class="mt-0.5 h-4 w-4 shrink-0 {comingSoon
																? 'text-gray-500'
																: 'text-success'}"
														/>
														<span
															class="explainer {comingSoon ? 'text-tertiary' : 'text-secondary'}"
															data-tooltip={featureHelpers.getDescription(featureKey)}
															use:tooltip>{featureHelpers.getName(featureKey)}</span
														>
														{#if featureKey === SNAPSHOT_RETENTION}
															<span class="text-tertiary">{formatSnapshotRetention(plan.type)}</span
															>
														{/if}
														{#if comingSoon}
															<Tag label={common_comingSoon()} color="Gray" />
														{/if}
														{#if beta}
															<Tag {...beta} />
														{/if}
													</li>
												{/if}
											{/each}
											{#if group.category === discoveryCategory && displayIntegrations.length > 0}
												{@render integrationRows(displayIntegrations)}
											{/if}
										</ul>
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
							<span class="text-xs font-semibold text-primary lg:text-sm"
								>{billingPlanHelpers.getName(plan.type)}</span
							>
						</div>
					{/each}
				</div>

				{#each [...groupedFeatures.entries()] as [category, categoryFeatures] (category)}
					{@const CategoryIcon = productAreaHelpers.getIconComponent(category)}
					<!-- Category Header -->
					<div class="comparison-category-row">
						<span
							class="flex items-center gap-2 p-2 text-xs font-semibold tracking-wide text-secondary uppercase lg:p-3 lg:text-sm"
						>
							<CategoryIcon class="h-4 w-4 shrink-0" />
							{productAreaHelpers.getName(category)}
						</span>
					</div>

					{#each categoryFeatures as featureKey (featureKey)}
						{@const comingSoon = isComingSoon(featureKey)}
						{@const beta = betaTag(featureKey)}
						<div class="comparison-row" style="grid-template-columns: {gridColumns}">
							<div class="comparison-label-cell gap-2">
								<div
									class="explainer text-xs font-medium lg:text-sm"
									data-tooltip={featureHelpers.getDescription(featureKey)}
									use:tooltip
								>
									{featureHelpers.getName(featureKey)}
								</div>
								{#if beta}
									<Tag {...beta} />
								{/if}
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
											<X class="mx-auto h-4 w-4 text-muted lg:h-5 lg:w-5" />
										{/if}
									{:else if typeof value === 'number' && value === 0}
										<X class="mx-auto h-4 w-4 text-muted lg:h-5 lg:w-5" />
									{:else if value === null}
										<span class="text-tertiary">&mdash;</span>
									{:else if featureKey === SNAPSHOT_RETENTION}
										<span class="text-xs text-secondary lg:text-sm"
											>{formatSnapshotRetention(plan.type)}</span
										>
									{:else}
										<span class="text-xs text-secondary lg:text-sm">{value}</span>
									{/if}
								</div>
							{/each}
						</div>
					{/each}

					<!-- The integrations themselves, one row each, after the category holding the
					     discovery-integrations feature they stand in for. -->
					{#if category === discoveryCategory && comparisonIntegrations.length > 0}
						<div class="comparison-category-row">
							<span class="p-2 text-xs font-medium text-tertiary lg:p-3 lg:text-sm">
								{featureHelpers.getName(DISCOVERY_INTEGRATIONS)}
							</span>
						</div>
						{#each comparisonIntegrations as integration (integration)}
							<div class="comparison-row" style="grid-template-columns: {gridColumns}">
								<div class="comparison-label-cell">
									<div class="text-xs font-medium lg:text-sm">
										{integrationHelpers.getName(integration)}
									</div>
								</div>
								{#each filteredPlans as plan (plan.type)}
									<div class="comparison-value-cell">
										{#if planHasIntegration(plan.type, integration)}
											<Check class="mx-auto h-4 w-4 text-success lg:h-5 lg:w-5" />
										{:else}
											<X class="mx-auto h-4 w-4 text-muted lg:h-5 lg:w-5" />
										{/if}
									</div>
								{/each}
							</div>
						{/each}
					{/if}
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

	/* Individual plan card. Its six sections (name, price, description, action, limits, features)
	   share the grid's rows with the cards beside it, so each section starts level across them. */
	.plan-card {
		padding: 1.25rem;
		position: relative;
		display: grid;
		grid-template-rows: subgrid;
		grid-row: span 6;
		row-gap: 0;
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
</style>
