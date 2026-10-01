<script lang="ts" generics="T extends string">
	import FormField from './FormField.svelte';
	import type { AnyFieldApi } from '@tanstack/svelte-form';

	interface RadioOption {
		value: T;
		label: string;
		helpText?: string;
	}

	interface Props {
		label: string;
		field: AnyFieldApi;
		id: string;
		options: RadioOption[];
		required?: boolean;
		disabled?: boolean;
		/** The selected value, when the caller keeps it in its own `$state`. TanStack's
		 *  `field.state.value` is not tracked by Svelte 5, so a value set programmatically
		 *  would not re-render; passing it here makes the caller's state drive `checked`. */
		value?: T;
		/** Called with the chosen value, after the form field is updated. */
		onChange?: (value: T) => void;
	}

	let {
		label,
		field,
		id,
		options,
		required = false,
		disabled = false,
		value,
		onChange
	}: Props = $props();

	let selected = $derived(value !== undefined ? value : field.state.value);
</script>

<div class:disabled>
	<FormField {label} {field} {id} {required}>
		<div class="flex flex-col gap-3 sm:flex-row sm:gap-4">
			{#each options as option (option.value)}
				<label class="card card-static flex flex-1 cursor-pointer flex-col gap-2 p-3">
					<div class="flex items-center gap-2">
						<input
							type="radio"
							name={id}
							value={option.value}
							checked={selected === option.value}
							{disabled}
							onchange={() => {
								field.handleChange(option.value);
								onChange?.(option.value);
							}}
							class="checkbox-card h-4 w-4 disabled:cursor-not-allowed disabled:opacity-50"
						/>
						<span class="text-primary text-sm">{option.label}</span>
					</div>
					{#if option.helpText}
						<p class="text-tertiary text-xs">{option.helpText}</p>
					{/if}
				</label>
			{/each}
		</div>
	</FormField>
</div>

<style>
	.disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.disabled :global(label) {
		cursor: not-allowed;
	}
</style>
