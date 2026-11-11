<script lang="ts">
	import Tag from '#lib/shared/components/data/Tag.svelte';
	import type { Color } from '#lib/shared/utils/styling.ts';
	import type { IconComponent } from '#lib/shared/utils/types.ts';

	export interface FilterItem {
		value: string;
		label: string;
		color: Color;
		icon?: IconComponent | null;
		isShiny?: boolean;
		tooltip?: string;
	}

	let {
		items,
		selectedValues,
		mode,
		onToggle,
		onHoverStart,
		onHoverEnd,
		disabled = false,
		label,
		nativeTooltip = false
	}: {
		items: FilterItem[];
		selectedValues: string[];
		mode: 'include' | 'exclude';
		onToggle: (value: string) => void;
		onHoverStart?: (value: string, color: Color) => void;
		onHoverEnd?: () => void;
		disabled?: boolean;
		label?: string;
		nativeTooltip?: boolean;
	} = $props();

	function isItemFaded(value: string): boolean {
		const isSelected = selectedValues.includes(value);
		if (mode === 'exclude') {
			return isSelected;
		}
		return !isSelected;
	}
</script>

<div class="space-y-2">
	{#if label}
		<div class="text-xs text-tertiary">{label}</div>
	{/if}
	<div class="flex flex-wrap gap-1.5">
		{#each items as item (item.value)}
			<Tag
				label={item.label}
				color={item.color}
				icon={item.icon}
				isShiny={item.isShiny}
				title={item.tooltip ?? ''}
				{nativeTooltip}
				{disabled}
				faded={isItemFaded(item.value)}
				onclick={() => !disabled && onToggle(item.value)}
				onmouseenter={() => onHoverStart?.(item.value, item.color)}
				onmouseleave={() => onHoverEnd?.()}
			/>
		{/each}
	</div>
</div>
