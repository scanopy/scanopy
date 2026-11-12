<script lang="ts">
	import Tag from '#lib/shared/components/data/Tag.svelte';
	import { guestStates } from '#lib/shared/stores/metadata.ts';

	/** A container's or VM's run state at the last scan, as its runtime or hypervisor reported it. */
	let { state, pill = false }: { state: string; pill?: boolean } = $props();
</script>

<Tag {...guestStates.getTag(state)} icon={guestStates.getIconComponent(state)} {pill} />
