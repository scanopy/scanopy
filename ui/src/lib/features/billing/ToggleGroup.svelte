<script lang="ts">
	interface ToggleOption {
		value: string;
		label: string;
		badge?: string;
	}

	interface Props {
		options: ToggleOption[];
		selected: string;
		onchange: (value: string) => void;
		/** Render the group non-interactive (still shows the selected option). */
		disabled?: boolean;
	}

	let { options, selected, onchange, disabled = false }: Props = $props();
</script>

<div class="card inline-flex gap-1 rounded-full p-1 {disabled ? 'opacity-50' : ''}">
	{#each options as option (option.value)}
		<button
			type="button"
			{disabled}
			onclick={() => onchange(option.value)}
			class="rounded-full px-2.5 py-1 text-sm font-medium transition-all {selected === option.value
				? 'bg-gray-200 text-gray-900 dark:bg-gray-700 dark:text-white'
				: 'text-tertiary hover:text-secondary'} {disabled ? 'cursor-not-allowed' : ''}"
		>
			<span>{option.label}</span>
			{#if option.badge}
				<span
					class="ml-1 text-xs font-semibold {selected === option.value
						? 'text-green-700 dark:text-green-300'
						: 'text-success'}">{option.badge}</span
				>
			{/if}
		</button>
	{/each}
</div>
