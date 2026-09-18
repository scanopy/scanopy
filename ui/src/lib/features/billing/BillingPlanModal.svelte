<script lang="ts">
	import billingPlansJson from '$lib/data/billing-plans.json';
	import featuresJson from '$lib/data/features.json';
	import BillingPlanForm from '$lib/features/billing/BillingPlanForm.svelte';
	import type { BillingPlan, PlanPickerHosting } from '$lib/features/billing/types';
	import {
		createStaticHelpers,
		type BillingPlanMetadata,
		type FeatureMetadata
	} from '$lib/shared/stores/metadata';
	import { useCheckoutMutation } from '$lib/features/billing/queries';
	import { onboardingStore } from '$lib/features/auth/stores/onboarding';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { waitForOrgUpdate } from '$lib/shared/billing/wait-for-org-update';
	import { hasLicensedPlan, isBillingPlanActive } from '$lib/features/organizations/types';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import { upgradeContext } from '$lib/features/billing/stores';
	import { isLicenseSigningAvailable, useConfigQuery } from '$lib/shared/stores/config-query';
	import { openModal } from '$lib/shared/stores/modal-registry';
	import { canPay } from '$lib/shared/utils/trial';

	let {
		isOpen = false,
		dismissible = true,
		onClose,
		name = undefined
	}: {
		isOpen?: boolean;
		dismissible?: boolean;
		/** Receives the plan the user just picked, so the caller can react to it without
		 *  waiting for the organization query to catch up. Undefined on a plain dismiss. */
		onClose: (selectedPlan?: BillingPlan) => void;
		name?: string;
	} = $props();

	// Create helpers from static fixtures (no API calls needed)
	const billingPlanHelpers = createStaticHelpers<BillingPlanMetadata>(
		'billing_plans',
		billingPlansJson
	);
	const featureHelpers = createStaticHelpers<FeatureMetadata>('features', featuresJson);

	// Transform fixture data to BillingPlan[] format (exclude plans that can't be obtained
	// in-app, deduplicate). purchase_flow 'none' is Community (GitHub) and Free; Free
	// stays because it activates in-app. Enterprise is sold through the website, not here.
	const plansData = (() => {
		const seen = new Set<string>(); // eslint-disable-line svelte/prefer-svelte-reactivity
		return billingPlansJson
			.filter((p) => p.metadata.purchase_flow !== 'none' || p.metadata.is_free)
			.filter((p) => !(p.metadata.is_free && p.metadata.rate === 'Year'))
			.filter((p) => !p.metadata.is_enterprise)
			.map(
				(p) =>
					({
						type: p.id,
						base_cents: p.metadata.base_cents,
						rate: p.metadata.rate,
						trial_days: p.metadata.trial_days,
						seat_cents: p.metadata.seat_cents,
						network_cents: p.metadata.network_cents,
						included_seats: p.metadata.included_seats,
						included_networks: p.metadata.included_networks,
						// Checkout validates the full plan config; self-hosted plans carry an org cap.
						included_orgs: p.metadata.included_orgs ?? null,
						host_cents: p.metadata.host_cents ?? null,
						included_hosts: p.metadata.included_hosts ?? null
					}) as BillingPlan
			)
			.filter((p) => {
				const key = `${p.type}-${p.rate}`;
				if (seen.has(key)) return false;
				seen.add(key);
				return true;
			});
	})();

	// TanStack Query for organization
	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	const configQuery = useConfigQuery();
	let signingAvailable = $derived(
		configQuery.data != null && isLicenseSigningAvailable(configQuery.data)
	);
	// A server with no signing key cannot mint a license, so its self-hosted tiers
	// are unbuyable. Dropping them empties the Self-Hosted tab, which is why the
	// hosting toggle goes with them.
	let pickablePlans = $derived(
		signingAvailable
			? plansData
			: plansData.filter((p) => billingPlanHelpers.getMetadata(p.type)?.license_plan == null)
	);

	let isCurrentlyTrialing = $derived(organization?.plan_status === 'trialing');

	// Only show trial offers to orgs that have never had a non-Free paid plan and never trialed.
	// trial_end_date is set by Stripe webhook only for subscriptions with trial periods
	// (Free plan has trial_days=0, so it never sets trial_end_date).
	// Trialing users are NOT returning — they should see trial-aware UI instead.
	let isReturningCustomer = $derived(
		!isCurrentlyTrialing &&
			((organization?.plan != null &&
				billingPlanHelpers.getMetadata(organization.plan.type)?.is_free !== true) ||
				!!organization?.trial_end_date)
	);

	// Mutations
	const checkoutMutation = useCheckoutMutation();

	// Determine initial filter based on use case from onboarding
	let useCase = $derived($onboardingStore.useCase);

	// Open on Self-Hosted for orgs already on a licensed plan, else the tab requested at
	// signup (`?hosting=self_hosted`), else Cloud.
	let initialHosting = $derived<PlanPickerHosting>(
		!signingAvailable
			? 'cloud'
			: organization && hasLicensedPlan(organization)
				? 'self_hosted'
				: ($onboardingStore.hosting ?? 'cloud')
	);

	// Recommended plan based on use case
	let baseRecommendedPlan = $derived<string | null>(
		useCase === 'internal_it' ? 'Team' : useCase === 'msp' ? 'Business' : null
	);

	// Feature-contextual plan highlighting from upgrade CTAs
	let upgradeCtx = $derived($upgradeContext);

	let contextHighlightPlan = $derived.by(() => {
		if (!upgradeCtx) return null;
		const feat = upgradeCtx.feature;
		// Feature-based: look up minimum_plan from feature metadata
		const featureMeta = featureHelpers.getMetadata(feat);
		if (featureMeta?.minimum_plan) return featureMeta.minimum_plan;
		// Resource-based: find first plan with addon pricing
		if (feat === 'seats') return plansData.find((p) => p.seat_cents)?.type ?? null;
		if (feat === 'networks') return plansData.find((p) => p.network_cents)?.type ?? null;
		if (feat === 'hosts') return plansData.find((p) => p.host_cents)?.type ?? null;
		return null;
	});

	let recommendedPlan = $derived(contextHighlightPlan ?? baseRecommendedPlan);

	async function handlePlanSelect(plan: BillingPlan) {
		// A self-hosted plan bought with no trial left and no way to pay on file would
		// go to Stripe Checkout, which takes cards only. The payment-method dialog
		// offers invoice billing beside the card, and continues to Checkout for a card.
		if (
			billingPlanHelpers.getMetadata(plan.type)?.license_plan != null &&
			isReturningCustomer &&
			!canPay(organization)
		) {
			upgradeContext.set(null);
			// Closed without the plan: nothing is bought yet, so the page must not lock
			// onto the License tab over the dialog. The lock follows the webhook.
			onClose();
			openModal('payment-method', { entityData: { plan } });
			return;
		}

		// Only an immediate-payment selection (paid plan, no trial, no card on file)
		// redirects to Stripe Checkout; trial signups / Free / plan changes activate
		// in-app via a plain API call. Pre-open the tab synchronously (inside the
		// click, so popup blockers allow it) only when a redirect is expected — so we
		// don't flash a blank tab for the in-app cases. A misprediction (e.g. a
		// returning customer who already used their trial) falls back to a same-tab
		// redirect below. (No 'noopener' — that makes window.open return null.)
		// Poll until the selected plan lands, not just any active plan: a switch between
		// two active plans (e.g. self-hosted ↔ cloud) would otherwise stop on the old one.
		const planApplied = (org: Parameters<typeof isBillingPlanActive>[0]) =>
			isBillingPlanActive(org) && org.plan?.type === plan.type;
		const expectsStripeCheckout =
			plan.base_cents > 0 && plan.trial_days === 0 && !canPay(organization);
		const stripeTab = expectsStripeCheckout ? window.open('', '_blank') : null;
		try {
			// New tab — this tab stays put, so track immediately rather than stashing
			// the event for a post-redirect flush.
			const metadata = billingPlanHelpers.getMetadata(plan.type);
			trackEvent('plan_selected', {
				plan: plan.type,
				is_commercial: metadata?.is_commercial ?? false
			});

			// Backend decides: new subscriber → checkout URL, existing → plan change message
			const result = await checkoutMutation.mutateAsync(plan);
			if (result?.startsWith('http')) {
				// First-time checkout: open Stripe in a new tab and close the modal. This
				// tab converges once the checkout webhook activates the plan.
				if (stripeTab) {
					stripeTab.location.href = result;
					upgradeContext.set(null);
					onClose(plan);
					// 500 ms steps: the Settings modal opens on the picked plan's intent flag
					// and clears it when the org confirms, so the poll is what ends the
					// provisional state. The default 2000 ms leaves it standing too long.
					void waitForOrgUpdate(planApplied, { intervalMs: 500 });
				} else {
					// No pre-opened tab (redirect not anticipated, or popup blocked) —
					// fall back to a same-tab redirect.
					window.location.href = result;
				}
			} else {
				// Direct activation needs no Stripe tab.
				stripeTab?.close();
				upgradeContext.set(null);
				onClose(plan);
				// Plan activated directly (Free or trial) is still webhook-driven, so a
				// single refetch races the webhook and reads stale state (e.g. plan_status
				// still null, so NoPaymentMethodBanner never appears until a reload). Poll
				// like the Stripe-redirect branch until the org reflects the activation.
				// Closing first is safe: onClose sets planJustActivated, suppressing reopen.
				// 500 ms steps: the Settings modal opens on the picked plan's intent flag
				// and clears it when the org confirms, so the poll is what ends the
				// provisional state. The default 2000 ms leaves it standing too long.
				void waitForOrgUpdate(planApplied, { intervalMs: 500 });
			}
		} catch {
			// Error handled by mutation
			stripeTab?.close();
		}
	}
</script>

<GenericModal
	{isOpen}
	title=""
	{name}
	onClose={dismissible
		? () => {
				upgradeContext.set(null);
				onClose();
			}
		: null}
	size="max"
	preventCloseOnClickOutside={!dismissible}
	showCloseButton={false}
	floatingCloseButton={dismissible}
	borderless={true}
	compactPadding={true}
>
	<div class="flex min-h-0 flex-1 flex-col">
		<BillingPlanForm
			plans={billingPlanHelpers.getMetadata(organization?.plan?.type ?? null)?.is_free
				? pickablePlans
				: pickablePlans.filter((p) => billingPlanHelpers.getMetadata(p.type)?.is_free !== true)}
			{billingPlanHelpers}
			{featureHelpers}
			showHosting={signingAvailable}
			{initialHosting}
			onPlanSelect={handlePlanSelect}
			{recommendedPlan}
			{isReturningCustomer}
			{isCurrentlyTrialing}
			currentPlanType={organization?.plan?.type ?? null}
		/>
	</div>
</GenericModal>
