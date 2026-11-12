<script lang="ts">
	import GenericModal from '#lib/shared/components/layout/GenericModal.svelte';
	import StripeCardForm from '#lib/features/billing/StripeCardForm.svelte';
	import InvoiceBillingForm from '#lib/features/billing/InvoiceBillingForm.svelte';
	import Loading from '#lib/shared/components/feedback/Loading.svelte';
	import InlineInfo from '#lib/shared/components/feedback/InlineInfo.svelte';
	import { formatLongDate } from '#lib/shared/utils/formatting.ts';
	import { trialChargeDate } from '#lib/shared/utils/trial.ts';
	import { useDelayedFlag } from '#lib/shared/utils/delayed-flag.svelte.ts';
	import {
		clearSetupIntent,
		paymentDialogPreload,
		takeSetupIntent
	} from '#lib/features/billing/stripe.svelte.ts';
	import {
		useCheckoutMutation,
		useCreateSetupIntentMutation,
		useFinalizePaymentMethodMutation,
		useWrittenOffInvoice
	} from '#lib/features/billing/queries.ts';
	import type { BillingPlan } from '#lib/features/billing/types.ts';
	import type { components } from '#lib/api/schema';
	import { useCurrentUserQuery } from '#lib/features/auth/queries.ts';
	import { useOrganizationQuery } from '#lib/features/organizations/queries.ts';
	import { isPaidSubscriptionActive, isPlanLapsed } from '#lib/features/organizations/types.ts';
	import { modalState, closeModal, openModal } from '#lib/shared/stores/modal-registry.ts';
	import { billingOverModal, reopenSettingsTabAfterPayment } from '#lib/features/billing/stores.ts';
	import { waitForOrgUpdate } from '#lib/shared/billing/wait-for-org-update.ts';
	import { billingPlans } from '#lib/shared/stores/metadata.ts';
	import { pushSuccess, pushWarning } from '#lib/shared/stores/feedback.ts';
	import {
		apiFailure,
		trackPaymentFormFailed,
		type PaymentFormFailure,
		type PaymentFormSource
	} from '#lib/shared/billing/setup-payment.ts';
	import {
		common_save,
		billing_addPaymentMethod,
		billing_invoice_quoteCreated,
		billing_invoice_sent,
		billing_paymentMethodAdded,
		billing_paymentMethodFinalizeFailed,
		billing_payByInvoice,
		billing_invoice_termsUnavailable,
		billing_invoice_termsUnavailableSettled,
		billing_notChargedUntilTrialEnds,
		billing_invoice_payOutstanding
	} from '#lib/paraglide/messages.js';

	type InvoiceBillingMode = components['schemas']['InvoiceBillingMode'];

	// Single global instance opened by any "Add/Update payment method" nudge via
	// openModal('payment-method'). A plan picked by an org with no live
	// subscription rides along in `entityData.plan`, and the surface that opened
	// it in `entityData.source` (absent after a reload mid-flow).
	// Opened over a form (see `billingOverModal`), it takes no registry name and reads its plan from
	// that store, so the form underneath stays mounted.
	let overModal = $derived($billingOverModal?.modal === 'payment-method');
	let isOpen = $derived(overModal || $modalState.name === 'payment-method');
	let entityData = $derived(overModal ? $billingOverModal?.entityData : $modalState.entityData);
	let pendingPlan = $derived((entityData?.plan as BillingPlan | undefined) ?? null);
	let source = $derived((entityData?.source as PaymentFormSource | undefined) ?? null);

	const setupIntentMutation = useCreateSetupIntentMutation();
	const finalizeMutation = useFinalizePaymentMethodMutation();
	const checkoutMutation = useCheckoutMutation();
	const currentUserQuery = useCurrentUserQuery();
	let userEmail = $derived(currentUserQuery.data?.email);
	const organizationQuery = useOrganizationQuery();
	let org = $derived(organizationQuery.data);

	// The plan this card is for: the one being bought, or else the one the org
	// is on (a banner asking to pay for the current trial or subscription).
	// Every plan-dependent choice below reads it. `pendingPlan` alone means
	// "buy this after saving the card".
	let cardPlan = $derived(pendingPlan ?? org?.plan ?? null);
	// Invoice billing is offered only on the self-hosted plans sold in the app.
	let cardPlanLicensed = $derived(
		cardPlan != null && billingPlans.getMetadata(cardPlan.type).license_plan != null
	);
	// The card starts or continues a trial, so nothing is charged until it ends:
	// a picked plan with a trial the org can still take, or an org already
	// trialing (a switch keeps the trial).
	let trialEndsAt = $derived(trialChargeDate(org, pendingPlan));
	// An org that defaulted on an invoice we wrote off has lost payment terms
	// until it settles. The server refuses the request either way; withholding
	// the link here is what stops the buyer walking into that refusal.
	const writtenOffInvoice = useWrittenOffInvoice();
	// A lapsed org can settle and get everything back. One that has already
	// re-subscribed cannot: the resume path declines once a subscription
	// exists, so paying here would buy nothing. It still cannot take terms,
	// which is what the note has to explain without offering a payment.
	let orgIsLapsed = $derived(org != null && isPlanLapsed(org));
	// Decided by the plan the card is for, so an org on a license trial adding a
	// card for a cloud plan isn't offered terms the cloud plan doesn't take.
	let planAllowsInvoice = $derived(cardPlanLicensed);
	let invoiceEligible = $derived(planAllowsInvoice && writtenOffInvoice.current == null);
	// Settling is the one route back for a lapsed org, so it takes the centred
	// slot under the element where "Pay by invoice" sits for everyone else.
	// Never both: an org with a written-off invoice cannot take terms.
	let payOutstandingUrl = $derived(
		orgIsLapsed && planAllowsInvoice
			? (writtenOffInvoice.current?.hosted_invoice_url ?? null)
			: null
	);
	// Nothing names the plan to invoice for: the form asks. Covers a reload
	// mid-flow, which rebuilds modal state from the URL without the plan.
	let needsPlanChoice = $derived(pendingPlan == null && !cardPlanLicensed);

	// Card and bank are Stripe's own tabs inside the Payment Element, which is up
	// as soon as the dialog opens. Invoice is a text link under it.
	type Method = 'card' | 'invoice';
	let method = $state<Method>('card');
	// The intent this open is bound to: one fetched on open, or the prefetched
	// one pinned when its card is saved, so the form keeps it while finalize
	// runs and the next intent is prefetched.
	let pinnedSecret = $state<string | null>(null);
	// While a surface that opens this is showing, the form mounts on the
	// prefetched intent behind the closed dialog, so opening shows it drawn.
	let clientSecret = $derived(pinnedSecret ?? paymentDialogPreload.clientSecretFor(org?.id));
	// The intent is usually prefetched, so a wheel only shows for a slow fetch.
	const showSpinner = useDelayedFlag(() => isOpen && clientSecret == null, 400);

	// Opened from a tab inside Settings, this modal replaced Settings in the
	// registry while Settings stayed on screen (a locked org's Settings can't
	// close). Naming it again puts the registry and the URL back on what is
	// visible, whichever way this dialog ends.
	function closeAndReturn() {
		// An unconfirmed intent can back the next open. A saved card has already
		// spent it (clearSetupIntent), and the next one is prefetched.
		pinnedSecret = null;
		method = 'card';
		if (overModal) {
			billingOverModal.set(null);
			return;
		}
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
			// Usually made already, by the surface that opened this (usePreloadStripe).
			pinnedSecret =
				org != null ? await takeSetupIntent(org.id) : await setupIntentMutation.mutateAsync();
		} catch (err) {
			trackPaymentFormFailed('setup_intent', source, apiFailure(err));
			// setup-intent error is toasted by the mutation; close the empty dialog
			closeAndReturn();
		}
	}

	function handleConfirmFailed(failure: PaymentFormFailure) {
		trackPaymentFormFailed('confirm', source, failure);
	}

	async function handleCardSuccess(setupIntentId: string) {
		// Read before the await: the registry can move on while the card is
		// finalizing, and the plan this dialog was opened for must not go with it.
		const plan = pendingPlan;
		// Stripe has confirmed this intent, so it can't back another form. This
		// form keeps it, so a finalize retry confirms nothing twice.
		pinnedSecret = clientSecret;
		clearSetupIntent();
		try {
			await finalizeMutation.mutateAsync(setupIntentId);
		} catch (err) {
			// The mutation toasts the cause. Stripe already holds the card, so
			// rethrow: the form stays open and its button retries finalize for
			// this same SetupIntent.
			trackPaymentFormFailed('finalize', source, apiFailure(err));
			pushWarning(billing_paymentMethodFinalizeFailed());
			throw err;
		}

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
		// The finalize mutation has marked the cached org as having a card, so
		// the banners clear now. No refetch here: until the webhook lands it would
		// read the old value back.
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
	keepMounted={paymentDialogPreload.warm}
	name={overModal ? undefined : 'payment-method'}
	title={billing_addPaymentMethod()}
	size="xl"
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
					title={orgIsLapsed
						? billing_invoice_termsUnavailable({
								number: writtenOffInvoice.current.number ?? ''
							})
						: billing_invoice_termsUnavailableSettled()}
				/>
			</div>
		{/if}
		{#key clientSecret}
			<StripeCardForm
				{clientSecret}
				active={isOpen}
				notice={trialEndsAt
					? billing_notChargedUntilTrialEnds({ date: formatLongDate(trialEndsAt) })
					: undefined}
				email={userEmail}
				submitLabel={common_save()}
				onSuccess={handleCardSuccess}
				onConfirmFailed={handleConfirmFailed}
				onCancel={closeAndReturn}
				altAction={payOutstandingUrl
					? {
							label: billing_invoice_payOutstanding(),
							onclick: () => window.open(payOutstandingUrl, '_blank', 'noopener,noreferrer')
						}
					: invoiceEligible
						? { label: billing_payByInvoice(), onclick: () => (method = 'invoice') }
						: null}
			/>
		{/key}
	{:else}
		<div class="flex min-h-[12rem] items-center justify-center p-6">
			{#if showSpinner.current}
				<Loading />
			{/if}
		</div>
	{/if}
</GenericModal>
