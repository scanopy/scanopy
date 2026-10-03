<script lang="ts" generics="T">
	import { getFieldKey, type FieldConfig } from '../types';
	import type { FilterState } from './filtering';
	import type { Tag as TagType } from '$lib/features/tags/types/base';
	import FieldFilter from './FieldFilter.svelte';
	import { common_filters, common_clearAll } from '$lib/paraglide/messages';

	let {
		fields,
		filterState,
		allTags,
		staleOnly,
		hasActiveFilters,
		/** Staleness is server-only, so the control appears only when the parent handles it. */
		showStaleFilter,
		getUniqueValues,
		onClearFilters,
		onToggleBoolean,
		onToggleString,
		onToggleTag,
		onToggleStale
	}: {
		/** The fields with a rendered column: a list filters by nothing else. */
		fields: FieldConfig<T>[];
		filterState: FilterState;
		allTags: TagType[];
		staleOnly: boolean;
		hasActiveFilters: boolean;
		showStaleFilter: boolean;
		getUniqueValues: (field: FieldConfig<T>) => string[];
		onClearFilters: () => void;
		onToggleBoolean: (fieldKey: string, which: 'showTrue' | 'showFalse') => void;
		onToggleString: (fieldKey: string, value: string) => void;
		onToggleTag: (tagId: string) => void;
		onToggleStale: () => void;
	} = $props();

	let filterFields = $derived(
		fields.filter((f) => f.filterable || (showStaleFilter && f.staleFilter))
	);
</script>

<div class="card mt-4 !rounded-lg !p-5">
	<div class="flex items-center justify-between">
		<h3 class="text-primary text-sm font-semibold">{common_filters()}</h3>
		{#if hasActiveFilters}
			<button
				onclick={onClearFilters}
				class="text-tertiary hover:text-secondary text-xs transition-colors"
			>
				{common_clearAll()}
			</button>
		{/if}
	</div>

	<div class="mt-4 grid grid-cols-1 gap-x-8 gap-y-5 md:grid-cols-2 lg:grid-cols-3">
		{#each filterFields as field (getFieldKey(field))}
			<FieldFilter
				{field}
				filter={filterState[getFieldKey(field)]}
				{allTags}
				showStale={showStaleFilter && field.staleFilter === true}
				{staleOnly}
				{getUniqueValues}
				{onToggleBoolean}
				{onToggleString}
				{onToggleTag}
				{onToggleStale}
			/>
		{/each}
	</div>
</div>
