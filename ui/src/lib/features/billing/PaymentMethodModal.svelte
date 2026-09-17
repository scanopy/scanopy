<script lang="ts">
	import { CreditCard, FileText } from 'lucide-svelte';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import StripeCardForm from '$lib/features/billing/StripeCardForm.svelte';
	import InvoiceBillingForm from '$lib/features/billing/InvoiceBillingForm.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import {
		useCheckoutMutation,
		useCreateSetupIntentMutation,
		useFinalizePaymentMethodMutation
	} from '$lib/features/billing/queries';
	import type { BillingPlan } from '$lib/features/billing/types';
	import type { components } from '$lib/api/schema';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
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
		billing_paymentOptionCard,
		billing_paymentOptionCardDescription,
		billing_paymentOptionInvoiceDescription,
		common_invoice
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
	let invoiceEligible = $derived.by(() => {
		const type = pendingPlan?.type ?? org?.plan?.type ?? null;
		return type != null && billingPlans.getMetadata(type).license_plan != null;
	});

	type View = 'choose' | 'card' | 'invoice';
	let view = $state<View>('card');
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
		clientSecret = null;
		if (invoiceEligible) {
			view = 'choose';
		} else {
			void chooseCard();
		}
	}

	async function chooseCard() {
		// An org with no live subscription buys the plan through Stripe Checkout,
		// which collects the card itself.
		if (pendingPlan) {
			const plan = pendingPlan;
			closeModal();
			try {
				const result = await checkoutMutation.mutateAsync(plan);
				if (result.startsWith('http')) window.location.href = result;
			} catch {
				// The mutation toasts the failure.
			}
			return;
		}
		view = 'card';
		clientSecret = null;
		try {
			clientSecret = await setupIntentMutation.mutateAsync();
		} catch {
			// setup-intent error is toasted by the mutation; close the empty dialog
			closeAndReturn();
		}
	}

	async function handleCardSuccess(setupIntentId: string) {
		await finalizeMutation.mutateAsync(setupIntentId);
		closeAndReturn();
		// Converge once the webhook/finalize records the new payment method, then
		// confirm to the user (mirrors the other billing flows' success cadence).
		await waitForOrgUpdate((o) => o.has_payment_method ?? false);
		pushSuccess(billing_paymentMethodAdded());
	}

	function handleInvoiceDone(mode: InvoiceBillingMode) {
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
	{#if view === 'choose'}
		<div class="space-y-3 p-6">
			<button
				type="button"
				class="card flex w-full items-center gap-4 p-4 text-left transition-all hover:bg-gray-100 dark:hover:bg-gray-800"
				onclick={chooseCard}
			>
				<CreditCard class="text-secondary h-5 w-5 flex-shrink-0" />
				<div>
					<div class="text-primary font-medium">{billing_paymentOptionCard()}</div>
					<div class="text-secondary text-sm">{billing_paymentOptionCardDescription()}</div>
				</div>
			</button>
			<button
				type="button"
				class="card flex w-full items-center gap-4 p-4 text-left transition-all hover:bg-gray-100 dark:hover:bg-gray-800"
				onclick={() => (view = 'invoice')}
			>
				<FileText class="text-secondary h-5 w-5 flex-shrink-0" />
				<div>
					<div class="text-primary font-medium">{common_invoice()}</div>
					<div class="text-secondary text-sm">{billing_paymentOptionInvoiceDescription()}</div>
				</div>
			</button>
		</div>
	{:else if view === 'invoice'}
		<InvoiceBillingForm
			plan={pendingPlan}
			email={userEmail}
			isTrialing={org?.plan_status === 'trialing'}
			onDone={handleInvoiceDone}
			onBack={() => (view = 'choose')}
		/>
	{:else if clientSecret}
		<StripeCardForm
			{clientSecret}
			email={userEmail}
			submitLabel={common_save()}
			onSuccess={handleCardSuccess}
			onCancel={closeAndReturn}
		/>
	{:else}
		<div class="flex min-h-[12rem] items-center justify-center p-6">
			<Loading />
		</div>
	{/if}
</GenericModal>
