<script lang="ts">
	import { Lock } from '@lucide/svelte';
	import { tooltip } from '#lib/shared/actions/tooltip.ts';

	let {
		label,
		description,
		locked = false,
		disabled = false,
		disabledTooltip
	}: {
		label: string;
		description?: string;
		locked?: boolean;
		disabled?: boolean;
		disabledTooltip?: string;
	} = $props();

	// What the rule does, then, set apart, why it does nothing in this view.
	let content = $derived(
		description || disabledTooltip
			? {
					sections: [description, disabledTooltip]
						.filter((text): text is string => !!text)
						.map((text) => ({ text }))
				}
			: null
	);
</script>

<div class="flex items-center gap-2" class:opacity-40={disabled} use:tooltip={{ content }}>
	<span class="text-sm" class:text-primary={!disabled} class:text-tertiary={disabled}>{label}</span>
	{#if locked}
		<Lock class="h-3 w-3 text-tertiary" />
	{/if}
</div>
