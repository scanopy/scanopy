<script lang="ts" generics="T">
	import {
		ArrowUpDown,
		ArrowUpNarrowWide,
		ArrowDownWideNarrow,
		ChevronDown,
		ChevronRight
	} from 'lucide-svelte';
	import { getCoreRowModel, type ColumnDef, type Row } from '@tanstack/table-core';
	import { createSvelteTable } from './createSvelteTable.svelte';
	import type { EntityColumn } from './columns';
	import FieldValue from '../FieldValue.svelte';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import { SvelteSet } from 'svelte/reactivity';
	import { getFieldValue } from '../controls/fieldValues';
	import type { SortState } from '../controls/sorting';
	import type { CardAction, GroupSlice } from '../types';
	import {
		common_actions,
		common_selectRow,
		common_sortByColumn,
		common_deselectAll,
		common_selectAllOnPage,
		common_groupTotalShowing
	} from '$lib/paraglide/messages';

	let {
		items,
		groups = null,
		columns,
		sortState,
		selectable,
		selectedIds,
		allSelected,
		someSelected,
		getItemId,
		getActions,
		caption,
		onToggleSort,
		onToggleRow,
		onToggleAll
	}: {
		/** Ungrouped rows. Null when the list is grouped. */
		items: T[] | null;
		/**
		 * Grouped rows, rendered as collapsible sections of one table rather than
		 * a table each — so every group shares the one header and the one set of
		 * column widths, which is what makes groups comparable.
		 */
		groups: { name: string; items: T[]; range: GroupSlice | null }[] | null;
		columns: EntityColumn<T>[];
		sortState: SortState;
		selectable: boolean;
		selectedIds: ReadonlySet<string>;
		allSelected: boolean;
		someSelected: boolean;
		getItemId: (item: T) => string;
		getActions: ((item: T) => CardAction[]) | null;
		caption: string;
		onToggleSort: (fieldKey: string) => void;
		onToggleRow: (id: string, selected: boolean) => void;
		onToggleAll: () => void;
	} = $props();

	/**
	 * Column identity is keyed on the field keys, not the array.
	 *
	 * Tabs build `fields` inside a `$derived` over several queries, so every
	 * refetch yields a fresh array. Rebuilding the column defs on identity would
	 * throw away the row model on each one; the value functions read the current
	 * columns through a closure instead, so cells stay live regardless.
	 */
	let columnsKey = $derived(columns.map((c) => c.id).join('\0'));
	let columnDefs = $derived.by<ColumnDef<T>[]>(() => {
		void columnsKey;
		return columns.map((column) => ({
			id: column.id,
			accessorFn: (row: T) => getFieldValue(row, column.field),
			enableSorting: column.sortable
		}));
	});

	/** Every row on the page, grouped or not — table-core sees one flat list. */
	let allRows = $derived(items ?? (groups ?? []).flatMap((group) => group.items));

	const collapsed = new SvelteSet<string>();

	function toggleGroup(name: string) {
		if (collapsed.has(name)) collapsed.delete(name);
		else collapsed.add(name);
	}

	const view = createSvelteTable<T>(() => ({
		get data() {
			return allRows;
		},
		get columns() {
			return columnDefs;
		},
		getCoreRowModel: getCoreRowModel(),
		// Rows arrive already filtered, sorted and paged. table-core is given no
		// row model that could reorder or drop one, so its view cannot drift from
		// what the controls produced.
		manualSorting: true,
		manualFiltering: true,
		manualPagination: true,
		getRowId: (row: T) => getItemId(row),
		state: {
			get sorting() {
				return sortState.field
					? [{ id: sortState.field, desc: sortState.direction === 'desc' }]
					: [];
			}
		}
	}));

	/**
	 * Rows keyed by group, so the body can emit a group header row followed by
	 * that group's rows while table-core still owns one row model for all of them.
	 */
	let rowsByGroup = $derived.by(() => {
		const rowById = new Map(view.rows.map((row) => [row.id, row]));
		return (groups ?? []).map((group) => ({
			...group,
			rows: group.items
				.map((item) => rowById.get(getItemId(item)))
				.filter((row) => row !== undefined)
		}));
	});

	/** Header checkbox, plus every data column, plus the actions column. */
	let spannedColumns = $derived(columns.length + (selectable ? 1 : 0) + (getActions ? 1 : 0));

	let byId = $derived(new Map(columns.map((c) => [c.id, c])));
	let primaryColumn = $derived(columns.find((c) => c.primary) ?? columns[0]);

	function ariaSort(columnId: string): 'ascending' | 'descending' | 'none' {
		if (sortState.field !== columnId) return 'none';
		return sortState.direction === 'asc' ? 'ascending' : 'descending';
	}

	/** Names a row's checkbox after the row, so a column of them isn't all "Select". */
	function rowLabel(item: T): string {
		if (!primaryColumn) return '';
		const value = getFieldValue(item, primaryColumn.field);
		return value === null || value === undefined ? '' : String(value);
	}
</script>

<!--
	A plain table, deliberately: `role="grid"` promises two-dimensional arrow-key
	navigation with managed focus, and claiming it without implementing it takes
	away the table-reading commands screen reader users already have.

	The tabindex is deliberate too. The lint rule assumes one on a non-interactive
	element is a mistake, but a wide table overflows horizontally and a scroll
	container that only answers to the mouse is a WCAG 2.1.1 failure.

	Note this wrapper is the containing block for `position: sticky`, so a sticky
	header here would offset from the wrapper rather than the viewport — which is
	why the header scrolls with the rows instead of pinning.

	`relative` is what makes it the containing block for *absolute* descendants as
	well, and that is load-bearing rather than tidy. `FieldValue` renders an
	absolutely positioned `sr-only` span for every empty cell, and `overflow` only
	clips descendants it is also the containing block for. Left static, those spans
	resolved against `main` — the next positioned ancestor — kept their static
	position out at the far columns of a wide table, and extended `main`'s scrollable
	width past its own box. The window did not scroll, `main` did, into blank space
	to the right of the content. Same escape the `relative` on `main` was added for
	vertically, one level down.
-->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="relative overflow-x-auto" tabindex="0" role="region" aria-label={caption}>
	<table class="w-full border-collapse text-sm">
		<caption class="sr-only">{caption}</caption>
		<thead>
			<tr>
				{#if selectable}
					<th scope="col" class="w-10 px-3 py-2">
						<input
							type="checkbox"
							checked={allSelected}
							indeterminate={someSelected}
							onchange={onToggleAll}
							aria-label={allSelected ? common_deselectAll() : common_selectAllOnPage()}
							class="checkbox-card h-4 w-4"
						/>
					</th>
				{/if}

				{#each view.headers as header (header.id)}
					{@const column = byId.get(header.column.id)}
					{#if column}
						<th
							scope="col"
							aria-sort={ariaSort(column.id)}
							style={column.width ? `width: ${column.width}px` : ''}
							class="text-secondary whitespace-nowrap px-3 py-2 text-xs font-medium {column.align ===
							'right'
								? 'text-right'
								: 'text-left'}"
						>
							{#if header.column.getCanSort()}
								<!--
									A real button: Enter and Space work natively, and the direction
									is announced through aria-sort rather than repeated in the name.
								-->
								<button
									type="button"
									onclick={() => onToggleSort(column.id)}
									aria-label={common_sortByColumn({ column: column.label })}
									class="hover:text-primary group inline-flex items-center gap-1 transition-colors"
								>
									<span>{column.label}</span>
									<!--
										Every sortable header carries an icon, so it reads as sortable
										before anyone clicks it. A header without one cannot sort.
									-->
									{#if sortState.field === column.id}
										{#if sortState.direction === 'asc'}
											<ArrowUpNarrowWide class="text-accent h-3.5 w-3.5" aria-hidden="true" />
										{:else}
											<ArrowDownWideNarrow class="text-accent h-3.5 w-3.5" aria-hidden="true" />
										{/if}
									{:else}
										<ArrowUpDown
											class="text-tertiary group-hover:text-secondary group-focus-visible:text-secondary h-3.5 w-3.5"
											aria-hidden="true"
										/>
									{/if}
								</button>
							{:else}
								<span>{column.label}</span>
							{/if}
						</th>
					{/if}
				{/each}

				{#if getActions}
					<!--
						Not sticky. Only the header was pinned while its cells were not, so
						on a wide table the last column's header scrolled underneath this
						one's opaque background and read as a missing header, while its
						cells stayed visible. Pinning the whole column is a bigger change
						than it looks — every cell needs the same background or content
						shows through — so both scroll together instead.
					-->
					<th scope="col" class="text-secondary px-3 py-2 text-right text-xs font-medium">
						{common_actions()}
					</th>
				{/if}
			</tr>
		</thead>

		<tbody>
			{#if groups}
				{#each rowsByGroup as group (group.name)}
					{@const isCollapsed = collapsed.has(group.name)}
					<tr class="border-t" style="border-color: var(--color-border)">
						<!--
							scope="colgroup": this heading names the rows beneath it rather
							than a column, so it is announced as the section it is.
						-->
						<th
							scope="colgroup"
							colspan={spannedColumns}
							class="bg-black/[0.03] px-3 py-2 text-left dark:bg-white/[0.03]"
						>
							<button
								type="button"
								onclick={() => toggleGroup(group.name)}
								aria-expanded={!isCollapsed}
								class="text-primary flex items-center gap-2 text-sm font-semibold"
							>
								{#if isCollapsed}
									<ChevronRight class="h-4 w-4" aria-hidden="true" />
								{:else}
									<ChevronDown class="h-4 w-4" aria-hidden="true" />
								{/if}
								<span>{group.name}</span>
								<span class="text-tertiary text-xs font-normal">
									{#if group.range}
										{common_groupTotalShowing({
											total: group.range.total,
											start: group.range.start,
											end: group.range.end
										})}
									{:else}
										({group.items.length})
									{/if}
								</span>
							</button>
						</th>
					</tr>

					{#if !isCollapsed}
						{#each group.rows as row (row.id)}
							{@render bodyRow(row)}
						{/each}
					{/if}
				{/each}
			{:else}
				{#each view.rows as row (row.id)}
					{@render bodyRow(row)}
				{/each}
			{/if}
		</tbody>
	</table>
</div>

{#snippet bodyRow(row: Row<T>)}
	{@const item = row.original}
	{@const itemId = getItemId(item)}
	{@const isSelected = selectedIds.has(itemId)}
	<tr
		class="border-t transition-colors {isSelected
			? 'bg-black/5 dark:bg-white/5'
			: 'hover:bg-black/[0.03] dark:hover:bg-white/[0.03]'}"
		style="border-color: var(--color-border)"
	>
		{#if selectable}
			<td class="w-10 px-3 py-2 align-middle">
				<input
					type="checkbox"
					checked={isSelected}
					onchange={(e) => onToggleRow(itemId, e.currentTarget.checked)}
					aria-label={common_selectRow({ name: rowLabel(item) })}
					class="checkbox-card h-4 w-4"
				/>
			</td>
		{/if}

		{#each view.headers as header (header.id)}
			{@const column = byId.get(header.column.id)}
			{#if column}
				{#if column.primary}
					<!-- Announces the row's identity before each cell when navigating across. -->
					<th
						scope="row"
						class="text-primary max-w-xs px-3 py-2 text-left align-middle font-medium"
					>
						<FieldValue {item} {column} />
					</th>
				{:else}
					<td
						class="max-w-xs px-3 py-2 align-middle {column.align === 'right' ? 'text-right' : ''}"
					>
						<FieldValue {item} {column} />
					</td>
				{/if}
			{/if}
		{/each}

		{#if getActions}
			{@const actions = getActions(item)}
			<td class="px-3 py-2 text-right align-middle">
				<div class="flex items-center justify-end gap-1">
					{#each actions as action (action.label)}
						{@const tip =
							typeof action.tooltip === 'function'
								? action.tooltip(!!action.disabled)
								: (action.tooltip ?? action.label)}
						<!--
									The label floats in a tooltip rather than growing inside the
									button. An in-flow label has to span its neighbours to fit its
									text, which is what let one action cover the rest of the row.
								-->
						<button
							type="button"
							onclick={action.onClick}
							disabled={action.disabled}
							use:tooltip
							data-tooltip={tip}
							aria-label={action.label}
							class="{action.class || 'btn-icon'} disabled:cursor-not-allowed disabled:opacity-50"
						>
							<action.icon size={16} class={action.animation || ''} />
						</button>
					{/each}
				</div>
			</td>
		{/if}
	</tr>
{/snippet}
