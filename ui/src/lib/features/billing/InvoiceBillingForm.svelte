<script lang="ts">
	// `/pure` for the same reason as StripeCardForm: importing must not inject Stripe.js.
	import { loadStripe } from '@stripe/stripe-js/pure';
	import type {
		Stripe,
		StripeAddressElement,
		StripeElements,
		StripeTaxIdElement
	} from '@stripe/stripe-js';
	import { untrack } from 'svelte';
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, email as emailValidator, max } from '$lib/shared/components/forms/validators';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import { buildStripeAppearance } from '$lib/shared/billing/stripe-appearance';
	import { useSetUpInvoiceBillingMutation } from './queries';
	import type { BillingPlan } from './types';
	import billingPlansJson from '$lib/data/billing-plans.json';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import type { components } from '$lib/api/schema';
	import {
		billing_cardLoadError,
		billing_invoice_addressIncomplete,
		billing_invoice_billingEmail,
		billing_invoice_billingEmailHelp,
		billing_invoice_choosePlan,
		billing_invoice_getQuote,
		billing_invoice_poNumber,
		billing_invoice_poNumberHelp,
		billing_invoice_sendNow,
		billing_invoice_trialEnds,
		billing_payByCard,
		common_cancel,
		common_processing
	} from '$lib/paraglide/messages';

	type InvoiceBillingMode = components['schemas']['InvoiceBillingMode'];

	let {
		plan = null,
		needsPlanChoice = false,
		orgName = undefined,
		email = undefined,
		isTrialing = false,
		onDone,
		onCancel,
		onPayByCard
	}: {
		/** Plan to invoice for, when the org has no live subscription. */
		plan?: BillingPlan | null;
		/**
		 * The org has no live subscription and no licensed plan of its own, so
		 * the invoice needs a plan picked here.
		 */
		needsPlanChoice?: boolean;
		/** Organization name, prefilled as the billing entity. */
		orgName?: string;
		/** Default billing email. */
		email?: string;
		isTrialing?: boolean;
		/** Called with the mode that succeeded. */
		onDone: (mode: InvoiceBillingMode) => void | Promise<void>;
		onCancel: () => void;
		/** Back to Stripe's card and bank form. */
		onPayByCard: () => void;
	} = $props();

	// Licensed self-hosted plans, annual rows only, from the same fixture the
	// plan picker reads. Held in the form itself so a page reload mid-flow
	// cannot lose the plan the way transient modal state did.
	const licensedPlans: BillingPlan[] = billingPlansJson
		.filter((p) => p.metadata.license_plan != null && p.metadata.rate === 'Year')
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
					// Checkout validates the whole plan config against the server's.
					included_orgs: p.metadata.included_orgs ?? null,
					host_cents: p.metadata.host_cents ?? null,
					included_hosts: p.metadata.included_hosts ?? null
				}) as BillingPlan
		);
	// Initial value only: the prop names the plan the picker sent, and the
	// buttons below own it from there.
	// eslint-disable-next-line svelte/prefer-writable-derived
	let selectedPlan = $state<BillingPlan | null>(untrack(() => plan));
	let missingPlan = $derived(needsPlanChoice && selectedPlan == null);

	const configQuery = useConfigQuery();
	let publishableKey = $derived(configQuery.data?.stripe_publishable_key ?? null);
	let configLoaded = $derived(configQuery.data != null);

	const setUpMutation = useSetUpInvoiceBillingMutation();

	let addressContainer = $state<HTMLDivElement | null>(null);
	let taxIdContainer = $state<HTMLDivElement | null>(null);
	let stripe: Stripe | null = null;
	let elements: StripeElements | null = null;
	let addressElement: StripeAddressElement | null = null;
	let taxIdElement: StripeTaxIdElement | null = null;
	let ready = $state(false);
	let loadFailed = $state(false);
	let errorMessage = $state('');
	let busyMode = $state<InvoiceBillingMode | null>(null);
	let initialized = false;

	// The entity's name and address come from Stripe's Address Element, its tax
	// ID from the Tax ID Element (beta). Elements run without an intent: nothing
	// is confirmed client-side, the values go to the backend, which writes them
	// onto the Stripe customer.
	$effect(() => {
		if (initialized || !addressContainer || !taxIdContainer) return;
		if (configLoaded && !publishableKey) {
			initialized = true;
			loadFailed = true;
			return;
		}
		if (!publishableKey) return;

		initialized = true;
		const addressNode = addressContainer;
		const taxIdNode = taxIdContainer;
		void (async () => {
			stripe = await loadStripe(publishableKey, { betas: ['elements_tax_id_1'] });
			if (!stripe) {
				loadFailed = true;
				return;
			}
			elements = stripe.elements({
				mode: 'setup',
				currency: 'usd',
				appearance: buildStripeAppearance(),
				loader: 'never'
			});
			addressElement = elements.create('address', {
				mode: 'billing',
				display: { name: 'organization' },
				// The name the buyer gave at signup is the entity nine times out
				// of ten; they can still edit it before submitting.
				defaultValues: orgName ? { name: orgName } : undefined
			});
			taxIdElement = elements.create('taxId', {
				visibility: 'auto',
				fields: { businessName: 'never' }
			});
			addressElement.on('ready', () => (ready = true));
			addressElement.on('loaderror', () => (loadFailed = true));
			addressElement.mount(addressNode);
			taxIdElement.mount(taxIdNode);
		})();
	});

	let submitMode: InvoiceBillingMode = 'send_invoice';

	const form = createForm(() => ({
		defaultValues: {
			billing_email: email ?? '',
			po_number: ''
		},
		onSubmit: async ({ value }) => {
			if (!addressElement || !taxIdElement) return;
			errorMessage = '';

			const address = await addressElement.getValue();
			if (!address.complete) {
				errorMessage = billing_invoice_addressIncomplete();
				return;
			}
			const taxId = await taxIdElement.getValue();
			const hasTaxId = taxId.visible && !taxId.empty && taxId.complete;

			const mode = submitMode;
			busyMode = mode;
			try {
				await setUpMutation.mutateAsync({
					mode,
					plan: selectedPlan,
					details: {
						entity_name: address.value.name,
						billing_email: value.billing_email.trim(),
						address: {
							line1: address.value.address.line1,
							line2: address.value.address.line2 || null,
							city: address.value.address.city,
							state: address.value.address.state || null,
							postal_code: address.value.address.postal_code,
							country: address.value.address.country
						},
						tax_id: hasTaxId
							? {
									tax_id_type: taxId.value.externalTaxIdType,
									value: taxId.value.taxId
								}
							: null,
						po_number: value.po_number.trim() || null
					}
				});
				await onDone(mode);
			} catch {
				// The API client toasts the failure.
			} finally {
				busyMode = null;
			}
		}
	}));

	async function submitWith(mode: InvoiceBillingMode) {
		submitMode = mode;
		await submitForm(form);
	}
</script>

<form
	onsubmit={(e) => {
		e.preventDefault();
		e.stopPropagation();
		submitWith('send_invoice');
	}}
	class="flex min-h-0 flex-1 flex-col"
>
	<div class="min-h-0 flex-1 space-y-4 overflow-auto p-6">
		{#if isTrialing}
			<InlineInfo title={billing_invoice_trialEnds()} />
		{/if}

		{#if needsPlanChoice}
			<div class="space-y-2">
				<p class="text-secondary text-sm">{billing_invoice_choosePlan()}</p>
				<div class="flex flex-wrap gap-2">
					{#each licensedPlans as licensed (licensed.type)}
						{@const selected = selectedPlan?.type === licensed.type}
						<button
							type="button"
							class="card flex-1 p-3 text-left transition-all {selected
								? 'ring-2 ring-primary-500'
								: 'hover:bg-gray-100 dark:hover:bg-gray-800'}"
							aria-pressed={selected}
							onclick={() => (selectedPlan = licensed)}
						>
							<span class="text-primary block text-sm font-medium">
								{billingPlans.getName(licensed.type)}
							</span>
							<span class="text-secondary text-xs">
								${(licensed.base_cents / 100).toLocaleString('en-US')}
							</span>
						</button>
					{/each}
				</div>
			</div>
		{/if}

		<div class="relative min-h-[8rem]">
			<div class:invisible={!ready} class="space-y-4">
				<div bind:this={addressContainer}></div>
				<div bind:this={taxIdContainer}></div>
			</div>
			{#if !ready && !loadFailed}
				<div class="absolute inset-0 flex items-center justify-center">
					<Loading />
				</div>
			{/if}
		</div>

		{#if loadFailed}
			<p class="text-sm text-red-400">{billing_cardLoadError()}</p>
		{/if}

		<form.Field
			name="billing_email"
			validators={{
				onBlur: ({ value }) => required(value) || emailValidator(value)
			}}
		>
			{#snippet children(field)}
				<TextInput
					label={billing_invoice_billingEmail()}
					id="invoice-billing-email"
					type="email"
					required
					helpText={billing_invoice_billingEmailHelp()}
					{field}
				/>
			{/snippet}
		</form.Field>

		<form.Field
			name="po_number"
			validators={{
				onBlur: ({ value }) => max(140)(value)
			}}
		>
			{#snippet children(field)}
				<TextInput
					label={billing_invoice_poNumber()}
					id="invoice-po-number"
					helpText={billing_invoice_poNumberHelp()}
					{field}
				/>
			{/snippet}
		</form.Field>

		{#if errorMessage}
			<p class="text-sm text-red-400">{errorMessage}</p>
		{/if}

		<button
			type="button"
			class="text-link text-sm hover:underline"
			disabled={busyMode != null}
			onclick={onPayByCard}
		>
			{billing_payByCard()}
		</button>
	</div>

	<div class="modal-footer flex flex-wrap items-center justify-end gap-3">
		<button type="button" class="btn-secondary" disabled={busyMode != null} onclick={onCancel}>
			{common_cancel()}
		</button>
		<button
			type="button"
			class="btn-secondary"
			disabled={!ready || busyMode != null || missingPlan}
			onclick={() => submitWith('quote')}
		>
			{busyMode === 'quote' ? common_processing() : billing_invoice_getQuote()}
		</button>
		<button type="submit" class="btn-primary" disabled={!ready || busyMode != null || missingPlan}>
			{busyMode === 'send_invoice' ? common_processing() : billing_invoice_sendNow()}
		</button>
	</div>
</form>
