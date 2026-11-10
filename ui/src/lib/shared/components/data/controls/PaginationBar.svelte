<script lang="ts">
	import type { Snippet } from 'svelte';
	import {
		ChevronLeft,
		ChevronRight,
		Download,
		FunnelX,
		SquareDashed,
		Ungroup,
		X
	} from '@lucide/svelte';
	import { PAGE_SIZE_OPTIONS, type PageSizeOption } from '../types';
	import { isShiftShortcut, shiftShortcutLabel } from '$lib/shared/utils/shortcuts';
	import { isFreeKeydown, useListShortcuts } from './listShortcuts';
	import KbdKey from '$lib/shared/components/feedback/KbdKey.svelte';
	import {
		common_noItems,
		common_showingRange,
		common_showingTotal,
		common_item,
		common_items,
		common_previousPage,
		common_nextPage,
		common_perPage,
		common_clearFilter,
		common_clearFilters,
		common_ungroupColumn,
		common_clearSelection,
		common_itemsSelected,
		common_shortcutsPausedInSearch,
		common_export,
		common_exporting
	} from '$lib/paraglide/messages';

	let {
		totalCount,
		totalPages,
		pageSize,
		showingStart,
		showingEnd,
		canGoPrev,
		canGoNext,
		groupCount,
		groupLabel = null,
		onPrevPage,
		onNextPage,
		onPageSizeChange,
		filterCount = 0,
		onClearFilters = undefined,
		onClearGrouping = undefined,
		selectedCount = 0,
		onClearSelection = undefined,
		clearShortcuts = {},
		shortcutsPaused = false,
		pageShortcuts = {},
		columnMenu = undefined,
		onExport = null
	}: {
		totalCount: number;
		totalPages: number;
		pageSize: PageSizeOption;
		showingStart: number;
		showingEnd: number;
		canGoPrev: boolean;
		canGoNext: boolean;
		groupCount: number | null;
		/** The column the list groups by, which the ungroup button names. */
		groupLabel?: string | null;
		onPrevPage: () => void;
		onNextPage: () => void;
		onPageSizeChange: (size: PageSizeOption) => void;
		/** Filters set from the column headers; shown beside the count with a way to clear them. */
		filterCount?: number;
		onClearFilters?: () => void;
		/** Shown beside the group count. */
		onClearGrouping?: () => void;
		/** Rows checked for a bulk action; shown with the other clears. */
		selectedCount?: number;
		onClearSelection?: () => void;
		/** Key chips for the clears, as their shortcuts are shown. The caller handles the keys. */
		clearShortcuts?: { filters?: string; grouping?: string; selection?: string };
		/** The shortcuts are off (the table search has focus); their keys dim to say so. */
		shortcutsPaused?: boolean;
		/** Keys for the previous and next page, named in their tooltips. The caller handles them. */
		pageShortcuts?: { prev?: string; next?: string };
		/** The column visibility menu, right-aligned with the export and paging controls. */
		columnMenu?: Snippet;
		/** Export handler; the button shows only when there is one. */
		onExport?: (() => void | Promise<void>) | null;
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

	// Shift+E exports while this list is on screen, like the other Shift clears in this row.
	const EXPORT_KEY = 'E';
	const exportShortcut = shiftShortcutLabel(EXPORT_KEY);
	const listShortcuts = useListShortcuts();

	function handleKeydown(event: KeyboardEvent) {
		if (
			!onExport ||
			!listShortcuts?.isActive() ||
			!isShiftShortcut(event, EXPORT_KEY) ||
			!isFreeKeydown(event)
		) {
			return;
		}
		event.preventDefault();
		runExport();
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- A key in parentheses after a compact control's label: a key chip doesn't fit these buttons. -->
{#snippet keyHint(shortcut: string | undefined)}
	{#if shortcut}
		<span
			class="font-normal text-tertiary transition-opacity max-md:hidden {shortcutsPaused
				? 'opacity-40'
				: ''}"
			title={shortcutsPaused ? common_shortcutsPausedInSearch() : undefined}>({shortcut})</span
		>
	{/if}
{/snippet}

<!-- Lucide has no dashed-box X, so Clear Selection draws a small X inside a dashed square. -->
{#snippet selectionIcon()}
	<span class="relative inline-flex h-3.5 w-3.5 shrink-0 items-center justify-center">
		<SquareDashed class="absolute inset-0 h-3.5 w-3.5" />
		<X class="h-2 w-2" strokeWidth={3} />
	</span>
{/snippet}

{#snippet filterIcon()}
	<FunnelX class="h-3.5 w-3.5 shrink-0" />
{/snippet}

{#snippet groupIcon()}
	<Ungroup class="h-3.5 w-3.5 shrink-0" />
{/snippet}

{#snippet clearButton(
	onclick: () => void,
	label: string,
	shortcut: string | undefined,
	icon: Snippet
)}
	<button type="button" class="summary-control btn-secondary" {onclick}>
		{@render icon()}
		<span class="max-md:sr-only">{label}</span>
		{@render keyHint(shortcut)}
	</button>
{/snippet}

<!--
	A page arrow. With a key, the button is that key's chip, so the control and its shortcut are
	one thing to click or press; without one, a plain chevron button.
-->
{#snippet pageButton(
	onclick: () => void,
	enabled: boolean,
	label: string,
	shortcut: string | undefined,
	direction: 'prev' | 'next'
)}
	{#if shortcut}
		<button
			type="button"
			{onclick}
			disabled={!enabled}
			class="group rounded-sm transition-opacity disabled:cursor-not-allowed disabled:opacity-40"
			aria-label={label}
			title={shortcutsPaused ? common_shortcutsPausedInSearch() : label}
		>
			<KbdKey
				key={shortcut}
				class="group-hover:group-enabled:border-gray-400 group-hover:group-enabled:text-gray-900 dark:group-hover:group-enabled:border-gray-500 dark:group-hover:group-enabled:text-gray-100"
			/>
		</button>
	{:else}
		<button
			type="button"
			{onclick}
			disabled={!enabled}
			class="summary-control btn-secondary disabled:cursor-not-allowed disabled:opacity-50"
			aria-label={label}
			title={label}
		>
			{#if direction === 'prev'}
				<ChevronLeft class="h-4 w-4" />
			{:else}
				<ChevronRight class="h-4 w-4" />
			{/if}
		</button>
	{/if}
{/snippet}

<div class="flex flex-wrap items-center justify-between gap-y-2 text-sm text-tertiary">
	<!-- Left: the table's own controls, then the clears for what narrows or shapes the view. -->
	<div class="flex items-center gap-2">
		{#if columnMenu}
			{@render columnMenu()}
		{/if}
		{#if onExport}
			<button
				type="button"
				onclick={runExport}
				disabled={isExporting}
				class="summary-control btn-secondary disabled:cursor-not-allowed disabled:opacity-50"
			>
				<Download class="h-3.5 w-3.5" />
				<span class="max-md:sr-only">{isExporting ? common_exporting() : common_export()}</span>
				{@render keyHint(exportShortcut)}
			</button>
		{/if}
		{#if selectedCount > 0 && onClearSelection}
			{@render clearButton(
				onClearSelection,
				common_clearSelection(),
				clearShortcuts.selection,
				selectionIcon
			)}
		{/if}
		{#if filterCount > 0 && onClearFilters}
			{@render clearButton(
				onClearFilters,
				filterCount === 1 ? common_clearFilter() : common_clearFilters(),
				clearShortcuts.filters,
				filterIcon
			)}
		{/if}
		{#if groupCount !== null && onClearGrouping}
			{@render clearButton(
				onClearGrouping,
				common_ungroupColumn({ column: groupLabel ?? '' }),
				clearShortcuts.grouping,
				groupIcon
			)}
		{/if}
	</div>
	<!--
		Right: the range on screen between the page arrows, which move it, then rows per page. The
		range already says where you are, so there is no separate "Page 1 of 4".
	-->
	<div class="flex items-center gap-2">
		{#if totalPages > 1}
			{@render pageButton(onPrevPage, canGoPrev, common_previousPage(), pageShortcuts.prev, 'prev')}
		{/if}
		<span class="text-xs text-secondary tabular-nums">
			{#if totalCount === 0}
				{common_noItems()}
			{:else if totalPages > 1}
				{common_showingRange({
					start: showingStart,
					end: showingEnd,
					total: totalCount,
					itemLabel: totalCount === 1 ? common_item() : common_items()
				})}
			{:else}
				{common_showingTotal({
					count: totalCount,
					total: totalCount,
					itemLabel: totalCount === 1 ? common_item() : common_items()
				})}
			{/if}
			{#if selectedCount > 0}
				<span class="text-tertiary"
					><span aria-hidden="true">·</span> {common_itemsSelected({ count: selectedCount })}</span
				>
			{/if}
		</span>
		{#if totalPages > 1}
			{@render pageButton(onNextPage, canGoNext, common_nextPage(), pageShortcuts.next, 'next')}
		{/if}
		<!-- Rows per page, once there are more rows than the smallest page holds. -->
		{#if totalCount > 20}
			<label class="ml-3 flex items-center gap-1.5 text-xs">
				<select
					value={pageSize}
					onchange={(e) => onPageSizeChange(parseInt(e.currentTarget.value) as PageSizeOption)}
					class="summary-control input-field mx-0 py-0! pr-6! pl-2!"
				>
					{#each PAGE_SIZE_OPTIONS as size (size)}
						<option value={size}>{size}</option>
					{/each}
				</select>
				{common_perPage()}
			</label>
		{/if}
	</div>
</div>
