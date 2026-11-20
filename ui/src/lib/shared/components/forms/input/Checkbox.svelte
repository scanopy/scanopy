<script lang="ts">
	import FormField from './FormField.svelte';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import type { MessageLink } from '#lib/shared/components/data/MessageWithLinks.svelte';

	interface Props {
		label: string;
		field: AnyFieldApi;
		id: string;
		helpText?: string;
		required?: boolean;
		disabled?: boolean;
		/** Links the label places with `slot(key)`. */
		labelLinks?: Record<string, MessageLink>;
	}

	let {
		label,
		field,
		id,
		helpText = '',
		required = false,
		disabled = false,
		labelLinks = {}
	}: Props = $props();
</script>

<div class:disabled>
	<FormField {label} {field} {helpText} {id} inline={true} {required} {labelLinks}>
		<input
			type="checkbox"
			{id}
			checked={field.state.value}
			{disabled}
			onchange={(e) => field.handleChange(e.currentTarget.checked)}
			class="checkbox-card h-4 w-4 focus:ring-1 focus:ring-blue-500 disabled:cursor-not-allowed disabled:opacity-50"
		/>
	</FormField>
</div>

<style>
	input[type='checkbox']:checked {
		background-color: rgb(37, 99, 235);
		border-color: rgb(37, 99, 235);
	}

	.disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.disabled :global(label) {
		cursor: not-allowed;
	}
</style>
