<script lang="ts">
	import EntityTag from './EntityTag.svelte';
	import Tag from './Tag.svelte';
	import type { TooltipChip } from '$lib/shared/actions/tooltip';

	/** A tooltip's value, drawn by the same components as anywhere else in the app. */
	let { chip }: { chip: TooltipChip } = $props();
</script>

{#if chip.type === 'Entity'}
	<EntityTag entityRef={chip.ref} label={chip.label} color={chip.color} disablePopover />
{:else}
	<Tag label={chip.label} color={chip.color} icon={chip.icon ?? null} />
{/if}
