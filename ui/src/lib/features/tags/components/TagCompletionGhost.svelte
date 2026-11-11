<!--
	The tag that typed text completes to, shown faded beside the input with a Tab key chip. The
	input's owner handles Tab; this handles the click. It never takes focus, so the input keeps it.
-->
<script lang="ts">
	import Tag from '#lib/shared/components/data/Tag.svelte';
	import KbdKey from '#lib/shared/components/feedback/KbdKey.svelte';
	import { keyLabel } from '#lib/shared/utils/shortcuts.ts';
	import { isApplicationTag, tagIcon } from '#lib/features/tags/groups.ts';
	import type { Tag as TagType } from '#lib/features/tags/types/base.ts';

	let {
		tag,
		applicationGroup,
		onAccept
	}: {
		tag: TagType;
		applicationGroup: string | null;
		onAccept: () => void;
	} = $props();
</script>

<button
	type="button"
	tabindex="-1"
	class="pointer-events-auto ml-1.5 inline-flex items-center gap-1.5 opacity-60 transition-opacity hover:opacity-100"
	onmousedown={(event) => event.preventDefault()}
	onclick={(event) => {
		event.stopPropagation();
		onAccept();
	}}
>
	<Tag
		label={tag.name}
		color={tag.color}
		icon={tagIcon(tag, applicationGroup)}
		isShiny={isApplicationTag(tag, applicationGroup)}
	/>
	<KbdKey key={keyLabel('Tab')} size="sm" />
</button>
