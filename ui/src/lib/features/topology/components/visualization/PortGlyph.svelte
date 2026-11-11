<script lang="ts">
	import { entities } from '#lib/shared/stores/metadata.ts';

	/**
	 * A port: the Interface entity's icon in gray. Bare beside text; on an icon card, in a
	 * tile the size of a device's so a port card has the same shape as every other icon card.
	 */
	let {
		size = 'sm'
	}: {
		/** `sm` beside text, `lg` on an icon card. */
		size?: 'sm' | 'lg';
	} = $props();

	const Icon = entities.getIconComponent('Interface');
</script>

{#if size === 'lg'}
	<span
		class="inline-flex h-12 w-12 shrink-0 items-center justify-center rounded-lg border-2 border-gray-200 bg-gray-200 dark:border-gray-700 dark:bg-gray-700"
	>
		<Icon class="h-7 w-7 text-secondary" />
	</span>
{:else}
	<Icon class="h-4 w-4 shrink-0 text-secondary" />
{/if}
