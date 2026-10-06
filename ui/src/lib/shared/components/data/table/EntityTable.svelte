<script lang="ts" generics="T">
	import {
		ArrowUpDown,
		ArrowUpNarrowWide,
		ArrowDownWideNarrow,
		ChevronDown,
		ChevronRight
	} from 'lucide-svelte';
	import {
		getCoreRowModel,
		type ColumnDef,
		type ColumnSizingInfoState,
		type Header,
		type Row,
		type Updater
	} from '@tanstack/table-core';
	import { createSvelteTable } from './createSvelteTable.svelte';
	import type { Snippet } from 'svelte';
	import {
		clampColumnWidth,
		columnWidth,
		MAX_COLUMN_WIDTH,
		MIN_COLUMN_WIDTH,
		UNSIZED_COLUMN_MAX,
		type EntityColumn
	} from './columns';
	import { displaySettings } from '$lib/shared/stores/display-settings.svelte';
	import FieldValue from '../FieldValue.svelte';
	import TreeGuides from '../TreeGuides.svelte';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import { getFieldValue } from '../controls/fieldValues';
	import type { RenderGroup, TreeEntry, TreeGuide, TreeSection } from '../controls/grouping';
	import type { SortState } from '../controls/sorting';
	import type { CardAction } from '../types';
	import {
		common_actions,
		common_selectRow,
		common_sortByColumn,
		common_deselectAll,
		common_selectAllOnPage,
		common_groupTotalShowing,
		common_resizeColumn
	} from '$lib/paraglide/messages';

	let {
		items,
		groups = null,
		collapsed,
		onToggleCollapse,
		columns,
		columnSizing,
		onColumnSizingChange,
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
		onToggleAll,
		headerControl = undefined
	}: {
		/** Ungrouped rows. Null when the list is grouped. */
		items: T[] | null;
		/**
		 * Grouped rows, rendered as collapsible sections of one table rather than
		 * a table each — so every group shares the one header and the one set of
		 * column widths, which is what makes groups comparable.
		 */
		groups: RenderGroup<T>[] | null;
		/** Keys of the collapsed groups and tree sections. Owned by the caller. */
		collapsed: ReadonlySet<string>;
		onToggleCollapse: (key: string) => void;
		columns: EntityColumn<T>[];
		/** Widths the user resized columns to, in px. Owned and persisted by the caller. */
		columnSizing: Record<string, number>;
		onColumnSizingChange: (sizing: Record<string, number>) => void;
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
		/**
		 * A column's filter and group control, rendered beside its label. Owned by
		 * the caller, which holds the filter state, so the table stays a renderer.
		 */
		headerControl?: Snippet<[EntityColumn<T>]>;
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
			enableSorting: column.sortable,
			minSize: MIN_COLUMN_WIDTH,
			maxSize: MAX_COLUMN_WIDTH
		}));
	});

	/**
	 * Drag progress table-core tracks while a column is being resized. Kept here
	 * rather than by the caller: it is meaningless once the pointer is released.
	 */
	let columnSizingInfo = $state<ColumnSizingInfoState>({
		startOffset: null,
		startSize: null,
		deltaOffset: null,
		deltaPercentage: null,
		isResizingColumn: false,
		columnSizingStart: []
	});

	function resolve<S>(updater: Updater<S>, old: S): S {
		return typeof updater === 'function' ? (updater as (old: S) => S)(old) : updater;
	}

	/** Every write goes through the resize bounds, whichever path produced it. */
	function setSizing(next: Record<string, number>) {
		const sizing: Record<string, number> = {};
		for (const [id, width] of Object.entries(next)) {
			const clamped = clampColumnWidth(width);
			if (clamped !== null) sizing[id] = clamped;
		}
		onColumnSizingChange(sizing);
	}

	/** Every row on the page, grouped or not — table-core sees one flat list. */
	let allRows = $derived(items ?? (groups ?? []).flatMap((group) => group.items));

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
		enableColumnResizing: true,
		columnResizeMode: 'onChange',
		onColumnSizingChange: (updater) => setSizing(resolve(updater, columnSizing)),
		onColumnSizingInfoChange: (updater) => {
			columnSizingInfo = resolve(updater, columnSizingInfo);
		},
		state: {
			get sorting() {
				return sortState.field
					? [{ id: sortState.field, desc: sortState.direction === 'desc' }]
					: [];
			},
			get columnSizing() {
				return columnSizing;
			},
			get columnSizingInfo() {
				return columnSizingInfo;
			}
		}
	}));

	/** Step, in px, for one arrow-key press on a resize handle; Shift takes four. */
	const RESIZE_STEP = 16;

	/**
	 * Start a drag from a resize handle.
	 *
	 * A column the user never resized has no size in table-core, so its
	 * `getSize()` would be the library's default rather than what is on screen,
	 * and the drag would jump. Seeding the rendered width first makes the drag
	 * start where the edge is. Reading `view.headers` again re-applies the table
	 * options, so the handler sees the seeded size.
	 */
	function startResize(event: MouseEvent | TouchEvent, header: Header<T, unknown>) {
		const th = (event.currentTarget as HTMLElement).closest('th');
		if (!th) return;
		event.preventDefault();
		if (columnSizing[header.column.id] === undefined) {
			setSizing({ ...columnSizing, [header.column.id]: th.offsetWidth });
		}
		const fresh = view.headers.find((h) => h.id === header.id) ?? header;
		fresh.getResizeHandler()(event);
	}

	function resizeByKey(event: KeyboardEvent, columnId: string) {
		const th = (event.currentTarget as HTMLElement).closest('th');
		const current = columnSizing[columnId] ?? th?.offsetWidth;
		if (current === undefined) return;
		const step = event.shiftKey ? RESIZE_STEP * 4 : RESIZE_STEP;

		if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
			event.preventDefault();
			const delta = event.key === 'ArrowLeft' ? -step : step;
			setSizing({ ...columnSizing, [columnId]: current + delta });
		} else if (event.key === 'Enter' || event.key === 'Delete' || event.key === 'Backspace') {
			event.preventDefault();
			resetWidth(columnId);
		}
	}

	/** Drop the user's width, so the column goes back to its default. */
	function resetWidth(columnId: string) {
		if (columnSizing[columnId] === undefined) return;
		const rest = { ...columnSizing };
		delete rest[columnId];
		onColumnSizingChange(rest);
	}

	/**
	 * Cap on a cell's content: the column width less its padding, or a fixed
	 * cap for a content-sized column.
	 *
	 * In an auto-layout table a block's `max-width` also caps its column's
	 * minimum width, so this is what lets a column be narrower than its widest
	 * chip or name. Nothing is clipped: chips and text truncate inside it, and
	 * popovers opened from a cell are not cut off.
	 */
	function contentMaxWidth(column: EntityColumn<T>): string {
		const width = columnWidth(column, columnSizing) ?? UNSIZED_COLUMN_MAX;
		return `max-width: calc(${width}px - 2 * var(--cell-px))`;
	}

	/**
	 * table-core's rows by id, so the body can emit group and section headers between rows while
	 * table-core still owns one row model for all of them.
	 */
	let rowById = $derived(new Map(view.rows.map((row) => [row.id, row])));

	/** Header checkbox, plus every data column, plus the actions column. */
	let spannedColumns = $derived(columns.length + (selectable ? 1 : 0) + (getActions ? 1 : 0));

	/**
	 * Where a tree section header's guides start, so its lines meet the guides drawn in the rows'
	 * primary cell, which sits after the checkbox column.
	 */
	let sectionGuideOffset = $derived(
		selectable ? 'calc(2.5rem + var(--cell-px))' : 'var(--cell-px)'
	);

	function entryKey(entry: TreeEntry<T>): string {
		return entry.type === 'row' ? getItemId(entry.item) : entry.key;
	}

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
<div
	class="entity-table relative overflow-x-auto"
	data-density={displaySettings.current.table_density}
	tabindex="0"
	role="region"
	aria-label={caption}
>
	<table class="w-full border-collapse text-sm">
		<caption class="sr-only">{caption}</caption>
		<thead>
			<tr>
				{#if selectable}
					<th scope="col" class="w-10 px-[var(--cell-px)] py-[var(--cell-py)]">
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
						{@const width = columnWidth(column, columnSizing)}
						<th
							scope="col"
							aria-sort={ariaSort(column.id)}
							style={width ? `width: ${width}px` : ''}
							class="text-secondary relative whitespace-nowrap px-[var(--cell-px)] py-[var(--cell-py)] text-xs font-medium {column.align ===
							'right'
								? 'text-right'
								: 'text-left'}"
						>
							<!--
								Capped only once the user sized the column, so a narrowed header
								truncates its label instead of holding the column open.
							-->
							<div
								class="inline-flex max-w-full items-center gap-1 {column.align === 'right'
									? 'flex-row-reverse'
									: ''}"
								style={columnSizing[column.id] !== undefined ? contentMaxWidth(column) : ''}
							>
								{@render headerLabel(column, header.column.getCanSort())}
								{#if headerControl}
									{@render headerControl(column)}
								{/if}
							</div>
							{@render resizeHandle(header, column)}
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
					<th
						scope="col"
						class="text-secondary px-[var(--cell-px)] py-[var(--cell-py)] text-right text-xs font-medium"
					>
						{common_actions()}
					</th>
				{/if}
			</tr>
		</thead>

		<tbody>
			{#if groups}
				{#each groups as group (group.key)}
					{@const isCollapsed = collapsed.has(group.key)}
					<tr class="border-t" style="border-color: var(--color-border)">
						<!--
							scope="colgroup": this heading names the rows beneath it rather
							than a column, so it is announced as the section it is.
						-->
						<th
							scope="colgroup"
							colspan={spannedColumns}
							class="bg-black/[0.03] px-[var(--cell-px)] py-[var(--cell-py)] text-left dark:bg-white/[0.03]"
						>
							<button
								type="button"
								onclick={() => onToggleCollapse(group.key)}
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
						{#if group.entries}
							{@render treeEntries(group.entries)}
						{:else}
							{#each group.items as item (getItemId(item))}
								{@const row = rowById.get(getItemId(item))}
								{#if row}
									{@render bodyRow(row, [])}
								{/if}
							{/each}
						{/if}
					{/if}
				{/each}
			{:else}
				{#each view.rows as row (row.id)}
					{@render bodyRow(row, [])}
				{/each}
			{/if}
		</tbody>
	</table>
</div>

{#snippet treeEntries(entries: TreeEntry<T>[])}
	{#each entries as entry (entryKey(entry))}
		{#if entry.type === 'row'}
			{@const row = rowById.get(getItemId(entry.item))}
			{#if row}
				{@render bodyRow(row, entry.guides)}
			{/if}
		{:else}
			{@render sectionHeader(entry)}
			{#if !collapsed.has(entry.key)}
				{@render treeEntries(entry.entries)}
			{/if}
		{/if}
	{/each}
{/snippet}

{#snippet sectionHeader(section: TreeSection<T>)}
	{@const isCollapsed = collapsed.has(section.key)}
	<tr class="border-t" style="border-color: var(--color-border)">
		<th
			scope="colgroup"
			colspan={spannedColumns}
			class="relative bg-black/[0.03] py-[var(--cell-py)] pr-[var(--cell-px)] text-left dark:bg-white/[0.03]"
			style="padding-left: calc({sectionGuideOffset} + {section.guides.length}rem)"
		>
			<TreeGuides guides={section.guides} offset={sectionGuideOffset} />
			<button
				type="button"
				onclick={() => onToggleCollapse(section.key)}
				aria-expanded={!isCollapsed}
				class="text-primary flex items-center gap-2 text-sm font-medium"
			>
				{#if isCollapsed}
					<ChevronRight class="h-4 w-4" aria-hidden="true" />
				{:else}
					<ChevronDown class="h-4 w-4" aria-hidden="true" />
				{/if}
				<span>{section.label}</span>
				<span class="text-tertiary text-xs font-normal">({section.count})</span>
			</button>
		</th>
	</tr>
{/snippet}

{#snippet bodyRow(row: Row<T>, guides: TreeGuide[])}
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
			<td class="w-10 px-[var(--cell-px)] py-[var(--cell-py)] align-middle">
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
						class="text-primary relative px-[var(--cell-px)] py-[var(--cell-py)] text-left align-middle font-medium"
					>
						{#if guides.length > 0}
							<TreeGuides {guides} offset="var(--cell-px)" />
							<div style="padding-left: {guides.length}rem">
								<div style={contentMaxWidth(column)}>
									<FieldValue {item} {column} />
								</div>
							</div>
						{:else}
							<div style={contentMaxWidth(column)}>
								<FieldValue {item} {column} />
							</div>
						{/if}
					</th>
				{:else}
					<td
						class="px-[var(--cell-px)] py-[var(--cell-py)] align-middle {column.align === 'right'
							? 'text-right'
							: ''}"
					>
						<div class={column.align === 'right' ? 'ml-auto' : ''} style={contentMaxWidth(column)}>
							<FieldValue {item} {column} />
						</div>
					</td>
				{/if}
			{/if}
		{/each}

		{#if getActions}
			{@const actions = getActions(item)}
			<td class="px-[var(--cell-px)] py-[var(--cell-py)] text-right align-middle">
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

{#snippet headerLabel(column: EntityColumn<T>, canSort: boolean)}
	{#if canSort}
		<!--
			A real button: Enter and Space work natively, and the direction
			is announced through aria-sort rather than repeated in the name.
		-->
		<button
			type="button"
			onclick={() => onToggleSort(column.id)}
			aria-label={common_sortByColumn({ column: column.label })}
			class="hover:text-primary group inline-flex min-w-0 items-center gap-1 transition-colors"
		>
			<span class="truncate">{column.label}</span>
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
		<span class="truncate">{column.label}</span>
	{/if}
{/snippet}

{#snippet resizeHandle(header: Header<T, unknown>, column: EntityColumn<T>)}
	{@const width = columnSizing[column.id]}
	<!--
		A focusable separator is the ARIA pattern for a splitter, so arrow keys
		resize and the current width is announced. Double-click, Enter or Delete
		drops the user's width and returns the column to its default.

		The lint rules below treat `separator` as non-interactive, which holds only
		for an unfocusable one; ARIA 1.2 makes a focusable separator a widget.
	-->
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div
		role="separator"
		aria-orientation="vertical"
		aria-label={common_resizeColumn({ column: column.label })}
		aria-valuemin={MIN_COLUMN_WIDTH}
		aria-valuemax={MAX_COLUMN_WIDTH}
		aria-valuenow={width}
		tabindex="0"
		class="group absolute inset-y-0 right-0 z-10 flex w-2 cursor-col-resize touch-none select-none justify-end focus:outline-none"
		onmousedown={(e) => startResize(e, header)}
		ontouchstart={(e) => startResize(e, header)}
		ondblclick={() => resetWidth(column.id)}
		onkeydown={(e) => resizeByKey(e, column.id)}
	>
		<span
			class="h-full w-0.5 transition-colors group-hover:bg-blue-500/60 group-focus-visible:bg-blue-500 {columnSizingInfo.isResizingColumn ===
			column.id
				? 'bg-blue-500'
				: ''}"
		></span>
	</div>
{/snippet}
