<script lang="ts">
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import EntityTag from '$lib/shared/components/data/EntityTag.svelte';
	import { entityRef } from '$lib/shared/components/data/types';
	import { daemonHealths, entities } from '$lib/shared/stores/metadata';
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
</script>

<!-- The title names the problem; the chips beside it name what it's about. Chips sit above any
     click target around them, so they hover and open on their own. -->
<div class="space-y-1">
	<div class="flex flex-wrap items-center gap-x-2 gap-y-1">
		<p class="text-sm font-medium {done ? 'text-secondary' : 'text-primary'}">{view.title}</p>
		{#if !done}
			{#each view.subjects as subject (subject.id)}
				<span class="pointer-events-auto relative z-10">
					<EntityTag
						entityRef={entityRef(
							subject.type,
							subject.id,
							subject.data ?? { id: subject.id, name: subject.label }
						)}
						label={subject.label}
						color={entities.getColorHelper(subject.type).color}
						badge={subject.badge}
					/>
				</span>
			{/each}
			{#if item.type === 'DaemonHealth'}
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
