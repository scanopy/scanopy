<script lang="ts">
	import Tag from './Tag.svelte';
	import TagPickerInline from '#lib/features/tags/components/TagPickerInline.svelte';
	import type { EntityDiscriminants } from '#lib/features/tags/queries.ts';
	import type { CardFieldItem } from './types';
	import { MAX_ITEMS_IN_CELL } from './types';
	import { common_moreItems, common_tags } from '#lib/paraglide/messages.js';
	import { MediaQuery } from 'svelte/reactivity';
	import { Plus } from '@lucide/svelte';
	import Popover from './Popover.svelte';

	/**
	 * A row's tags.
	 *
	 * When the viewer can edit, this is the picker itself rather than chips with
	 * an edit affordance: a chip has to be removable in place, and gating that
	 * behind a mode meant a tag could be added from the table but not taken off.
	 *
	 * Read-only viewers get plain chips, capped, since none of the picker's
	 * machinery would do anything for them.
	 */
	let {
		items,
		tagIds,
		entityId,
		entityType,
		editable = true
	}: {
		items: CardFieldItem[];
		tagIds: string[];
		entityId: string;
		/** Absent when the viewer cannot edit tags, which also disables the picker. */
		entityType?: EntityDiscriminants;
		editable?: boolean;
	} = $props();

	let showAll = $state(false);

	// Below `md` the cell shows one chip and a count; an editor taps it to open the picker.
	const narrowViewport = new MediaQuery('max-width: 767px');
	let pickerOpen = $state(false);
	let trigger: HTMLButtonElement | undefined = $state();

	let cap = $derived(narrowViewport.current ? 1 : MAX_ITEMS_IN_CELL);
	let visible = $derived(showAll ? items : items.slice(0, cap));
	let overflow = $derived(items.length - visible.length);
</script>

{#if editable && entityType && narrowViewport.current}
	<button
		type="button"
		bind:this={trigger}
		onclick={() => (pickerOpen = !pickerOpen)}
		aria-haspopup="dialog"
		aria-expanded={pickerOpen}
		aria-label={common_tags()}
		class="flex items-center gap-1 rounded-sm"
	>
		{#if items.length > 0}
			<Tag
				label={items[0].label}
				color={items[0].color}
				icon={items[0].icon}
				badge={items[0].badge}
				title={items[0].title}
			/>
			{#if items.length > 1}
				<span class="text-xs text-tertiary">+{items.length - 1}</span>
			{/if}
		{:else}
			<span class="rounded-sm border border-dashed border-(--color-border) px-1.5 text-tertiary">
				<Plus class="h-3.5 w-3.5" />
			</span>
		{/if}
	</button>
	<Popover
		triggerElement={trigger ?? null}
		isOpen={pickerOpen}
		onClose={() => (pickerOpen = false)}
		role="dialog"
		ariaLabel={common_tags()}
		maxWidth="20rem"
	>
		<div class="p-2">
			<TagPickerInline selectedTagIds={tagIds} {entityId} {entityType} />
		</div>
	</Popover>
{:else if editable && entityType}
	<TagPickerInline selectedTagIds={tagIds} {entityId} {entityType} />
{:else}
	<div class="flex flex-wrap items-center gap-1">
		{#each visible as item (item.id)}
			<Tag
				label={item.label}
				color={item.color}
				icon={item.icon}
				badge={item.badge}
				title={item.title}
			/>
		{/each}

		{#if overflow > 0}
			<button
				type="button"
				onclick={() => (showAll = true)}
				class="text-xs text-tertiary transition-colors hover:text-secondary"
			>
				{common_moreItems({ count: overflow })}
			</button>
		{/if}
	</div>
{/if}
