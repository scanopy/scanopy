<script module lang="ts">
	import { MediaQuery } from 'svelte/reactivity';

	// Below `md` a row's actions fold into one menu, so the row keeps room for its columns. One
	// query for every row.
	const narrowViewport = new MediaQuery('max-width: 767px');
</script>

<script lang="ts">
	import ActionMenu from '../controls/ActionMenu.svelte';
	import { tooltip } from '#lib/shared/actions/tooltip.ts';
	import {
		iconButtonClass,
		resolveTooltip,
		shownActions,
		toolbarButtonClass
	} from '../controls/actionShortcuts';
	import type { RowAction } from '../types';
	import { common_actions } from '#lib/paraglide/messages.js';

	/**
	 * A row's actions: icon buttons, each disabled with its reason as the tooltip when unavailable,
	 * or one `…` menu on a narrow viewport. Actions hidden in the row's state are left out.
	 */
	let { actions: rowActions }: { actions: RowAction[] } = $props();

	let actions = $derived(shownActions(rowActions));
</script>

{#if narrowViewport.current}
	{#if actions.length > 0}
		<ActionMenu {actions} label={common_actions()} />
	{/if}
{:else}
	<div class="flex items-center justify-end gap-1">
		{#each actions as action (action.label)}
			{@const tip = resolveTooltip(action) ?? action.label}
			{#if action.showLabel}
				<!-- A table whose rows have one action names it, so the row reads as its fix. -->
				<button
					type="button"
					onclick={action.onClick}
					disabled={!!action.unavailable}
					use:tooltip
					data-tooltip={resolveTooltip(action)}
					class="{toolbarButtonClass(
						action.tone
					)} flex items-center gap-1.5 px-2 py-1 text-xs whitespace-nowrap disabled:cursor-not-allowed disabled:opacity-50"
				>
					{action.label}
					<action.icon size={14} class={action.animation || ''} />
				</button>
			{:else}
				<!--
					The label floats in a tooltip rather than growing inside the
					button. An in-flow label has to span its neighbours to fit its
					text, which is what let one action cover the rest of the row.
				-->
				<button
					type="button"
					onclick={action.onClick}
					disabled={!!action.unavailable}
					use:tooltip
					data-tooltip={tip}
					aria-label={action.label}
					class="{iconButtonClass(action.tone)} disabled:cursor-not-allowed disabled:opacity-50"
				>
					<action.icon size={16} class={action.animation || ''} />
				</button>
			{/if}
		{/each}
	</div>
{/if}
