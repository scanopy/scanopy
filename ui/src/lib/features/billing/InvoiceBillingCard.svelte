<script lang="ts">
	import { Download } from 'lucide-svelte';
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { max } from '$lib/shared/components/forms/validators';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import {
		downloadQuotePdf,
		useAcceptQuoteMutation,
		useCancelQuoteMutation,
		useUpdatePoNumberMutation
	} from './queries';
	import { pushSuccess } from '$lib/shared/stores/feedback';
	import type { components } from '$lib/api/schema';
	import {
		billing_invoice_acceptConfirm,
		billing_invoice_acceptNoPo,
		billing_invoice_acceptTitle,
		billing_invoice_acceptWithPo,
		billing_invoice_accepted,
		billing_invoice_downloadQuote,
		billing_invoice_poNumber,
		billing_invoice_poNumberHelp,
		billing_invoice_poNumberUpdated,
		billing_invoice_quoteBodyNoPo,
		billing_invoice_quoteBodyWithPo,
		billing_invoice_quoteCancelled,
		billing_invoice_quoteTitle,
		common_cancel,
		common_edit,
		common_processing,
		common_save
	} from '$lib/paraglide/messages';

	type InvoiceBillingStatus = components['schemas']['InvoiceBillingStatus'];

	let { status }: { status: InvoiceBillingStatus } = $props();

	const acceptMutation = useAcceptQuoteMutation();
	const cancelMutation = useCancelQuoteMutation();
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
	let acceptingQuote = $state(false);

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

	// The PO row above owns the number; accepting a quote just confirms what it
	// already says, rather than asking a second time and inviting two answers.
	async function handleAccept() {
		try {
			await acceptMutation.mutateAsync();
			acceptingQuote = false;
			pushSuccess(billing_invoice_accepted());
		} catch {
			// The API client toasts the failure.
		}
	}

	// Stripe renders the PDF on demand and it proxies through the backend, so
	// the wait is long enough to need saying.
	let downloading = $state(false);

	async function handleDownload() {
		downloading = true;
		try {
			await downloadQuotePdf(quote?.number ?? null);
		} finally {
			downloading = false;
		}
	}

	async function handleCancelQuote() {
		try {
			await cancelMutation.mutateAsync();
			pushSuccess(billing_invoice_quoteCancelled());
		} catch {
			// The API client toasts the failure.
		}
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
		<div class="flex flex-wrap gap-2">
			<button
				type="button"
				class="btn-secondary flex items-center gap-2"
				disabled={downloading}
				onclick={handleDownload}
			>
				<Download class="h-4 w-4" />
				{downloading ? common_processing() : billing_invoice_downloadQuote()}
			</button>
			<button type="button" class="btn-primary" onclick={() => (acceptingQuote = true)}>
				{billing_invoice_acceptTitle()}
			</button>
			<button
				type="button"
				class="btn-secondary"
				disabled={cancelMutation.isPending}
				onclick={handleCancelQuote}
			>
				{common_cancel()}
			</button>
		</div>
	</div>
{/if}

<GenericModal
	isOpen={acceptingQuote}
	title={billing_invoice_acceptTitle()}
	size="sm"
	onClose={() => (acceptingQuote = false)}
>
	<div class="flex min-h-0 flex-1 flex-col">
		<div class="p-6">
			<p class="text-secondary text-sm">
				{status.po_number
					? billing_invoice_acceptWithPo({ po: status.po_number })
					: billing_invoice_acceptNoPo()}
			</p>
		</div>
		<div class="modal-footer flex justify-end gap-3">
			<button type="button" class="btn-secondary" onclick={() => (acceptingQuote = false)}>
				{common_cancel()}
			</button>
			<button
				type="button"
				class="btn-primary"
				disabled={acceptMutation.isPending}
				onclick={handleAccept}
			>
				{acceptMutation.isPending ? common_processing() : billing_invoice_acceptConfirm()}
			</button>
		</div>
	</div>
</GenericModal>
