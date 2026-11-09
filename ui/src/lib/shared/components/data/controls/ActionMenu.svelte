<script lang="ts">
	import { ChevronDown, MoreHorizontal } from '@lucide/svelte';
	import Popover from '../Popover.svelte';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import { resolveTooltip } from './actionShortcuts';
	import type { RowAction } from '../types';

	/**
	 * A list's actions folded into one menu, for a phone where a row of buttons doesn't fit: a
	 * row's actions behind a `…` button, or a selection's behind a labelled one. Unavailable
	 * actions stay in the menu, disabled, with their reason as the tooltip.
	 */
	let {
		actions,
		label,
		variant = 'icon'
	}: {
		actions: RowAction[];
		/** The trigger's text, or its accessible name when `variant` is `icon`. */
		label: string;
		variant?: 'icon' | 'button';
	} = $props();

	let open = $state(false);
	let trigger: HTMLButtonElement | undefined = $state();

	function run(action: RowAction) {
		open = false;
		action.onClick();
	}
</script>

<button
	type="button"
	bind:this={trigger}
	onclick={() => (open = !open)}
	aria-haspopup="menu"
	aria-expanded={open}
	aria-label={variant === 'icon' ? label : undefined}
	class={variant === 'icon' ? 'btn-icon' : 'btn-secondary toolbar-control flex items-center gap-2'}
>
	{#if variant === 'icon'}
		<MoreHorizontal size={16} />
	{:else}
		{label}
		<ChevronDown class="h-4 w-4 transition-transform {open ? 'rotate-180' : ''}" />
	{/if}
</button>

<Popover triggerElement={trigger ?? null} isOpen={open} onClose={() => (open = false)}>
	<div class="flex flex-col" role="menu" aria-label={label}>
		{#each actions as action (action.label)}
			<button
				type="button"
				role="menuitem"
				disabled={!!action.unavailable}
				use:tooltip
				data-tooltip={resolveTooltip(action)}
				onclick={() => run(action)}
				class="flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm transition-colors hover:bg-gray-100 disabled:cursor-not-allowed disabled:opacity-50 dark:hover:bg-gray-700 {action.tone ===
				'danger'
					? 'text-danger'
					: 'text-primary'}"
			>
				<action.icon size={16} class="shrink-0 {action.animation || ''}" />
				{action.label}
			</button>
		{/each}
	</div>
</Popover>
