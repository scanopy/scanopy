<script lang="ts">
	import type { IconComponent } from '$lib/shared/utils/types';

	interface Option {
		value: string;
		label: string;
		icon?: IconComponent;
	}

	let {
		options,
		selected,
		onchange,
		size = 'sm',
		disabled = false,
		fullWidth = false
	}: {
		options: Option[];
		selected: string;
		onchange: (value: string) => void;
		size?: 'sm' | 'md';
		disabled?: boolean;
		fullWidth?: boolean;
	} = $props();

	let sizeClasses = $derived(size === 'sm' ? 'px-2 py-1 text-xs' : 'px-3 py-1.5 text-sm');
	let iconSize = $derived(size === 'sm' ? 'h-3.5 w-3.5' : 'h-4 w-4');
</script>

<div
	class="rounded-md border border-gray-600"
	class:inline-flex={!fullWidth}
	class:flex={fullWidth}
	class:opacity-50={disabled}
	class:cursor-not-allowed={disabled}
>
	{#each options as option (option.value)}
		<button
			type="button"
			{disabled}
			class="{sizeClasses} flex items-center justify-center gap-1 transition-colors {selected ===
			option.value
				? 'bg-blue-600 text-white'
				: 'text-secondary hover:text-primary'}"
			class:flex-1={fullWidth}
			onclick={() => {
				if (!disabled) onchange(option.value);
			}}
		>
			{#if option.icon}
				{@const Icon = option.icon}
				<Icon class={iconSize} />
			{/if}
			{#if option.label}
				{option.label}
			{/if}
		</button>
	{/each}
</div>
