<script lang="ts">
	import { usePreloadStripe } from '#lib/features/billing/stripe.svelte.ts';
	import billingPlansJson from '#lib/data/billing-plans.json';
	import featuresJson from '#lib/data/features.json';
	import productAreasJson from '#lib/data/product-areas.json';
	import credentialIntegrationsJson from '#lib/data/credential-integrations.json';
	import planLimitsJson from '#lib/data/plan-limits.json';
	import BillingPlanForm from '#lib/features/billing/BillingPlanForm.svelte';
	import type { BillingPlan, PlanPickerHosting } from '#lib/features/billing/types.ts';
	import {
		createStaticHelpers,
		type BillingPlanMetadata,
		type CredentialIntegrationMetadata,
		type FeatureMetadata
	} from '#lib/shared/stores/metadata.ts';
	import { useCheckoutMutation } from '#lib/features/billing/queries.ts';
	import { onboardingStore } from '#lib/features/auth/stores/onboarding.ts';
	import { useOrganizationQuery } from '#lib/features/organizations/queries.ts';
	import { trackEvent } from '#lib/shared/utils/analytics.ts';
	import { waitForOrgUpdate } from '#lib/shared/billing/wait-for-org-update.ts';
	import type { PaymentFormSource } from '#lib/shared/billing/setup-payment.ts';
	import { isPlanLapsed, isBillingPlanActive } from '#lib/features/organizations/types.ts';
	import GenericModal from '#lib/shared/components/layout/GenericModal.svelte';
	import { isLicenseSigningAvailable, useConfigQuery } from '#lib/shared/stores/config-query.ts';
	import { modalState } from '#lib/shared/stores/modal-registry.ts';
	import { billingOverModal, openBillingModal } from '#lib/features/billing/stores.ts';
	import {
		canPay,
		isInTrial,
		isReturningCustomer as isReturningCustomerOrg
	} from '#lib/shared/utils/trial.ts';

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
	const productAreaHelpers = createStaticHelpers<object>('product_areas', productAreasJson);
	const integrationHelpers = createStaticHelpers<CredentialIntegrationMetadata>(
		'credential_integrations',
		credentialIntegrationsJson
	);
	const limitHelpers = createStaticHelpers<object>('plan_limits', planLimitsJson);

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
						site_cents: p.metadata.site_cents,
						included_seats: p.metadata.included_seats,
						included_sites: p.metadata.included_sites,
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

	// A trial with a cancellation scheduled still runs, and a switch keeps it.
	let isCurrentlyTrialing = $derived(isInTrial(organization));

	// Only orgs that have never had a paid subscription or a trial see trial offers.
	let isReturningCustomer = $derived(isReturningCustomerOrg(organization));
	// The plan the org holds, unless no subscription ever started on it: then
	// its card offers the trial instead of saying "Your current plan". Free has
	// no subscription and is still the current plan.
	let currentPlanType = $derived(
		organization == null ||
			(organization.plan_status == null &&
				billingPlanHelpers.getMetadata(organization.plan.type)?.is_stripe_managed === true)
			? null
			: organization.plan.type
	);

	// Mutations
	const checkoutMutation = useCheckoutMutation();

	// An org on a Stripe-managed plan (live or lapsed) opens on that plan's
	// hosting. A lapsed org keeps its plan, so a licensed one lands on
	// Self-Hosted from that alone. Otherwise the tab requested at signup
	// (`?hosting=self_hosted`), else Cloud.
	let planHosting = $derived.by((): PlanPickerHosting | null => {
		const meta = billingPlanHelpers.getMetadata(organization?.plan?.type ?? null);
		if (meta?.is_stripe_managed !== true) return null;
		return meta.hosting === 'SelfHosted'
			? 'self_hosted'
			: meta.hosting === 'Cloud'
				? 'cloud'
				: null;
	});
	// An opener can ask for a tab (Settings → License asks for Cloud), which wins
	// over the current plan's.
	let requestedHosting = $derived(
		(($billingOverModal?.modal === 'billing-plan'
			? $billingOverModal.entityData?.hosting
			: $modalState.name === name
				? $modalState.entityData?.hosting
				: undefined) as PlanPickerHosting | undefined) ?? null
	);
	let initialHosting = $derived<PlanPickerHosting>(
		!signingAvailable
			? 'cloud'
			: (requestedHosting ?? planHosting ?? $onboardingStore.hosting ?? 'cloud')
	);

	async function handlePlanSelect(plan: BillingPlan) {
		// A paid plan with no trial left and no way to pay on file would otherwise
		// go to Stripe Checkout. The payment-method dialog collects the card here
		// instead (and offers invoice billing on self-hosted plans), then the
		// backend creates the subscription. Cloud and self-hosted behave alike.
		// Every cloud trial needs the card first too, whether it starts here or an
		// org already trialing (a license trial needs none) switches onto a cloud
		// plan; the switch keeps the trial running.
		const cloudPlan = billingPlanHelpers.getMetadata(plan.type)?.license_plan == null;
		const needsCardFirst = isCurrentlyTrialing
			? cloudPlan
			: isReturningCustomer || (cloudPlan && plan.trial_days > 0);
		if (plan.base_cents > 0 && needsCardFirst && !canPay(organization)) {
			// Closed without the plan: nothing is bought yet, so the page must not lock
			// onto the License tab over the dialog. The lock follows the webhook.
			onClose();
			openBillingModal('payment-method', {
				plan,
				source: 'plan_picker' satisfies PaymentFormSource
			});
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
			// The signup's hosting choice has opened the picker on its tab; from here
			// the org's own plan does that.
			onboardingStore.clearHosting();
			if (result?.startsWith('http')) {
				// First-time checkout: open Stripe in a new tab and close the modal. This
				// tab converges once the checkout webhook activates the plan.
				if (stripeTab) {
					stripeTab.location.href = result;
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

	// Stripe.js is ready before anyone opens the payment dialog from here.
	usePreloadStripe(() => isOpen);
</script>

<!-- The plan picker shows no banners of any kind. -->
<GenericModal
	{isOpen}
	title=""
	{name}
	noBanners
	onClose={dismissible ? () => onClose() : null}
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
			{productAreaHelpers}
			{integrationHelpers}
			{limitHelpers}
			showHosting={signingAvailable}
			{initialHosting}
			onPlanSelect={handlePlanSelect}
			{isReturningCustomer}
			{isCurrentlyTrialing}
			{currentPlanType}
			currentPlanLapsed={organization != null && isPlanLapsed(organization)}
		/>
	</div>
</GenericModal>
