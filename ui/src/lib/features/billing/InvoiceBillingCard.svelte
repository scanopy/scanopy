<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { max } from '$lib/shared/components/forms/validators';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import { useUpdatePoNumberMutation } from './queries';
	import { pushSuccess } from '$lib/shared/stores/feedback';
	import type { components } from '$lib/api/schema';
	import {
		billing_invoice_poNumber,
		billing_invoice_poNumberHelp,
		billing_invoice_poNumberUpdated,
		billing_invoice_quoteBodyNoPo,
		billing_invoice_quoteBodyWithPo,
		billing_invoice_quoteTitle,
		common_cancel,
		common_edit,
		common_processing,
		common_save
	} from '$lib/paraglide/messages';

	type InvoiceBillingStatus = components['schemas']['InvoiceBillingStatus'];

	// Accepting, downloading and cancelling a quote are the tab's primary,
	// secondary and overflow actions, so they live with the other CTAs in
	// BillingTab's ButtonMenu rather than as a button row inside this card.
	let { status }: { status: InvoiceBillingStatus } = $props();

	const poMutation = useUpdatePoNumberMutation();

	function formatMoney(cents: number, currency: string): string {
		return new Intl.NumberFormat(undefined, {
			style: 'currency',
			currency: currency.toUpperCase()
		}).format(cents / 100);
	}

	function formatDate(value: string): string {
		return new Date(value).toLocaleDateString(undefined, {
			month: 'long',
			day: 'numeric',
			year: 'numeric'
		});
	}

	let quote = $derived(status.pending_quote ?? null);
	let showPo = $derived(status.bills_by_invoice || quote != null || status.po_number != null);

	let editingPo = $state(false);

	const poForm = createForm(() => ({
		defaultValues: { po_number: '' },
		onSubmit: async ({ value }) => {
			try {
				await poMutation.mutateAsync(value.po_number.trim() || null);
				editingPo = false;
				pushSuccess(billing_invoice_poNumberUpdated());
			} catch {
				// The API client toasts the failure.
			}
		}
	}));

	function startEditingPo() {
		poForm.reset({ po_number: status.po_number ?? '' });
		editingPo = true;
	}
</script>

{#if showPo}
	<dl class="grid grid-cols-[auto_1fr] items-center gap-x-6 gap-y-1 text-sm">
		{#if showPo && !editingPo}
			<dt class="text-secondary">{billing_invoice_poNumber()}</dt>
			<dd class="text-primary flex items-center gap-3">
				<span>{status.po_number ?? '—'}</span>
				<button type="button" class="text-link hover:underline" onclick={startEditingPo}>
					{common_edit()}
				</button>
			</dd>
		{/if}
	</dl>
{/if}

{#if editingPo}
	<form
		class="space-y-3"
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			submitForm(poForm);
		}}
	>
		<poForm.Field name="po_number" validators={{ onBlur: ({ value }) => max(140)(value) }}>
			{#snippet children(field)}
				<TextInput
					label={billing_invoice_poNumber()}
					id="license-po-number"
					helpText={billing_invoice_poNumberHelp()}
					{field}
				/>
			{/snippet}
		</poForm.Field>
		<div class="flex gap-2">
			<button type="submit" class="btn-primary" disabled={poMutation.isPending}>
				{poMutation.isPending ? common_processing() : common_save()}
			</button>
			<button type="button" class="btn-secondary" onclick={() => (editingPo = false)}>
				{common_cancel()}
			</button>
		</div>
	</form>
{/if}

{#if quote}
	<div class="card card-static space-y-3 p-4">
		<h3 class="text-primary text-sm font-semibold">
			{billing_invoice_quoteTitle({ number: quote.number ?? '' })}
		</h3>
		<p class="text-secondary text-sm">
			{status.po_number
				? billing_invoice_quoteBodyWithPo({
						amount: formatMoney(quote.amount_total_cents, quote.currency),
						date: formatDate(quote.expires_at),
						po: status.po_number
					})
				: billing_invoice_quoteBodyNoPo({
						amount: formatMoney(quote.amount_total_cents, quote.currency),
						date: formatDate(quote.expires_at)
					})}
		</p>
	</div>
{/if}
