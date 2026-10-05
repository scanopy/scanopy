<script lang="ts" generics="T">
	import type { Snippet } from 'svelte';
	import type { GroupSlice } from './types';
	import TreeIndent from './TreeIndent.svelte';
	import { common_groupTotalShowing } from '$lib/paraglide/messages';

	/** Card view's body: one grid, or a headed grid per group. */
	let {
		items,
		groups,
		depthOf = null,
		getItemId,
		card
	}: {
		/** Ungrouped cards. Null when the list is grouped. */
		items: T[] | null;
		groups: { name: string; items: T[]; range: GroupSlice | null }[] | null;
		/**
		 * Each card's depth when the groups are trees. A tree reads top to bottom, so those groups
		 * render as one indented column rather than a grid that would scatter a parent's children
		 * across rows. Null otherwise.
		 */
		depthOf?: ((item: T) => number) | null;
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

				{#if depthOf}
					{@render tree(group.items, depthOf)}
				{:else}
					{@render grid(group.items)}
				{/if}
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

{#snippet tree(rows: T[], depth: (item: T) => number)}
	<div class="space-y-3">
		{#each rows as item (getItemId(item))}
			<div class="flex items-start gap-2" style="padding-left: {depth(item) * 16}px">
				<div class="pt-4"><TreeIndent depth={Math.min(depth(item), 1)} /></div>
				<div class="min-w-0 flex-1">{@render card(item)}</div>
			</div>
		{/each}
	</div>
{/snippet}
