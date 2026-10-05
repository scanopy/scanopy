<script lang="ts" module>
	import type { IconComponent } from '$lib/shared/utils/types';

	export interface CreatableOption {
		id: string;
		label: string;
		/** A colour swatch drawn before the label (CSS colour). */
		swatch?: string;
		/** An icon drawn before the label, in place of a swatch. */
		icon?: IconComponent | null;
	}
</script>

<script lang="ts">
	import { Plus } from 'lucide-svelte';

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
		creating = false
	}: {
		options: CreatableOption[];
		onSelect: (id: string) => void;
		/** Label of the create row, or null to hide it. */
		createLabel?: string | null;
		onCreate?: () => void;
		creating?: boolean;
	} = $props();
</script>

{#if createLabel}
	<button
		type="button"
		class="select-option flex w-full items-center gap-2 border-b px-3 py-2 text-left text-xs transition-colors"
		style="border-color: var(--color-border)"
		onmousedown={() => onCreate?.()}
		disabled={creating}
	>
		<Plus class="h-3 w-3 shrink-0 text-green-400" />
		<span class="text-primary">{createLabel}</span>
	</button>
{/if}

{#each options as option (option.id)}
	<button
		type="button"
		class="select-option flex w-full items-center gap-2 px-3 py-2 text-left text-xs transition-colors"
		onmousedown={() => onSelect(option.id)}
	>
		{#if option.icon}
			<option.icon class="text-secondary h-3 w-3 shrink-0" />
		{:else if option.swatch}
			<span class="h-2.5 w-2.5 shrink-0 rounded-full" style="background-color: {option.swatch};"
			></span>
		{/if}
		<span class="text-primary">{option.label}</span>
	</button>
{/each}
