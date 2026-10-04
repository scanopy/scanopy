<!-- One tag from a display's `getTags`: an entity tag with its popover, a tag with its own
     handlers, or a plain tag with its tooltip. Static contexts render every tag plain. -->
<script lang="ts">
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import EntityTag from '$lib/shared/components/data/EntityTag.svelte';
	import type { TagProps } from '$lib/shared/components/data/types';

	let { tag, interactive }: { tag: TagProps; interactive: boolean } = $props();
</script>

{#if interactive && tag.entityRef}
	<EntityTag
		entityRef={tag.entityRef}
		label={tag.label}
		color={tag.color}
		icon={tag.icon ?? null}
	/>
{:else if interactive && (tag.onmouseenter || tag.onmouseleave || tag.onclick)}
	<button
		type="button"
		class="inline-flex cursor-pointer"
		onmouseenter={tag.onmouseenter}
		onmouseleave={tag.onmouseleave}
		onclick={tag.onclick}
	>
		<Tag
			label={tag.label}
			color={tag.color}
			pill={tag.pill}
			icon={tag.icon ?? null}
			href={tag.href ?? ''}
			title={tag.title ?? ''}
		/>
	</button>
{:else}
	<Tag
		label={tag.label}
		color={tag.color}
		pill={tag.pill}
		icon={tag.icon ?? null}
		href={tag.href ?? ''}
		title={tag.title ?? ''}
	/>
{/if}
