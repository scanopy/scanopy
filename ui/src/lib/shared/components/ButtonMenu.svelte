<script lang="ts" module>
	export interface ButtonMenuItem {
		label: string;
		onclick: () => void;
		disabled?: boolean;
		/** 'danger' renders the item in the destructive color. */
		tone?: 'default' | 'danger';
	}

	export interface ButtonMenuSecondaryAction {
		label: string;
		onclick: () => void;
		disabled?: boolean;
	}
</script>

<script lang="ts">
	import { ChevronDown } from '@lucide/svelte';
	import type { IconComponent } from '#lib/shared/utils/types.ts';
	import Popover from '#lib/shared/components/data/Popover.svelte';
	import { common_moreActions } from '#lib/paraglide/messages.js';

	let {
		label,
		onclick,
		items = [],
		secondaryAction = undefined,
		icon = undefined,
		variant = 'primary',
		disabled = false,
		moreLabel = common_moreActions()
	}: {
		/** Primary action label. */
		label: string;
		/** Primary action handler. */
		onclick: () => void;
		/** Secondary actions revealed by the "More Actions" button. When empty,
		 *  that button is omitted and this is a plain full-width primary button. */
		items?: ButtonMenuItem[];
		/** Optional prominent secondary CTA rendered as a full-width button
		 *  directly under the primary (above the "More Actions" menu). */
		secondaryAction?: ButtonMenuSecondaryAction;
		/** Optional leading icon for the primary button. */
		icon?: IconComponent;
		variant?: 'primary' | 'secondary';
		disabled?: boolean;
		/** Label for the secondary menu trigger. */
		moreLabel?: string;
	} = $props();

	let isOpen = $state(false);
	let menuTrigger: HTMLButtonElement | undefined = $state();

	const Icon = $derived(icon);
	let hasMenu = $derived(items.length > 0);
	let primaryClass = $derived(variant === 'secondary' ? 'btn-secondary' : 'btn-primary');

	function runItem(item: ButtonMenuItem) {
		isOpen = false;
		item.onclick();
	}
</script>

<div class="flex w-full flex-col gap-2">
	<button type="button" {onclick} {disabled} class="{primaryClass} w-full">
		{#if Icon}
			<Icon size={14} />
		{/if}
		{label}
	</button>
	{#if secondaryAction}
		<button
			type="button"
			onclick={secondaryAction.onclick}
			disabled={secondaryAction.disabled}
			class="btn-secondary w-full"
		>
			{secondaryAction.label}
		</button>
	{/if}
	{#if hasMenu}
		<button
			bind:this={menuTrigger}
			type="button"
			onclick={() => (isOpen = !isOpen)}
			aria-haspopup="menu"
			aria-expanded={isOpen}
			class="inline-flex items-center gap-1 self-center text-sm text-link hover:underline"
		>
			{moreLabel}
			<ChevronDown class="h-4 w-4 transition-transform {isOpen ? 'rotate-180' : ''}" />
		</button>
	{/if}
</div>

{#if hasMenu}
	<Popover triggerElement={menuTrigger ?? null} {isOpen} onClose={() => (isOpen = false)}>
		<div class="flex flex-col" role="menu">
			{#each items as item (item.label)}
				<button
					type="button"
					role="menuitem"
					disabled={item.disabled}
					onclick={() => runItem(item)}
					class="flex w-full items-center rounded-md px-3 py-2 text-left text-sm transition-colors hover:bg-gray-100 disabled:cursor-not-allowed disabled:opacity-50 dark:hover:bg-gray-700 {item.tone ===
					'danger'
						? 'text-danger'
						: 'text-primary'}"
				>
					{item.label}
				</button>
			{/each}
		</div>
	</Popover>
{/if}
