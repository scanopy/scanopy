<script lang="ts">
	import type { IconComponent } from '$lib/shared/utils/types';
	import { tooltip } from '$lib/shared/actions/tooltip';

	interface Option {
		value: string;
		label: string;
		icon?: IconComponent;
		tooltip?: string;
		/** Render this one option non-selectable, leaving the rest interactive. */
		disabled?: boolean;
	}

	let {
		options,
		selected,
		onchange,
		size = 'sm',
		iconSize: iconSizeProp,
		disabled = false,
		fullWidth = false
	}: {
		options: Option[];
		selected: string;
		onchange: (value: string) => void;
		size?: 'sm' | 'md';
		iconSize?: 'sm' | 'md' | 'lg';
		disabled?: boolean;
		fullWidth?: boolean;
	} = $props();

	let sizeClasses = $derived(size === 'sm' ? 'px-2 py-1 text-xs' : 'px-3 py-1.5 text-sm');

	const iconSizeMap = { sm: 'h-3.5 w-3.5', md: 'h-4 w-4', lg: 'h-5 w-5' };
	let iconSizeClass = $derived(
		iconSizeProp ? iconSizeMap[iconSizeProp] : size === 'sm' ? 'h-3.5 w-3.5' : 'h-4 w-4'
	);
</script>

<div
	class="rounded-md border border-gray-600"
	class:inline-flex={!fullWidth}
	class:flex={fullWidth}
	class:opacity-50={disabled}
	class:cursor-not-allowed={disabled}
>
	{#each options as option (option.value)}
		{@const optionDisabled = disabled || option.disabled === true}
		<!-- The tooltip sits on the wrapper, not the button: a disabled button fires no
		     mouseenter, so an option explaining why it can't be picked would never show it. -->
		<span
			class="inline-flex"
			class:flex-1={fullWidth}
			use:tooltip
			data-tooltip={option.tooltip || null}
		>
			<button
				type="button"
				disabled={optionDisabled}
				class="{sizeClasses} flex items-center justify-center gap-1 transition-colors {selected ===
				option.value
					? 'bg-blue-600 text-white'
					: 'text-secondary hover:text-primary'} {option.disabled
					? 'cursor-not-allowed opacity-50'
					: ''}"
				class:w-full={fullWidth}
				onclick={() => {
					if (!optionDisabled) onchange(option.value);
				}}
			>
				{#if option.icon}
					{@const Icon = option.icon}
					<Icon class={iconSizeClass} />
				{/if}
				{#if option.label}
					{option.label}
				{/if}
			</button>
		</span>
	{/each}
</div>
