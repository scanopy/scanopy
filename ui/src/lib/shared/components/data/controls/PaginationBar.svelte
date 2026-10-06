<script lang="ts">
	import { ChevronLeft, ChevronRight, FunnelX } from 'lucide-svelte';
	import { PAGE_SIZE_OPTIONS, type PageSizeOption } from '../types';
	import {
		common_noItems,
		common_showingRange,
		common_showingTotal,
		common_item,
		common_items,
		common_group,
		common_groups,
		common_show,
		common_previousPage,
		common_nextPage,
		common_pageOf,
		common_clearAll,
		common_filterApplied,
		common_nFiltersApplied
	} from '$lib/paraglide/messages';

	let {
		totalCount,
		totalPages,
		currentPage,
		pageSize,
		showingStart,
		showingEnd,
		canGoPrev,
		canGoNext,
		groupCount,
		/** Client-side lists report "N of M loaded"; server-side reports the server's total. */
		useServerPagination,
		processedCount,
		itemCount,
		onPrevPage,
		onNextPage,
		onPageSizeChange,
		filterCount = 0,
		onClearFilters = undefined
	}: {
		totalCount: number;
		totalPages: number;
		currentPage: number;
		pageSize: PageSizeOption;
		showingStart: number;
		showingEnd: number;
		canGoPrev: boolean;
		canGoNext: boolean;
		groupCount: number | null;
		useServerPagination: boolean;
		processedCount: number;
		itemCount: number;
		onPrevPage: () => void;
		onNextPage: () => void;
		onPageSizeChange: (size: PageSizeOption) => void;
		/** Filters set from the column headers; shown beside the count with a way to clear them. */
		filterCount?: number;
		onClearFilters?: () => void;
	} = $props();
</script>

<div class="text-tertiary flex items-center justify-between text-sm">
	<div class="flex items-center gap-2">
		<span>
			{#if totalCount === 0}
				{common_noItems()}
			{:else if totalPages > 1}
				{common_showingRange({
					start: showingStart,
					end: showingEnd,
					total: totalCount,
					itemLabel: totalCount === 1 ? common_item() : common_items()
				})}
			{:else if useServerPagination}
				{common_showingTotal({
					count: totalCount,
					total: totalCount,
					itemLabel: totalCount === 1 ? common_item() : common_items()
				})}
			{:else}
				{common_showingTotal({
					count: processedCount,
					total: itemCount,
					itemLabel: itemCount === 1 ? common_item() : common_items()
				})}
			{/if}
		</span>
		<!-- Same pattern as the topology options panel's filter clearing. -->
		{#if filterCount > 0}
			<span aria-hidden="true">·</span>
			<span
				>{filterCount === 1
					? common_filterApplied()
					: common_nFiltersApplied({ count: filterCount })}</span
			>
			{#if onClearFilters}
				<button
					type="button"
					class="btn-secondary shrink-0 gap-1 rounded px-1.5 py-0 text-xs font-medium"
					onclick={onClearFilters}
				>
					<FunnelX class="h-3 w-3" />
					{common_clearAll()}
				</button>
			{/if}
		{/if}
	</div>
	<div class="flex items-center gap-4">
		{#if groupCount !== null}
			<span>
				{groupCount}
				{groupCount === 1 ? common_group() : common_groups()}
			</span>
		{/if}
		<!-- Page size selector (only show when there are more than 20 items) -->
		{#if totalCount > 20}
			<div class="flex items-center gap-2">
				<span class="text-tertiary text-sm">{common_show()}</span>
				<select
					value={pageSize}
					onchange={(e) => onPageSizeChange(parseInt(e.currentTarget.value) as PageSizeOption)}
					class="input-field mx-0 py-1 pr-6"
				>
					{#each PAGE_SIZE_OPTIONS as size (size)}
						<option value={size}>{size}</option>
					{/each}
				</select>
			</div>
		{/if}
		{#if totalPages > 1}
			<div class="flex items-center gap-2">
				<button
					onclick={onPrevPage}
					disabled={!canGoPrev}
					class="btn-secondary p-1 disabled:cursor-not-allowed disabled:opacity-50"
					title={common_previousPage()}
				>
					<ChevronLeft class="h-5.5 w-5.5" />
				</button>
				<span class="text-secondary min-w-[80px] text-center">
					{common_pageOf({ current: currentPage, total: totalPages })}
				</span>
				<button
					onclick={onNextPage}
					disabled={!canGoNext}
					class="btn-secondary p-1 disabled:cursor-not-allowed disabled:opacity-50"
					title={common_nextPage()}
				>
					<ChevronRight class="h-5.5 w-5.5" />
				</button>
			</div>
		{/if}
	</div>
</div>
