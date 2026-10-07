<script lang="ts" generics="T">
	import { Filter, Group } from 'lucide-svelte';
	import Popover from '../Popover.svelte';
	import FieldFilter from '../controls/FieldFilter.svelte';
	import { getFieldKey, type FieldConfig } from '../types';
	import { columnControlState, type ColumnControl } from '../controls/headerControls';
	import type { FilterState } from '../controls/filtering';
	import type { FilterActions } from '../controls/filterActions';
	import type { Tag as TagType } from '$lib/features/tags/types/base';
	import {
		common_clearFilter,
		common_clearGrouping,
		common_filterByColumn,
		common_groupByColumn
	} from '$lib/paraglide/messages';

	/**
	 * A column header's filter and group control: an icon button that opens a
	 * popover. Sits beside the sort button rather than inside it, so opening it
	 * never sorts. The filter body is `FieldFilter`. Every column shows the same
	 * filter icon; a column that only groups opens to just the grouping button.
	 */
	let {
		control,
		label,
		filterState,
		staleOnly,
		activeGroupField,
		allTags,
		getUniqueValues,
		filters,
		onToggleGroup
	}: {
		control: ColumnControl<T>;
		label: string;
		filterState: FilterState;
		staleOnly: boolean;
		activeGroupField: string | null;
		allTags: TagType[];
		getUniqueValues: (field: FieldConfig<T>) => string[];
		filters: FilterActions;
		onToggleGroup: (fieldKey: string) => void;
	} = $props();

	let isOpen = $state(false);
	let trigger: HTMLButtonElement | undefined = $state();

	let key = $derived(getFieldKey(control.field));
	let filtersHere = $derived(control.filter || control.stale);
	let name = $derived(
		filtersHere ? common_filterByColumn({ column: label }) : common_groupByColumn({ column: label })
	);
	let status = $derived(columnControlState(control, filterState, staleOnly, activeGroupField));
	let active = $derived(status.filtered || status.grouped);
</script>

<button
	type="button"
	bind:this={trigger}
	onclick={() => (isOpen = !isOpen)}
	aria-label={name}
	aria-haspopup="dialog"
	aria-expanded={isOpen}
	class="relative inline-flex h-5 w-5 shrink-0 items-center justify-center rounded transition-colors hover:bg-black/5 dark:hover:bg-white/10 {active
		? 'text-accent'
		: 'text-tertiary hover:text-secondary'}"
>
	<!-- One header control for every column; what the popover holds depends on the field. -->
	<Filter class="h-3.5 w-3.5" aria-hidden="true" />
	{#if active}
		<span
			class="absolute right-0 top-0 h-1.5 w-1.5 rounded-full bg-purple-600 dark:bg-purple-400"
			aria-hidden="true"
		></span>
	{/if}
</button>

<Popover
	triggerElement={trigger ?? null}
	{isOpen}
	role="dialog"
	ariaLabel={name}
	onClose={() => (isOpen = false)}
>
	<div class="w-64 space-y-3 p-1 text-left font-normal">
		{#if filtersHere}
			<FieldFilter
				field={control.field}
				filter={filterState[key]}
				{allTags}
				showStale={control.stale}
				{staleOnly}
				{getUniqueValues}
				onToggleBoolean={filters.toggleBoolean}
				onToggleString={filters.toggleString}
				onToggleTag={filters.toggleTag}
				onToggleStale={filters.toggleStale}
			/>
			{#if status.filtered}
				<button
					type="button"
					onclick={() => filters.clearField(key)}
					class="text-tertiary hover:text-secondary text-xs transition-colors"
				>
					{common_clearFilter()}
				</button>
			{/if}
		{/if}

		{#if control.group}
			<div class={filtersHere ? 'border-t pt-3' : ''} style="border-color: var(--color-border)">
				<button
					type="button"
					onclick={() => onToggleGroup(key)}
					aria-pressed={status.grouped}
					class="btn-secondary flex w-full items-center justify-center gap-2 text-sm"
				>
					<Group class="h-4 w-4" aria-hidden="true" />
					{status.grouped ? common_clearGrouping() : common_groupByColumn({ column: label })}
				</button>
			</div>
		{/if}
	</div>
</Popover>
