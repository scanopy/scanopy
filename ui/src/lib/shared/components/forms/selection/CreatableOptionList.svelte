<script lang="ts" module>
	import type { IconComponent } from '#lib/shared/utils/types.ts';

	export interface CreatableOption {
		id: string;
		label: string;
		/** A colour swatch drawn before the label (CSS colour). */
		swatch?: string;
		/** An icon drawn before the label, in place of a swatch. */
		icon?: IconComponent | null;
		/**
		 * The heading the option is listed under. Options arrive already ordered by group; when any
		 * has one, a heading starts each run and options with none are listed as ungrouped.
		 */
		group?: string | null;
	}

	/** `highlighted` value for the create row. */
	export const CREATE_ROW = '__create__';
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Plus } from '@lucide/svelte';
	import KbdKey from '#lib/shared/components/feedback/KbdKey.svelte';
	import { keyLabel } from '#lib/shared/utils/shortcuts.ts';
	import { common_ungrouped } from '#lib/paraglide/messages.js';

	/**
	 * The rows of a type-to-filter dropdown that can also create what was typed: an optional
	 * "+ Create" row, then the matching options. The caller owns the input, the filtering and
	 * where the list floats; this draws the rows so every such dropdown reads the same.
	 */
	let {
		options,
		onSelect,
		createLabel = null,
		onCreate,
		creating = false,
		createOnEnter = false,
		highlighted = null,
		row = undefined
	}: {
		options: CreatableOption[];
		onSelect: (id: string) => void;
		/** Label of the create row, or null to hide it. */
		createLabel?: string | null;
		onCreate?: () => void;
		creating?: boolean;
		/** Enter in the caller's input creates; shows the Enter key on the create row. */
		createOnEnter?: boolean;
		/**
		 * The row the caller's arrow keys have reached: an option id, or `CREATE_ROW` for the create
		 * row. It is drawn as hovered and kept scrolled into view.
		 */
		highlighted?: string | null;
		/** Draws an option's row content in place of the swatch or icon and label (tags draw the tag). */
		row?: Snippet<[CreatableOption]>;
	} = $props();

	let grouped = $derived(options.some((option) => option.group));

	/** The heading above option `index`, or null when it continues the previous option's group. */
	function headingAt(index: number): string | null {
		if (!grouped) return null;
		const group = options[index].group ?? null;
		if (index > 0 && (options[index - 1].group ?? null) === group) return null;
		return group ?? common_ungrouped();
	}

	const ACTIVE = 'bg-gray-100 dark:bg-gray-600';

	/**
	 * Keeps the highlighted row in view as the arrow keys move it. With group headings, each row
	 * keeps `scroll-mt-9` above it so the sticky heading (36px) never covers it.
	 */
	function revealWhen(node: HTMLElement, active: boolean) {
		const reveal = (on: boolean) => on && node.scrollIntoView({ block: 'nearest' });
		reveal(active);
		return { update: reveal };
	}
</script>

{#if createLabel}
	<button
		type="button"
		use:revealWhen={highlighted === CREATE_ROW}
		class="select-option flex w-full items-center gap-2 border-b px-3 py-2 text-left text-xs transition-colors {highlighted ===
		CREATE_ROW
			? ACTIVE
			: ''}"
		style="border-color: var(--color-border)"
		onmousedown={() => onCreate?.()}
		disabled={creating}
	>
		<Plus class="h-3 w-3 shrink-0 text-green-400" />
		<span class="text-primary">{createLabel}</span>
		{#if createOnEnter}
			<KbdKey key={keyLabel('Enter')} size="sm" class="ml-auto" />
		{/if}
	</button>
{/if}

{#each options as option, index (option.id)}
	{@const heading = headingAt(index)}
	{#if heading}
		<div
			class="sticky top-0 border-b px-3 py-2 text-xs font-semibold tracking-wide text-secondary uppercase"
			style="border-color: var(--color-border); background: var(--color-bg-surface)"
		>
			{heading}
		</div>
	{/if}
	<button
		type="button"
		use:revealWhen={highlighted === option.id}
		class="select-option flex w-full items-center gap-2 px-3 py-2 text-left text-xs transition-colors {grouped
			? 'scroll-mt-9'
			: ''} {highlighted === option.id ? ACTIVE : ''}"
		onmousedown={() => onSelect(option.id)}
	>
		{#if row}
			{@render row(option)}
		{:else}
			{#if option.icon}
				<option.icon class="h-3 w-3 shrink-0 text-secondary" />
			{:else if option.swatch}
				<span class="h-2.5 w-2.5 shrink-0 rounded-full" style="background-color: {option.swatch};"
				></span>
			{/if}
			<span class="text-primary">{option.label}</span>
		{/if}
	</button>
{/each}
