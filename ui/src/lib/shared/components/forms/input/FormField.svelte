<script lang="ts">
	import { AlertCircle } from '@lucide/svelte';
	import type { Snippet } from 'svelte';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import MessageWithLinks, {
		type MessageLink
	} from '#lib/shared/components/data/MessageWithLinks.svelte';

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
		/** Richer help below the input, such as text with entity chips. Drawn after `helpText`. */
		hint?: Snippet;
		/** Render label inline with input (for checkboxes) */
		inline?: boolean;
		/** Links an inline label places with `slot(key)`. */
		labelLinks?: Record<string, MessageLink>;
		/** Slot content */
		children: Snippet;
	}

	let {
		field,
		label,
		id,
		required = false,
		helpText = '',
		hint,
		inline = false,
		labelLinks = {},
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
				class="flex cursor-pointer items-center gap-2 text-sm font-medium text-secondary"
			>
				{@render children()}
				<div>
					<MessageWithLinks text={label} links={labelLinks} />
					{#if required}
						<span class="text-danger">*</span>
					{/if}
				</div>
			</label>
		{/if}

		{#if showErrors}
			<div class="flex items-center gap-2 text-danger">
				<AlertCircle size={16} />
				<p class="text-xs">{errors[0]}</p>
			</div>
		{/if}

		{#if helpText}
			<p class="text-xs text-tertiary">{helpText}</p>
		{/if}
	</div>
{:else}
	<div class="space-y-2">
		{#if label.length > 0}
			<label for={id} class="block text-sm font-medium text-secondary">
				{label}
				{#if required}
					<span class="ml-1 text-danger">*</span>
				{/if}
			</label>
		{/if}

		{@render children()}

		{#if showErrors}
			<div class="flex items-center gap-2 text-danger">
				<AlertCircle size={16} />
				<p class="text-xs">{errors[0]}</p>
			</div>
		{/if}

		{#if helpText}
			<p class="text-xs text-tertiary">{helpText}</p>
		{/if}

		{#if hint}
			{@render hint()}
		{/if}
	</div>
{/if}
