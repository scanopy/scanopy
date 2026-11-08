<script lang="ts">
	import { ChevronDown, ChevronRight } from '@lucide/svelte';
	import type { GroupSlice } from '../types';
	import { common_groupTotalShowing } from '$lib/paraglide/messages';

	/**
	 * A group's heading: a button that collapses or expands the group, with its name and its row
	 * count, or the slice of a paged group's total this page shows.
	 */
	let {
		name,
		count,
		range = null,
		collapsed,
		onToggle
	}: {
		name: string;
		/** Rows in the group, shown when there is no `range`. */
		count: number;
		range?: GroupSlice | null;
		collapsed: boolean;
		onToggle: () => void;
	} = $props();
</script>

<button
	type="button"
	onclick={onToggle}
	aria-expanded={!collapsed}
	class="pin-content flex w-max items-center gap-2 text-sm font-semibold text-primary"
>
	{#if collapsed}
		<ChevronRight class="h-4 w-4" aria-hidden="true" />
	{:else}
		<ChevronDown class="h-4 w-4" aria-hidden="true" />
	{/if}
	<span>{name}</span>
	<span class="text-xs font-normal text-tertiary">
		{#if range}
			{common_groupTotalShowing({
				total: range.total,
				start: range.start,
				end: range.end
			})}
		{:else}
			({count})
		{/if}
	</span>
</button>
