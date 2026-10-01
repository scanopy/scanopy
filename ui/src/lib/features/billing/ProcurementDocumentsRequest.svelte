<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import Checkbox from '$lib/shared/components/forms/input/Checkbox.svelte';
	import { billingMailto } from '$lib/features/support/support';
	import {
		settings_billing_procurementDocuments,
		settings_billing_procurementDocumentsBody,
		settings_billing_procurementDocumentsSubject,
		settings_billing_procurementNdaa889,
		common_w9
	} from '$lib/paraglide/messages';

	let {
		orgName,
		orgId,
		planName
	}: {
		orgName: string;
		orgId: string;
		planName: string;
	} = $props();

	const documents = [
		{ name: 'w9', label: () => common_w9() },
		{ name: 'ndaa889', label: () => settings_billing_procurementNdaa889() }
	] as const;

	const form = createForm(() => ({
		defaultValues: { w9: false, ndaa889: false },
		onSubmit: ({ value }) => {
			const requested = documents
				.filter((doc) => value[doc.name])
				.map((doc) => `- ${doc.label()}`)
				.join('\n');
			window.location.href = billingMailto(
				settings_billing_procurementDocumentsSubject({ orgName }),
				settings_billing_procurementDocumentsBody({
					documents: requested,
					orgName,
					orgId,
					planName
				})
			);
		}
	}));
</script>

<form
	onsubmit={(e) => {
		e.preventDefault();
		form.handleSubmit();
	}}
>
	<form.Subscribe selector={(state) => state.values.w9 || state.values.ndaa889}>
		{#snippet children(anySelected)}
			<button
				type="submit"
				disabled={!anySelected}
				class="text-link text-sm hover:underline disabled:cursor-not-allowed disabled:no-underline disabled:opacity-50"
			>
				{settings_billing_procurementDocuments()}
			</button>
		{/snippet}
	</form.Subscribe>
	<div class="mt-2 space-y-2">
		{#each documents as doc (doc.name)}
			<form.Field name={doc.name}>
				{#snippet children(field)}
					<Checkbox id="procurement-{doc.name}" label={doc.label()} {field} />
				{/snippet}
			</form.Field>
		{/each}
	</div>
</form>
