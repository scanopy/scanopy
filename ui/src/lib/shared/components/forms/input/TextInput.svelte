<script lang="ts">
	import FormField from './FormField.svelte';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import type { Snippet } from 'svelte';
	import { Lock } from '@lucide/svelte';
	import { tooltip } from '#lib/shared/actions/tooltip.ts';

	interface Props {
		label: string;
		field: AnyFieldApi;
		id: string;
		placeholder?: string;
		required?: boolean;
		helpText?: string;
		/** Richer help below the input, such as text with entity chips. */
		hint?: Snippet;
		type?: 'text' | 'email' | 'password' | 'number';
		disabled?: boolean;
		/**
		 * Why the field can't be edited. Disables it, draws a lock in it, and shows the reason on
		 * hover. The hover sits on a wrapper, because a disabled input fires no pointer events.
		 */
		disabledReason?: string | null;
		autofocus?: boolean;
		/** The field's accessible name when `label` is empty because a label is drawn beside it. */
		ariaLabel?: string;
	}

	let {
		label,
		field,
		id,
		placeholder = '',
		required = false,
		helpText = '',
		hint,
		type = 'text',
		disabled = false,
		disabledReason = null,
		autofocus = false,
		ariaLabel = undefined
	}: Props = $props();

	let hasErrors = $derived(field.state.meta.isTouched && field.state.meta.errors.length > 0);

	function focusOnMount(node: HTMLInputElement) {
		if (autofocus) node.focus();
	}
</script>

<FormField {label} {field} {required} {helpText} {hint} {id}>
	<div class="relative" use:tooltip data-tooltip={disabledReason ?? undefined}>
		<input
			use:focusOnMount
			{id}
			{type}
			value={field.state.value ?? ''}
			onblur={() => field.handleBlur()}
			oninput={(e) => {
				const value = e.currentTarget.value;
				// Convert to number for number inputs, otherwise keep as string
				field.handleChange(type === 'number' ? (value === '' ? '' : Number(value)) : value);
			}}
			{placeholder}
			aria-label={ariaLabel}
			disabled={disabled || !!disabledReason}
			class="input-field"
			class:pr-9={!!disabledReason}
			class:input-field-error={hasErrors}
		/>
		{#if disabledReason}
			<Lock
				class="pointer-events-none absolute top-1/2 right-3 h-4 w-4 -translate-y-1/2 text-tertiary"
			/>
		{/if}
	</div>
</FormField>
