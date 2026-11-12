<script lang="ts">
	import { topologyOptions, updateTopologyOptions } from '#lib/features/topology/queries.ts';
	import { tooltip } from '#lib/shared/actions/tooltip.ts';

	let {
		label,
		helpText = '',
		path,
		optionKey,
		disabled = false,
		disabledReason = ''
	}: {
		label: string;
		helpText?: string;
		path: 'local' | 'request';
		optionKey: string;
		disabled?: boolean;
		disabledReason?: string;
	} = $props();

	let checked = $derived(
		path === 'local'
			? !!($topologyOptions.local as Record<string, unknown>)[optionKey]
			: !!($topologyOptions.request as Record<string, unknown>)[optionKey]
	);

	function toggle() {
		if (disabled) return;
		updateTopologyOptions((opts) => {
			const updated = structuredClone(opts);
			if (path === 'local') {
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				(updated.local as any)[optionKey] = !checked;
			} else {
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				(updated.request as any)[optionKey] = !checked;
			}
			return updated;
		});
	}
</script>

<div use:tooltip data-tooltip={disabled && disabledReason ? disabledReason : null}>
	<label
		class="flex items-center gap-2 {disabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer'}"
	>
		<input type="checkbox" class="checkbox-card h-4 w-4" {checked} {disabled} onchange={toggle} />
		<span class="text-sm text-secondary">{label}</span>
	</label>
	{#if helpText}
		<p class="mt-1 ml-6 text-xs text-tertiary">{helpText}</p>
	{/if}
</div>
