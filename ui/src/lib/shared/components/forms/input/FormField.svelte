<script lang="ts">
	import { AlertCircle } from 'lucide-svelte';
	import type { Snippet } from 'svelte';
	import type { AnyFieldApi } from '@tanstack/svelte-form';

	interface Props {
		/** Field from TanStack Form snippet */
		field: AnyFieldApi;
		/** Field label text */
		label: string;
		/** Unique ID for the input */
		id: string;
		/** Show required indicator */
		required?: boolean;
		/** Help text below the input */
		helpText?: string;
		/** Render label inline with input (for checkboxes) */
		inline?: boolean;
		/** Slot content */
		children: Snippet;
	}

	let {
		field,
		label,
		id,
		required = false,
		helpText = '',
		inline = false,
		children
	}: Props = $props();

	let errors = $derived(field.state.meta.errors);
	let showErrors = $derived(field.state.meta.isTouched && errors.length > 0);
</script>

{#if inline}
	<div class="flex flex-col gap-2">
		{#if label.length > 0}
			<label
				for={id}
				class="text-secondary flex cursor-pointer items-center gap-2 text-sm font-medium"
			>
				{@render children()}
				<div>
					<!-- eslint-disable-next-line svelte/no-at-html-tags -- trusted: only used via Checkbox with i18n labels -->
					{@html label}
					{#if required}
						<span class="text-danger">*</span>
					{/if}
				</div>
			</label>
		{/if}

		{#if showErrors}
			<div class="text-danger flex items-center gap-2">
				<AlertCircle size={16} />
				<p class="text-xs">{errors[0]}</p>
			</div>
		{/if}

		{#if helpText}
			<p class="text-tertiary text-xs">{helpText}</p>
		{/if}
	</div>
{:else}
	<div class="space-y-2">
		{#if label.length > 0}
			<label for={id} class="text-secondary block text-sm font-medium">
				{label}
				{#if required}
					<span class="text-danger ml-1">*</span>
				{/if}
			</label>
		{/if}

		{@render children()}

		{#if showErrors}
			<div class="text-danger flex items-center gap-2">
				<AlertCircle size={16} />
				<p class="text-xs">{errors[0]}</p>
			</div>
		{/if}

		{#if helpText}
			<p class="text-tertiary text-xs">{helpText}</p>
		{/if}
	</div>
{/if}
