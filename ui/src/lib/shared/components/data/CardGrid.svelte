<script lang="ts" generics="T">
	import type { Snippet } from 'svelte';
	import type { GroupSlice } from './types';
	import { common_groupTotalShowing } from '$lib/paraglide/messages';

	/** Card view's body: one grid, or a headed grid per group. */
	let {
		items,
		groups,
		getItemId,
		card
	}: {
		/** Ungrouped cards. Null when the list is grouped. */
		items: T[] | null;
		groups: { name: string; items: T[]; range: GroupSlice | null }[] | null;
		getItemId: (item: T) => string;
		card: Snippet<[T]>;
	} = $props();
</script>

{#if groups}
	<div class="space-y-6">
		{#each groups as group (group.name)}
			<div class="space-y-3">
				<div class="flex items-center gap-3">
					<h3 class="text-primary text-lg font-semibold">{group.name}</h3>
					<span class="text-tertiary text-sm">
						{#if group.range}
							{common_groupTotalShowing({
								total: group.range.total,
								start: group.range.start,
								end: group.range.end
							})}
						{:else}
							({group.items.length})
						{/if}
					</span>
				</div>

				{@render grid(group.items)}
			</div>
		{/each}
	</div>
{:else}
	{@render grid(items ?? [])}
{/if}

{#snippet grid(rows: T[])}
	<div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
		{#each rows as item (getItemId(item))}
			{@render card(item)}
		{/each}
	</div>
{/snippet}
