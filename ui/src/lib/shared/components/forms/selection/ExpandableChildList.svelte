<script lang="ts" module>
	/** Per-child selection. When given, each child row toggles on click and carries a
	 *  right-aligned checkbox in the same column as a ListManager `checkbox` row's. */
	export interface ChildSelection<C> {
		isSelected: (child: C) => boolean;
		onToggle: (child: C) => void;
		/** A non-null reason disables the child's checkbox and shows as a tooltip. */
		disabledReason?: (child: C) => string | null | undefined;
	}
</script>

<!-- C: child item type, X: context passed to the child display -->
<script lang="ts" generics="C, X">
	import { ChevronDown, ChevronRight } from 'lucide-svelte';
	import type { Snippet } from 'svelte';
	import ListSelectItem from './ListSelectItem.svelte';
	import type { EntityDisplayComponent } from './types';
	import { tooltip } from '$lib/shared/actions/tooltip';

	interface Props {
		items: C[];
		displayComponent: EntityDisplayComponent<C, X>;
		getContext?: (child: C) => X;
		/** Text on the expand toggle, e.g. "4 services". */
		toggleLabel: string;
		/** Rendered beside the toggle — a count, a status. */
		badge?: Snippet;
		expanded?: boolean;
		onToggleExpanded?: (expanded: boolean) => void;
		selection?: ChildSelection<C>;
	}

	let {
		items,
		displayComponent,
		getContext = () => ({}) as X,
		toggleLabel,
		badge,
		expanded = $bindable(false),
		onToggleExpanded,
		selection
	}: Props = $props();

	// Sits inside a clickable ListManager row: the toggle and selectable children stop their
	// clicks before they reach (and select) the row.
	function toggle(e: MouseEvent) {
		e.stopPropagation();
		expanded = !expanded;
		onToggleExpanded?.(expanded);
	}
</script>

<div class="mt-2 w-full border-t border-gray-200 pt-2 dark:border-gray-700">
	<div class="flex items-center gap-2">
		<button type="button" class="text-tertiary flex items-center gap-1 text-xs" onclick={toggle}>
			{#if expanded}
				<ChevronDown class="h-3.5 w-3.5" />
			{:else}
				<ChevronRight class="h-3.5 w-3.5" />
			{/if}
			{toggleLabel}
		</button>
		{#if badge}
			{@render badge()}
		{/if}
	</div>
	{#if expanded}
		<div class="mt-1 divide-y divide-gray-200 dark:divide-gray-700/50">
			{#each items as child (displayComponent.getId(child))}
				{#if selection}
					{@const reason = selection.disabledReason?.(child) ?? null}
					<!-- The row bleeds 0.5rem past the panel on each side so its checkbox lands on the
					     panel's right edge, the same column as the parent row's checkbox. -->
					<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
					<span
						class="-mx-2 block rounded odd:bg-gray-50 dark:odd:bg-gray-800/30"
						data-tooltip={reason ?? undefined}
						use:tooltip
						onclick={(e) => e.stopPropagation()}
					>
						<label
							class="flex items-center gap-3 rounded py-1.5 pl-8 pr-2 {reason
								? 'cursor-not-allowed opacity-60'
								: 'child-row-interactive cursor-pointer'}"
						>
							<div class="min-w-0 flex-1">
								<ListSelectItem
									item={child}
									context={getContext(child)}
									{displayComponent}
									staticTags={true}
								/>
							</div>
							<input
								type="checkbox"
								class="checkbox-card h-4 w-4 flex-shrink-0"
								checked={selection.isSelected(child)}
								disabled={reason !== null}
								onchange={() => selection.onToggle(child)}
							/>
						</label>
					</span>
				{:else}
					<div class="rounded px-2 py-1.5 pl-6 odd:bg-gray-50 dark:odd:bg-gray-800/30">
						<ListSelectItem item={child} context={getContext(child)} {displayComponent} />
					</div>
				{/if}
			{/each}
		</div>
	{/if}
</div>

<style>
	/* Subtle, theme-aware hover: the surface-hover token rather than a fixed gray. */
	.child-row-interactive:hover {
		background: var(--color-bg-surface-hover);
	}
</style>
