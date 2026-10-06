<script lang="ts">
	import Tag from './Tag.svelte';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import type { EntityDiscriminants } from '$lib/features/tags/queries';
	import type { CardFieldItem } from './types';
	import { MAX_ITEMS_IN_CELL } from './types';
	import { common_moreItems } from '$lib/paraglide/messages';

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

	let visible = $derived(showAll ? items : items.slice(0, MAX_ITEMS_IN_CELL));
	let overflow = $derived(items.length - visible.length);
</script>

{#if editable && entityType}
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
				class="text-tertiary hover:text-secondary text-xs transition-colors"
			>
				{common_moreItems({ count: overflow })}
			</button>
		{/if}
	</div>
{/if}
