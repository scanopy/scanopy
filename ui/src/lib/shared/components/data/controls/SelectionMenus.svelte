<script lang="ts">
	import { Tags } from '@lucide/svelte';
	import Popover from '../Popover.svelte';
	import ActionMenu from './ActionMenu.svelte';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import type { EntityDiscriminants } from '$lib/features/tags/queries';
	import type { RowAction } from '../types';
	import { common_actions, common_commonTags, common_tags } from '$lib/paraglide/messages';

	/**
	 * A selection's controls on a phone: its actions in one menu, and its tags behind one button,
	 * in place of the toolbar's row of buttons and the tag island floating over the table.
	 */
	let {
		actions,
		selectedCount,
		tagging = null
	}: {
		actions: RowAction[];
		selectedCount: number;
		/** Present when the viewer can tag the selection. */
		tagging?: {
			commonTags: string[];
			entityType: EntityDiscriminants;
			onAdd: (tagId: string) => void;
			onRemove: (tagId: string) => void;
		} | null;
	} = $props();

	let tagsOpen = $state(false);
	let tagsTrigger: HTMLButtonElement | undefined = $state();
	let tagsLabel = $derived(selectedCount > 1 ? common_commonTags() : common_tags());
</script>

{#if tagging}
	<button
		type="button"
		bind:this={tagsTrigger}
		onclick={() => (tagsOpen = !tagsOpen)}
		aria-haspopup="dialog"
		aria-expanded={tagsOpen}
		aria-label={tagsLabel}
		class="btn-secondary toolbar-control flex items-center gap-2"
	>
		<Tags class="h-4 w-4" />
	</button>
	<Popover
		triggerElement={tagsTrigger ?? null}
		isOpen={tagsOpen}
		onClose={() => (tagsOpen = false)}
		role="dialog"
		ariaLabel={tagsLabel}
		maxWidth="20rem"
	>
		<div data-keeps-selection class="space-y-2 p-2">
			<p class="text-sm text-secondary">{tagsLabel}</p>
			<TagPickerInline
				selectedTagIds={tagging.commonTags}
				entityType={tagging.entityType}
				onAdd={tagging.onAdd}
				onRemove={tagging.onRemove}
			/>
		</div>
	</Popover>
{/if}
{#if actions.length > 0}
	<ActionMenu {actions} label={common_actions()} variant="button" />
{/if}
