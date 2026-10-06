<script lang="ts" generics="T">
	import type { Snippet } from 'svelte';
	import { ChevronDown, ChevronRight } from 'lucide-svelte';
	import type { RenderGroup, TreeEntry, TreeSection } from './controls/grouping';
	import TreeGuides from './TreeGuides.svelte';
	import { common_groupTotalShowing } from '$lib/paraglide/messages';

	/** Card view's body: one grid, or a collapsible headed grid per group. */
	let {
		items,
		groups,
		collapsed,
		onToggleCollapse,
		getItemId,
		card
	}: {
		/** Ungrouped cards. Null when the list is grouped. */
		items: T[] | null;
		/**
		 * Grouped cards. A group with `entries` is a tree: it reads top to bottom, so it renders as
		 * one column of nested sections rather than a grid that would scatter a parent's children
		 * across rows.
		 */
		groups: RenderGroup<T>[] | null;
		/** Keys of the collapsed groups and tree sections. Owned by the caller. */
		collapsed: ReadonlySet<string>;
		onToggleCollapse: (key: string) => void;
		getItemId: (item: T) => string;
		card: Snippet<[T]>;
	} = $props();

	function entryKey(entry: TreeEntry<T>): string {
		return entry.type === 'row' ? getItemId(entry.item) : entry.key;
	}
</script>

{#if groups}
	<div class="space-y-6">
		{#each groups as group (group.key)}
			{@const isCollapsed = collapsed.has(group.key)}
			<div class="space-y-3">
				<h3>
					<button
						type="button"
						onclick={() => onToggleCollapse(group.key)}
						aria-expanded={!isCollapsed}
						class="flex items-center gap-3"
					>
						{#if isCollapsed}
							<ChevronRight class="text-secondary h-5 w-5" aria-hidden="true" />
						{:else}
							<ChevronDown class="text-secondary h-5 w-5" aria-hidden="true" />
						{/if}
						<span class="text-primary text-lg font-semibold">{group.name}</span>
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
					</button>
				</h3>

				{#if !isCollapsed}
					{#if group.entries}
						<div>{@render treeEntries(group.entries)}</div>
					{:else}
						{@render grid(group.items)}
					{/if}
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

<!--
	Spacing is padding inside each entry rather than a gap between them, so a section's guide line
	runs unbroken from one card to the next.
-->
{#snippet treeEntries(entries: TreeEntry<T>[])}
	{#each entries as entry (entryKey(entry))}
		{#if entry.type === 'row'}
			<div class="relative pb-3" style="padding-left: {entry.guides.length}rem">
				<TreeGuides guides={entry.guides} />
				{@render card(entry.item)}
			</div>
		{:else}
			{@render sectionHeader(entry)}
			{#if !collapsed.has(entry.key)}
				{@render treeEntries(entry.entries)}
			{/if}
		{/if}
	{/each}
{/snippet}

{#snippet sectionHeader(section: TreeSection<T>)}
	{@const isCollapsed = collapsed.has(section.key)}
	<div class="relative pb-3" style="padding-left: {section.guides.length}rem">
		<TreeGuides guides={section.guides} />
		<button
			type="button"
			onclick={() => onToggleCollapse(section.key)}
			aria-expanded={!isCollapsed}
			class="text-primary flex items-center gap-2 text-sm font-semibold"
		>
			{#if isCollapsed}
				<ChevronRight class="h-4 w-4" aria-hidden="true" />
			{:else}
				<ChevronDown class="h-4 w-4" aria-hidden="true" />
			{/if}
			<span>{section.label}</span>
			<span class="text-tertiary text-xs font-normal">({section.count})</span>
		</button>
	</div>
{/snippet}
