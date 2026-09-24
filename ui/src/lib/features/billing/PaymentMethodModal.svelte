<script lang="ts">
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import StripeCardForm from '$lib/features/billing/StripeCardForm.svelte';
	import InvoiceBillingForm from '$lib/features/billing/InvoiceBillingForm.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import {
		useCheckoutMutation,
		useCreateSetupIntentMutation,
		useFinalizePaymentMethodMutation,
		useWrittenOffInvoice
	} from '$lib/features/billing/queries';
	import type { BillingPlan } from '$lib/features/billing/types';
	import type { components } from '$lib/api/schema';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { isPaidSubscriptionActive } from '$lib/features/organizations/types';
	import { modalState, closeModal, openModal } from '$lib/shared/stores/modal-registry';
	import { reopenSettingsTabAfterPayment } from '$lib/features/billing/stores';
	import { waitForOrgUpdate } from '$lib/shared/billing/wait-for-org-update';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import { pushSuccess } from '$lib/shared/stores/feedback';
	import {
		common_save,
		billing_addPaymentMethod,
		billing_invoice_quoteCreated,
		billing_invoice_sent,
		billing_paymentMethodAdded,
		billing_payByInvoice,
		billing_invoice_termsUnavailable,
		billing_invoice_payOutstanding
	} from '$lib/paraglide/messages';

	type InvoiceBillingMode = components['schemas']['InvoiceBillingMode'];

	// Single global instance opened by any "Add/Update payment method" nudge via
	// openModal('payment-method'). A plan picked by an org with no live
	// subscription rides along in `entityData.plan`.
	let isOpen = $derived($modalState.name === 'payment-method');
	let pendingPlan = $derived(($modalState.entityData?.plan as BillingPlan | undefined) ?? null);

	const setupIntentMutation = useCreateSetupIntentMutation();
	const finalizeMutation = useFinalizePaymentMethodMutation();
	const checkoutMutation = useCheckoutMutation();
	const currentUserQuery = useCurrentUserQuery();
	let userEmail = $derived(currentUserQuery.data?.email);
	const organizationQuery = useOrganizationQuery();
	let org = $derived(organizationQuery.data);

	// Invoice billing is offered only on the self-hosted plans sold in the app.
	let orgPlanLicensed = $derived(
		org?.plan?.type != null && billingPlans.getMetadata(org.plan.type).license_plan != null
	);
	// An org that defaulted on an invoice we wrote off has lost payment terms
	// until it settles. The server refuses the request either way; withholding
	// the link here is what stops the buyer walking into that refusal.
	const writtenOffInvoice = useWrittenOffInvoice();
	let planAllowsInvoice = $derived(
		(pendingPlan?.type != null &&
			billingPlans.getMetadata(pendingPlan.type).license_plan != null) ||
			orgPlanLicensed
	);
	let invoiceEligible = $derived(planAllowsInvoice && writtenOffInvoice.current == null);
	// Nothing names the plan to invoice for: the form asks. Covers a reload
	// mid-flow, which rebuilds modal state from the URL without the plan.
	let needsPlanChoice = $derived(pendingPlan == null && !orgPlanLicensed);

	// Card and bank are Stripe's own tabs inside the Payment Element, which is up
	// as soon as the dialog opens. Invoice is a text link under it.
	type Method = 'card' | 'invoice';
	let method = $state<Method>('card');
	let clientSecret = $state<string | null>(null);

	// Opened from a tab inside Settings, this modal replaced Settings in the
	// registry while Settings stayed on screen (a locked org's Settings can't
	// close). Naming it again puts the registry and the URL back on what is
	// visible, whichever way this dialog ends.
	function closeAndReturn() {
		const tab = $reopenSettingsTabAfterPayment;
		closeModal();
		if (tab) {
			reopenSettingsTabAfterPayment.set(null);
			openModal('settings', { tab });
		}
	}

	function handleOpen() {
		method = 'card';
		void loadCardForm();
	}

	async function loadCardForm() {
		if (clientSecret != null) return;
		try {
			clientSecret = await setupIntentMutation.mutateAsync();
		} catch {
			// setup-intent error is toasted by the mutation; close the empty dialog
			closeAndReturn();
		}
	}

	async function handleCardSuccess(setupIntentId: string) {
		// Read before the await: the registry can move on while the card is
		// finalizing, and the plan this dialog was opened for must not go with it.
		const plan = pendingPlan;
		await finalizeMutation.mutateAsync(setupIntentId);

		// Buying a plan: the card is now on file, so the backend creates the
		// subscription in place. It only returns a URL when Stripe still needs
		// the customer (3D Secure), and then we follow it.
		if (plan) {
			closeAndReturn();
			try {
				const result = await checkoutMutation.mutateAsync(plan);
				if (result.startsWith('http')) {
					window.location.href = result;
					return;
				}
				// The subscription lands on the org when the webhook does; poll for
				// the picked plan so the banner and Billing tab converge without a
				// reload.
				await waitForOrgUpdate((o) => isPaidSubscriptionActive(o) && o.plan?.type === plan.type, {
					intervalMs: 500
				});
			} catch {
				// The mutation toasts the failure.
			}
			return;
		}

		closeAndReturn();
		// Converge once the webhook/finalize records the new payment method, then
		// confirm to the user (mirrors the other billing flows' success cadence).
		await waitForOrgUpdate((o) => o.has_payment_method ?? false);
		pushSuccess(billing_paymentMethodAdded());
	}

	function handleInvoiceDone(mode: InvoiceBillingMode) {
		// The quote lands on the Billing tab, where it is downloaded, accepted
		// or withdrawn, so send the user there rather than back where they
		// started. Opened from the Billing tab the stored tab is already null,
		// which would otherwise close to nothing.
		if (mode === 'quote') reopenSettingsTabAfterPayment.set('billing');
		closeAndReturn();
		pushSuccess(mode === 'quote' ? billing_invoice_quoteCreated() : billing_invoice_sent());
	}
</script>

<GenericModal
	{isOpen}
	name="payment-method"
	title={billing_addPaymentMethod()}
	size="md"
	compactPadding={true}
	showCloseButton={true}
	onClose={closeAndReturn}
	onOpen={handleOpen}
>
	{#if method === 'invoice'}
		<InvoiceBillingForm
			plan={pendingPlan}
			{needsPlanChoice}
			orgName={org?.name}
			email={userEmail}
			isTrialing={org?.plan_status === 'trialing'}
			onDone={handleInvoiceDone}
			onCancel={closeAndReturn}
			onPayByCard={() => (method = 'card')}
		/>
	{:else if clientSecret}
		{#if planAllowsInvoice && writtenOffInvoice.current}
			<div class="px-6 pt-4">
				<InlineInfo
					title={billing_invoice_termsUnavailable({
						number: writtenOffInvoice.current.number ?? ''
					})}
				/>
				{#if writtenOffInvoice.current.hosted_invoice_url}
					<!-- eslint-disable svelte/no-navigation-without-resolve -->
					<a
						href={writtenOffInvoice.current.hosted_invoice_url}
						target="_blank"
						rel="noopener noreferrer"
						class="text-info mt-2 inline-block text-sm hover:underline"
					>
						{billing_invoice_payOutstanding()}
					</a>
					<!-- eslint-enable svelte/no-navigation-without-resolve -->
				{/if}
			</div>
		{/if}
		<StripeCardForm
			{clientSecret}
			email={userEmail}
			submitLabel={common_save()}
			onSuccess={handleCardSuccess}
			onCancel={closeAndReturn}
			altAction={invoiceEligible
				? { label: billing_payByInvoice(), onclick: () => (method = 'invoice') }
				: null}
		/>
	{:else}
		<div class="flex min-h-[12rem] items-center justify-center p-6">
			<Loading />
		</div>
	{/if}
</GenericModal>
