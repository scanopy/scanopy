<script lang="ts">
	import Tag from '#lib/shared/components/data/Tag.svelte';
	import EntityTag from '#lib/shared/components/data/EntityTag.svelte';
	import { MAX_ITEMS_IN_CELL, entityRef } from '#lib/shared/components/data/types.ts';
	import { daemonHealths, entities } from '#lib/shared/stores/metadata.ts';
	import { common_moreItems } from '#lib/paraglide/messages.js';
	import type { SetupRowView } from '../setup-row-view';
	import type { SetupItem } from '../setup-list';

	let {
		item,
		view,
		done = false
	}: {
		item: SetupItem;
		view: SetupRowView;
		/** A done step shows its title alone. */
		done?: boolean;
	} = $props();

	let showAll = $state(false);
	let visible = $derived(showAll ? view.subjects : view.subjects.slice(0, MAX_ITEMS_IN_CELL));
	let overflow = $derived(view.subjects.length - visible.length);
</script>

<!-- The title names the problem; the chips beside it name what it's about, one entity each.
     Chips sit above any click target around them, so they hover and open on their own. -->
<div class="space-y-1">
	<div class="flex flex-wrap items-center gap-x-2 gap-y-1">
		<p class="text-sm font-medium {done ? 'text-secondary' : 'text-primary'}">{view.title}</p>
		{#if !done}
			{#each visible as subject (`${subject.type}:${subject.id}`)}
				<span class="pointer-events-auto relative z-10">
					<EntityTag
						entityRef={entityRef(
							subject.type,
							subject.id,
							subject.data ?? { id: subject.id, name: subject.label }
						)}
						label={subject.label}
						color={entities.getColorHelper(subject.type).color}
					/>
				</span>
			{/each}
			{#if overflow > 0}
				<button
					type="button"
					onclick={() => (showAll = true)}
					class="pointer-events-auto relative z-10 text-xs text-tertiary transition-colors hover:text-secondary"
				>
					{common_moreItems({ count: overflow })}
				</button>
			{/if}
			<!-- Outdated's sentence already says a newer version is available, so its tag would
			     repeat it. -->
			{#if item.type === 'DaemonHealth' && item.health !== 'outdated'}
				<Tag
					{...daemonHealths.getTag(item.health)}
					icon={daemonHealths.getIconComponent(item.health)}
				/>
			{/if}
		{/if}
	</div>
	{#if !done && view.sentence}
		<p class="text-sm text-tertiary">{view.sentence}</p>
	{/if}
</div>
