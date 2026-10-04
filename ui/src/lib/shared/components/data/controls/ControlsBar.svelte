<script lang="ts" generics="T">
	import {
		Search,
		X,
		LayoutGrid,
		Table,
		CheckSquare,
		Square,
		Download,
		Filter,
		ArrowUpNarrowWide,
		ArrowDownWideNarrow
	} from 'lucide-svelte';
	import type { Snippet } from 'svelte';
	import { getFieldKey, type FieldConfig } from '../types';
	import type { SortState } from './sorting';
	import { DEFAULT_VIEW_MODE, type ViewMode } from './dataControlsStorage';
	import Tag from '../Tag.svelte';
	import {
		common_active,
		common_ascending,
		common_descending,
		common_deselectAll,
		common_export,
		common_exporting,
		common_groupByLabel,
		common_none,
		common_searchPlaceholder,
		common_selectAll,
		common_sortByLabel,
		common_switchToCardView,
		common_switchToTableView
	} from '$lib/paraglide/messages';

	let {
		searchQuery = $bindable(''),
		selectedGroupField = $bindable(null),
		sortState = $bindable({ field: null, direction: 'asc' }),
		viewMode = $bindable(DEFAULT_VIEW_MODE),
		showFilters = $bindable(false),
		canFilter,
		groupableFields,
		sortableFields,
		hasActiveFilters,
		hasActiveSearch,
		hasActiveGrouping,
		showSelectAll,
		allSelected,
		onToggleSort,
		onClearSearch,
		onClearGrouping,
		onSelectAll,
		onSelectNone,
		onExport,
		/** Column menu, rendered only in table mode. */
		columnMenu,
		/** Table mode's stand-in for the filter, group and sort controls. */
		tableControls = undefined
	}: {
		searchQuery: string;
		selectedGroupField: string | null;
		sortState: SortState;
		viewMode: ViewMode;
		showFilters: boolean;
		/** Some rendered column has a filter, so the pane has something to show. */
		canFilter: boolean;
		groupableFields: FieldConfig<T>[];
		sortableFields: FieldConfig<T>[];
		hasActiveFilters: boolean;
		hasActiveSearch: boolean;
		hasActiveGrouping: boolean;
		showSelectAll: boolean;
		allSelected: boolean;
		onToggleSort: (fieldKey: string) => void;
		onClearSearch: () => void;
		onClearGrouping: () => void;
		onSelectAll: () => void;
		onSelectNone: () => void;
		/** Export handler; the button shows only when there is one. */
		onExport: (() => void | Promise<void>) | null;
		columnMenu?: Snippet;
		tableControls?: Snippet;
	} = $props();

	let isExporting = $state(false);

	async function runExport() {
		if (!onExport || isExporting) return;
		isExporting = true;
		try {
			await onExport();
		} finally {
			isExporting = false;
		}
	}
</script>

<!--
	flex-wrap plus min-w-0: a flex item defaults to `min-width: auto`, which resolves to its
	min-content size, so neither group could shrink and the row stayed wider than the panel below
	about 1160px of viewport. That overflow reached `main` and left it scrollable sideways into
	blank space. The left group shrinks (the search keeps its own `min-w-48` floor) and the right
	group holds its size and wraps to a second line rather than squashing its buttons.
-->
<div class="flex flex-wrap items-end justify-between gap-y-3">
	<!-- Left: Search + Filter/Group/Sort -->
	<div class="flex min-w-0 items-end gap-4">
		<!-- Search Input -->
		<div class="relative w-96 min-w-48">
			<Search class="text-tertiary absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2" />
			<input
				type="text"
				bind:value={searchQuery}
				placeholder={common_searchPlaceholder()}
				class="input-field w-full pl-10 pr-10"
			/>
			{#if hasActiveSearch}
				<button
					onclick={onClearSearch}
					class="text-tertiary hover:text-secondary absolute right-3 top-1/2 -translate-y-1/2 transition-colors"
				>
					<X class="h-4 w-4" />
				</button>
			{/if}
		</div>

		<!--
			Card view only. The table puts each column's filter and group control in
			its header and sorts on a header click, so these would be a second copy.
		-->
		{#if viewMode === 'table'}
			{#if tableControls}
				{@render tableControls()}
			{/if}
		{:else}
			<div class="flex items-end gap-3">
				<!-- Filter Toggle -->
				{#if canFilter}
					<button
						onclick={() => (showFilters = !showFilters)}
						class="btn-secondary flex h-[42px] items-center gap-2"
					>
						<Filter class="h-4 w-4" />
						{#if hasActiveFilters}
							<Tag label={common_active()} color="Blue" />
						{/if}
					</button>
				{/if}

				<!-- Group By Dropdown -->
				{#if groupableFields.length > 0}
					<div class="flex flex-col gap-1">
						<span class="text-tertiary text-xs">{common_groupByLabel()}</span>
						<div class="relative">
							<select bind:value={selectedGroupField} class="input-secondary pr-8">
								<option value={null}>{common_none()}</option>
								{#each groupableFields as field (getFieldKey(field))}
									<option value={getFieldKey(field)}>{field.label}</option>
								{/each}
							</select>
							{#if hasActiveGrouping}
								<button
									onclick={onClearGrouping}
									class="text-tertiary hover:text-secondary absolute right-8 top-1/2 -translate-y-1/2 transition-colors"
								>
									<X class="h-3 w-3" />
								</button>
							{/if}
						</div>
					</div>
				{/if}

				<!-- Sort Dropdown + Direction -->
				{#if sortableFields.length > 0}
					<div class="flex flex-col gap-1">
						<span class="text-tertiary text-xs">{common_sortByLabel()}</span>
						<div class="flex items-center gap-1">
							<select
								bind:value={sortState.field}
								onchange={() => {
									if (!sortState.field) sortState = { ...sortState, direction: 'asc' };
								}}
								class="input-secondary pr-8"
							>
								<option value={null}>{common_none()}</option>
								{#each sortableFields as field (getFieldKey(field))}
									<option value={getFieldKey(field)}>{field.label}</option>
								{/each}
							</select>
							{#if sortState.field}
								<button
									onclick={() => onToggleSort(sortState.field || '')}
									class="btn-secondary h-[42px]"
									title={sortState.direction === 'asc' ? common_ascending() : common_descending()}
								>
									{#if sortState.direction === 'asc'}
										<ArrowUpNarrowWide class="h-5 w-5" />
									{:else}
										<ArrowDownWideNarrow class="h-5 w-5" />
									{/if}
								</button>
							{/if}
						</div>
					</div>
				{/if}
			</div>
		{/if}
	</div>

	<!-- Right: View & Actions Group -->
	<div class="flex flex-shrink-0 items-end gap-2">
		<!-- View Mode Toggle -->
		<button
			onclick={() => (viewMode = viewMode === 'card' ? 'table' : 'card')}
			class="btn-secondary h-[42px]"
			title={viewMode === 'card' ? common_switchToTableView() : common_switchToCardView()}
		>
			{#if viewMode === 'card'}
				<Table class="h-5 w-5" />
			{:else}
				<LayoutGrid class="h-5 w-5" />
			{/if}
		</button>

		<!--
			Shown in both views. It edits the one field set both views render, so
			gating it on the table made the card's fields look fixed when they are
			the same list.
		-->
		{#if columnMenu}
			{@render columnMenu()}
		{/if}

		<!-- Select All/None. Card view only: the table's header checkbox does this. -->
		{#if showSelectAll && viewMode === 'card'}
			<button
				onclick={allSelected ? onSelectNone : onSelectAll}
				class="btn-secondary h-[42px]"
				title={allSelected ? common_deselectAll() : common_selectAll()}
			>
				{#if allSelected}
					<Square class="h-5 w-5" />
				{:else}
					<CheckSquare class="h-5 w-5" />
				{/if}
			</button>
		{/if}

		<!-- Export Button -->
		{#if onExport}
			<button
				onclick={runExport}
				disabled={isExporting}
				class="btn-secondary h-[42px] disabled:cursor-not-allowed disabled:opacity-50"
				title={isExporting ? common_exporting() : common_export()}
			>
				<Download class="h-5 w-5" />
			</button>
		{/if}
	</div>
</div>
