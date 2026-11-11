<script lang="ts" generics="T extends RowData">
	import {
		ArrowUpDown,
		ArrowUpNarrowWide,
		ArrowDownWideNarrow,
		ChevronDown,
		ChevronRight
	} from '@lucide/svelte';
	import type {
		ColumnDef,
		columnResizingState,
		Header,
		Row,
		RowData,
		Updater
	} from '@tanstack/table-core';
	import { createSvelteTable, type EntityTableFeatures } from './createSvelteTable.svelte';
	import type { Snippet } from 'svelte';
	import { MediaQuery } from 'svelte/reactivity';
	import RowActions from './RowActions.svelte';
	import GroupHeading from './GroupHeading.svelte';
	import {
		clampColumnWidth,
		columnWidth,
		MAX_COLUMN_WIDTH,
		MIN_COLUMN_WIDTH,
		UNSIZED_COLUMN_MAX,
		type EntityColumn
	} from './columns';
	import { displaySettings } from '#lib/shared/stores/display-settings.svelte.ts';
	import FieldValue from '../FieldValue.svelte';
	import TreeGuides from '../TreeGuides.svelte';
	import { shortcutLabel } from '#lib/shared/utils/shortcuts.ts';
	import { getFieldValue } from '../controls/fieldValues';
	import type { RenderGroup, TreeCell, TreeContinuation, TreeEntry } from '../controls/grouping';
	import type { SortState } from '../controls/sorting';
	import type { RowAction, GroupSlice, TagProps } from '../types';
	import Tag from '../Tag.svelte';
	import {
		common_actions,
		common_selectRow,
		common_sortByColumn,
		common_deselectAll,
		common_selectAllOnPage,
		common_groupTotalShowing,
		common_resizeColumn,
		common_expand,
		common_collapse
	} from '#lib/paraglide/messages.js';

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
		getRowBadges = null,
		caption,
		onToggleSort,
		onClickRow,
		onToggleAll,
		headerControl = undefined,
		bottomInset = 0
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
		getActions: ((item: T) => RowAction[]) | null;
		/**
		 * Icon-only status badges drawn ahead of the row's name (Stale, Unconfirmed), whatever columns
		 * are shown. Each badge says what it means in its tooltip.
		 */
		getRowBadges?: ((item: T) => TagProps[]) | null;
		caption: string;
		onToggleSort: (fieldKey: string) => void;
		/**
		 * A click on a row or its checkbox. `extend` (Shift held) selects the block from the last
		 * clicked row; otherwise the row toggles.
		 */
		onClickRow: (id: string, extend: boolean) => void;
		onToggleAll: () => void;
		/**
		 * A column's filter and group control, rendered beside its label. Owned by
		 * the caller, which holds the filter state, so the table stays a renderer.
		 */
		headerControl?: Snippet<[EntityColumn<T>]>;
		/**
		 * Space kept below the last row, in px, for something floating over the table's bottom
		 * edge (the bulk-tag island), so the last row can scroll clear of it.
		 */
		bottomInset?: number;
	} = $props();

	/*
	 * From `md` up the table scrolls on its own, filling `main` from where it sits down to
	 * `main`'s bottom padding, and its header row is `position: sticky` at the top of it. The
	 * wrapper already scrolls sideways, which makes it the sticky containing block, so the header
	 * can only pin inside it: a header pinned to the page had to be moved by script on every
	 * scroll, and lagged the toolbar because the browser scrolls off the main thread. The height
	 * is measured on resize and tab switch, never on scroll. Below `md` the page scrolls and the
	 * header scrolls with it.
	 */
	const MAIN_BOTTOM_PADDING = 16;
	const fillsViewport = new MediaQuery('min-width: 48rem');
	let wrapperEl: HTMLDivElement | null = $state(null);
	let maxHeight = $state<number | null>(null);

	$effect(() => {
		const wrapper = wrapperEl;
		const main = wrapper?.closest('main');
		if (!wrapper || !main || !fillsViewport.current) {
			maxHeight = null;
			return;
		}
		const measure = () => {
			const mainRect = main.getBoundingClientRect();
			// Where the table sits in `main`'s content, whatever `main` is scrolled to.
			const top = wrapper.getBoundingClientRect().top - mainRect.top + main.scrollTop;
			const next = Math.max(160, main.clientHeight - top - MAIN_BOTTOM_PADDING);
			if (maxHeight === null || Math.abs(next - maxHeight) > 0.5) maxHeight = next;
		};
		// `main` resizing covers the window and banners; the list resizing covers the toolbar
		// above the table; visibility covers a tab switch.
		const resizes = new ResizeObserver(measure);
		resizes.observe(main);
		if (wrapper.parentElement) resizes.observe(wrapper.parentElement);
		const visibility = new IntersectionObserver(measure);
		visibility.observe(wrapper);
		measure();
		return () => {
			resizes.disconnect();
			visibility.disconnect();
		};
	});

	/**
	 * Column identity is keyed on the field keys, not the array.
	 *
	 * Tabs build `fields` inside a `$derived` over several queries, so every
	 * refetch yields a fresh array. Rebuilding the column defs on identity would
	 * throw away the row model on each one; the value functions read the current
	 * columns through a closure instead, so cells stay live regardless.
	 */
	let columnsKey = $derived(columns.map((c) => c.id).join('\0'));
	let columnDefs = $derived.by<ColumnDef<EntityTableFeatures, T>[]>(() => {
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
	let columnResizing = $state<columnResizingState>({
		startOffset: null,
		startSize: null,
		deltaOffset: null,
		deltaPercentage: null,
		isResizingColumn: false,
		columnSizingStart: []
	});

	/**
	 * Whether a click on a row selects it. A click on something inside the row that acts on its own
	 * (a link, a button, a tag, an input) keeps its own meaning, and finishing a text selection by
	 * dragging is not a click on the row.
	 */
	function isRowSelectClick(event: MouseEvent): boolean {
		const target = event.target as Element | null;
		if (
			target?.closest('a, button, input, select, textarea, label, [role="button"], [role="link"]')
		) {
			return false;
		}
		return !window.getSelection()?.toString();
	}

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
		forgetRenderedWidths(columnSizing, sizing);
		onColumnSizingChange(sizing);
	}

	/**
	 * A resized column's content cap starts again from its new width. Left at the old rendered
	 * width, the cap would hold the column open and a narrowing drag could never take.
	 */
	function forgetRenderedWidths(before: Record<string, number>, after: Record<string, number>) {
		for (const id of new Set([...Object.keys(before), ...Object.keys(after)])) {
			if (before[id] !== after[id]) delete renderedWidths[id];
		}
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
		// Rows arrive already filtered, sorted and paged. table-core is given no
		// row model that could reorder or drop one, so its view cannot drift from
		// what the controls produced.
		manualSorting: true,
		getRowId: (row: T) => getItemId(row),
		enableColumnResizing: true,
		columnResizeMode: 'onChange',
		onColumnSizingChange: (updater) => setSizing(resolve(updater, columnSizing)),
		onColumnResizingChange: (updater) => {
			columnResizing = resolve(updater, columnResizing);
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
			get columnResizing() {
				return columnResizing;
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
	function startResize(event: MouseEvent | TouchEvent, header: Header<EntityTableFeatures, T>) {
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
		forgetRenderedWidths(columnSizing, rest);
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
		const set = columnWidth(column, columnSizing) ?? UNSIZED_COLUMN_MAX;
		// A full-width table hands its spare room to the columns, so a column can render wider
		// than its set width or the cap. Its content fills what it renders at, up to the resize
		// handle, instead of wrapping at the cap with blank space beside it.
		const width = Math.max(set, renderedWidths[column.id] ?? 0);
		return `max-width: calc(${width}px - 2 * var(--cell-px))`;
	}

	/** Each data column's rendered header width, which the cells' content cap follows. */
	let renderedWidths = $state<Record<string, number>>({});

	function trackColumnWidth(node: HTMLElement, columnId: string) {
		let id = columnId;
		// Record on the next frame, not inside the delivery: the width caps the column's cell
		// content, which can resize the observed header and raise the ResizeObserver loop error.
		let frame = 0;
		const record = () => {
			if (renderedWidths[id] !== node.offsetWidth) renderedWidths[id] = node.offsetWidth;
		};
		const observer = new ResizeObserver(() => {
			cancelAnimationFrame(frame);
			frame = requestAnimationFrame(record);
		});
		observer.observe(node);
		return {
			update(next: string) {
				id = next;
			},
			destroy() {
				observer.disconnect();
				cancelAnimationFrame(frame);
				delete renderedWidths[id];
			}
		};
	}

	/**
	 * table-core's rows by id, so the body can emit group and section headers between rows while
	 * table-core still owns one row model for all of them.
	 */
	let rowById = $derived(new Map(view.rows.map((row) => [row.id, row])));

	/** Header checkbox, plus every data column, plus the actions column. */
	let spannedColumns = $derived(columns.length + (selectable ? 1 : 0) + (getActions ? 1 : 0));

	function entryKey(entry: TreeEntry<T>): string {
		return entry.type === 'row' ? getItemId(entry.item) : entry.key;
	}

	let byId = $derived(new Map(columns.map((c) => [c.id, c])));
	let primaryColumn = $derived(columns.find((c) => c.primary) ?? columns[0]);

	// Pinned while the table scrolls sideways: the checkbox and the name on the left, the tags and
	// the actions on the right (`.pin-left` / `.pin-right` in app.css). A column pins only in its
	// usual place, the name first and tags last; moved elsewhere, it scrolls like any other.
	let firstColumnId = $derived(view.headers[0]?.column.id);
	let lastColumnId = $derived(view.headers.at(-1)?.column.id);

	function pinOf(column: EntityColumn<T>): 'left' | 'right' | null {
		if (column.primary && column.id === firstColumnId) return 'left';
		if (column.id === 'tags' && column.id === lastColumnId) return 'right';
		return null;
	}

	/** The class and offset for a pinned data column's cells; `relative` for the rest. */
	function pinAttrs(column: EntityColumn<T>): { class: string; style: string } {
		const pin = pinOf(column);
		if (pin === 'left')
			return { class: 'pin-left pin-edge-right', style: '--pin-left: var(--pin-select-width)' };
		if (pin === 'right')
			return { class: 'pin-right pin-edge-left', style: '--pin-right: var(--pin-actions-width)' };
		return { class: 'relative', style: '' };
	}

	/**
	 * Whether a column's right edge is a pinned column's divider: the pinned name's own edge, or
	 * the edge beside the pinned tags. The divider line already shows there, so the resize grip
	 * hides at rest and appears only on hover, focus or drag.
	 */
	function edgeIsDivider(columnId: string): boolean {
		const at = view.headers.findIndex((h) => h.column.id === columnId);
		const here = byId.get(columnId);
		const next = byId.get(view.headers[at + 1]?.column.id ?? '');
		return (
			(here !== undefined && pinOf(here) === 'left') ||
			(next !== undefined && pinOf(next) === 'right')
		);
	}

	// The name pins beside the checkbox and tags beside the actions, so each needs its neighbour's
	// width as its offset.
	let selectHeader: HTMLTableCellElement | undefined = $state();
	let actionsHeader: HTMLTableCellElement | undefined = $state();
	let selectWidth = $state(0);
	let actionsWidth = $state(0);

	function trackWidth(node: HTMLElement, set: (width: number) => void): () => void {
		const observer = new ResizeObserver(() => set(node.offsetWidth));
		observer.observe(node);
		return () => observer.disconnect();
	}

	$effect(() => (selectHeader ? trackWidth(selectHeader, (w) => (selectWidth = w)) : undefined));
	$effect(() => (actionsHeader ? trackWidth(actionsHeader, (w) => (actionsWidth = w)) : undefined));

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
	header here pins to the wrapper rather than the viewport — which is why, from
	`md` up, the wrapper is also the vertical scroller (`maxHeight`).

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
	bind:this={wrapperEl}
	class="entity-table relative isolate overflow-x-auto"
	style:max-height={maxHeight === null ? undefined : `${maxHeight}px`}
	style:padding-bottom={bottomInset > 0 ? `${bottomInset}px` : undefined}
	style:--pin-select-width="{selectable ? selectWidth : 0}px"
	style:--pin-actions-width="{getActions ? actionsWidth : 0}px"
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
					<th
						bind:this={selectHeader}
						scope="col"
						class="pin-left w-10 px-(--cell-px) py-(--cell-py)"
					>
						<input
							type="checkbox"
							checked={allSelected}
							indeterminate={someSelected}
							onchange={onToggleAll}
							aria-label={allSelected ? common_deselectAll() : common_selectAllOnPage()}
							title="{common_selectAllOnPage()} ({shortcutLabel('A')})"
							class="checkbox-card h-4 w-4"
						/>
					</th>
				{/if}

				{#each view.headers as header (header.id)}
					{@const column = byId.get(header.column.id)}
					{#if column}
						{@const width = columnWidth(column, columnSizing)}
						{@const pin = pinAttrs(column)}
						<th
							use:trackColumnWidth={column.id}
							scope="col"
							aria-sort={ariaSort(column.id)}
							style="{width ? `width: ${width}px;` : ''} {pin.style}"
							class="text-secondary {pin.class} px-(--cell-px) py-(--cell-py) text-xs font-medium whitespace-nowrap {column.align ===
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
						Pinned with every one of its cells, not just the header: a header
						pinned alone let the last column's header scroll under it and read as
						missing. Each pinned cell paints the page background plus its row's
						tint, so content scrolling underneath never shows through.
					-->
					<th
						bind:this={actionsHeader}
						scope="col"
						class="pin-right px-(--cell-px) py-(--cell-py) text-left text-xs font-medium text-secondary"
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
							class="bg-black/[0.07] px-(--cell-px) py-(--cell-py) text-left dark:bg-white/8"
						>
							<GroupHeading
								name={group.name}
								count={group.items.length}
								range={group.range}
								collapsed={isCollapsed}
								onToggle={() => onToggleCollapse(group.key)}
							/>
						</th>
					</tr>

					{#if !isCollapsed}
						{#if group.entries}
							{@render treeEntries(group.entries, group.rootRanges)}
						{:else}
							{#each group.items as item (getItemId(item))}
								{@const row = rowById.get(getItemId(item))}
								{#if row}
									{@render bodyRow(row, 0, null)}
								{/if}
							{/each}
						{/if}
					{/if}
				{/each}
			{:else}
				{#each view.rows as row (row.id)}
					{@render bodyRow(row, 0, null)}
				{/each}
			{/if}
		</tbody>
	</table>
</div>

<!-- `rootRanges` holds the merged group's roots' own page ranges; only the top level has any. -->
{#snippet treeEntries(entries: TreeEntry<T>[], rootRanges: Map<string, GroupSlice> | null)}
	{#each entries as entry (entryKey(entry))}
		{#if entry.type === 'continued'}
			{@render continuationRow(entry)}
			{#if !collapsed.has(entry.key)}
				{@render treeEntries(entry.entries, null)}
			{/if}
		{:else if entry.type === 'row'}
			{@const row = rowById.get(getItemId(entry.item))}
			{#if row}
				{@render bodyRow(row, entry.depth, { section: null, range: null })}
			{/if}
		{:else}
			{@const row = rowById.get(getItemId(entry.item))}
			{#if row}
				{@render bodyRow(row, entry.depth, {
					section: entry,
					range: rootRanges?.get(getItemId(entry.item)) ?? null
				})}
			{/if}
			{#if !collapsed.has(entry.key)}
				{@render treeEntries(entry.entries, null)}
			{/if}
		{/if}
	{/each}
{/snippet}

<!-- The head of a tree whose root row is on another page: the root's name, standing in for its
     row so this page's rows of the tree nest under it. Not a row of the page, so it has no
     checkbox or actions. -->
{#snippet continuationRow(entry: TreeContinuation<T>)}
	{@const isCollapsed = collapsed.has(entry.key)}
	<tr class="border-t" style="border-color: var(--color-border)">
		<th
			scope="rowgroup"
			colspan={spannedColumns}
			class="px-(--cell-px) py-(--cell-py) text-left font-medium"
		>
			<button
				type="button"
				onclick={() => onToggleCollapse(entry.key)}
				aria-expanded={!isCollapsed}
				class="pin-content flex w-max items-center gap-1.5 text-sm text-secondary"
			>
				{#if isCollapsed}
					<ChevronRight class="h-4 w-4" aria-hidden="true" />
				{:else}
					<ChevronDown class="h-4 w-4" aria-hidden="true" />
				{/if}
				<span>{entry.label}</span>
				{#if entry.range}
					<span class="text-xs font-normal text-tertiary">
						{common_groupTotalShowing({
							total: entry.range.total,
							start: entry.range.start,
							end: entry.range.end
						})}
					</span>
				{/if}
			</button>
		</th>
	</tr>
{/snippet}

<!-- A row's status badges, icon-only, ahead of its name. -->
{#snippet rowBadges(item: T)}
	{#each getRowBadges?.(item) ?? [] as badge, index (index)}
		<span class="shrink-0"><Tag {...badge} label={undefined} /></span>
	{/each}
{/snippet}

<!-- A tree row's chevron, or the same width of space for a row with no children, so names line
     up across a level. A parent's chevron collapses its children and shows how many there are. -->
{#snippet treeToggle(tree: TreeCell<T>)}
	{#if tree.section}
		{@const section = tree.section}
		{@const isCollapsed = collapsed.has(section.key)}
		<button
			type="button"
			onclick={() => onToggleCollapse(section.key)}
			aria-expanded={!isCollapsed}
			aria-label={isCollapsed ? common_expand() : common_collapse()}
			class="flex shrink-0 items-center text-tertiary transition-colors hover:text-primary"
		>
			{#if isCollapsed}
				<ChevronRight class="h-4 w-4" aria-hidden="true" />
			{:else}
				<ChevronDown class="h-4 w-4" aria-hidden="true" />
			{/if}
		</button>
	{:else}
		<span class="w-4 shrink-0" aria-hidden="true"></span>
	{/if}
{/snippet}

{#snippet treeCount(tree: TreeCell<T>)}
	{#if tree.section}
		<span class="shrink-0 text-xs font-normal text-tertiary">
			{#if tree.range}
				{common_groupTotalShowing({
					total: tree.range.total,
					start: tree.range.start,
					end: tree.range.end
				})}
			{:else}
				({tree.section.count})
			{/if}
		</span>
	{/if}
{/snippet}

{#snippet bodyRow(row: Row<EntityTableFeatures, T>, depth: number, tree: TreeCell<T> | null)}
	{@const item = row.original}
	{@const itemId = getItemId(item)}
	{@const isSelected = selectedIds.has(itemId)}
	{@const rowActions = getActions ? getActions(item) : null}
	{@const openAction = rowActions?.find((a) => a.isOpen && !a.unavailable)}
	<!-- The checkbox is the keyboard path; the row click is a larger mouse target for the same. -->
	<tr
		data-keeps-selection
		class="border-t bg-(--row-tint) transition-colors {selectable || openAction
			? 'cursor-pointer'
			: ''} {isSelected
			? '[--row-tint:rgb(0_0_0/0.05)] dark:[--row-tint:rgb(255_255_255/0.05)]'
			: 'hover:[--row-tint:rgb(0_0_0/0.03)] dark:hover:[--row-tint:rgb(255_255_255/0.03)]'}"
		style="border-color: var(--color-border)"
		onmousedown={(e) => {
			// Shift+click would otherwise select the text between the two rows, and a double-click
			// the word under the pointer.
			if ((selectable && e.shiftKey) || (openAction && e.detail > 1 && isRowSelectClick(e))) {
				e.preventDefault();
			}
		}}
		onclick={(e) => {
			// The second click of a double-click opens the row; it doesn't undo the first's selection.
			if (openAction && e.detail > 1) return;
			if (selectable && isRowSelectClick(e)) onClickRow(itemId, e.shiftKey);
		}}
		ondblclick={(e) => {
			if (openAction && isRowSelectClick(e)) openAction.onClick();
		}}
	>
		{#if selectable}
			<td class="pin-left w-10 px-(--cell-px) py-(--cell-py) align-middle">
				<input
					type="checkbox"
					checked={isSelected}
					onclick={(e) => {
						e.stopPropagation();
						onClickRow(itemId, e.shiftKey);
						// A Shift+click on a selected row keeps it selected; undo the native toggle.
						e.currentTarget.checked = selectedIds.has(itemId);
					}}
					aria-label={common_selectRow({ name: rowLabel(item) })}
					class="checkbox-card h-4 w-4"
				/>
			</td>
		{/if}

		{#each view.headers as header (header.id)}
			{@const column = byId.get(header.column.id)}
			{#if column}
				{@const pin = pinAttrs(column)}
				{#if column.primary}
					<!-- Announces the row's identity before each cell when navigating across. -->
					<th
						scope="row"
						style={pin.style}
						class="text-primary {pin.class} px-(--cell-px) py-(--cell-py) text-left align-middle font-medium"
					>
						{#if tree}
							{#if depth > 0}
								<TreeGuides {depth} offset="var(--cell-px)" />
							{/if}
							<div class="flex items-center gap-1.5" style="padding-left: {depth}rem">
								{@render treeToggle(tree)}
								{@render rowBadges(item)}
								<div class="min-w-0" style={contentMaxWidth(column)}>
									<FieldValue {item} {column} />
								</div>
								{@render treeCount(tree)}
							</div>
						{:else}
							<div class="flex items-center gap-1.5">
								{@render rowBadges(item)}
								<div class="min-w-0" style={contentMaxWidth(column)}>
									<FieldValue {item} {column} />
								</div>
							</div>
						{/if}
					</th>
				{:else}
					<td
						style={pin.style}
						class="{pin.class} px-(--cell-px) py-(--cell-py) align-middle {column.align === 'right'
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

		{#if rowActions}
			<td class="pin-right px-(--cell-px) py-(--cell-py) text-right align-middle">
				<RowActions actions={rowActions} />
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
			class="group inline-flex min-w-0 items-center gap-1 transition-colors hover:text-primary"
		>
			<span class="truncate">{column.label}</span>
			<!--
				Every sortable header carries an icon, so it reads as sortable
				before anyone clicks it. A header without one cannot sort.
			-->
			{#if sortState.field === column.id}
				{#if sortState.direction === 'asc'}
					<ArrowUpNarrowWide class="h-3.5 w-3.5 text-accent" aria-hidden="true" />
				{:else}
					<ArrowDownWideNarrow class="h-3.5 w-3.5 text-accent" aria-hidden="true" />
				{/if}
			{:else}
				<ArrowUpDown
					class="h-3.5 w-3.5 text-tertiary group-hover:text-secondary group-focus-visible:text-secondary"
					aria-hidden="true"
				/>
			{/if}
		</button>
	{:else}
		<span class="truncate">{column.label}</span>
	{/if}
{/snippet}

{#snippet resizeHandle(header: Header<EntityTableFeatures, T>, column: EntityColumn<T>)}
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
		class="group absolute inset-y-0 right-0 z-10 flex w-2 cursor-col-resize touch-none justify-end select-none focus:outline-hidden"
		onmousedown={(e) => startResize(e, header)}
		ontouchstart={(e) => startResize(e, header)}
		ondblclick={() => resetWidth(column.id)}
		onkeydown={(e) => resizeByKey(e, column.id)}
	>
		<!-- A short grip at rest so the handle can be found; full height while in use. -->
		<span
			class="w-0.5 self-center rounded-full transition-all {columnResizing.isResizingColumn ===
			column.id
				? 'h-full bg-blue-500'
				: `h-1/2 group-hover:h-full group-hover:bg-blue-500/60 group-focus-visible:h-full group-focus-visible:bg-blue-500 ${edgeIsDivider(column.id) ? 'bg-transparent' : 'bg-black/20 dark:bg-white/25'}`}"
		></span>
	</div>
{/snippet}
