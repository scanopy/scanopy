<script lang="ts" generics="T">
	import {
		type FieldConfig,
		getFieldKey,
		groupPageSlice,
		type GroupSlice,
		type PageSizeOption,
		type CardAction,
		type TableDefaults
	} from './types';
	import { getUniqueValues as uniqueValuesOf } from './controls/fieldValues';
	import {
		sortItems,
		nextSortState,
		sortableFields as sortableFieldsOf,
		groupableFields as groupableFieldsOf,
		type SortState
	} from './controls/sorting';
	import {
		matchesSearch,
		matchesFilters,
		blankFilterState,
		restoredServerFilters,
		hasActiveFilters as hasActiveFiltersOf,
		type FilterState
	} from './controls/filtering';
	import { createFilterActions } from './controls/filterActions';
	import { guardServerPaginatedConfig, guardTableDefaults } from './controls/devGuards.svelte';
	import { columnControls, renderedFields, dropHiddenColumnState } from './controls/headerControls';
	import { paginationView, pageSlice } from './controls/pagination';
	import { createChangeTracker, sameOrder } from './controls/changeTracker';
	import {
		groupItems as groupItemsBy,
		buildTreeSections,
		flattenTreeEntries,
		computeGroupOffsets,
		serverGroupKey as serverGroupKeyOf,
		type RenderGroup,
		type TreeEntry
	} from './controls/grouping';
	import {
		isAllSelected,
		isPartiallySelected,
		visibleIds,
		runBulkAction
	} from './controls/selection';
	import { observeStuck } from './controls/stickyHeader';
	import {
		parseStoredState,
		serializeState,
		reviveFilterState,
		toStoredFilterState,
		resolveOrdering,
		STORED_STATE_VERSION,
		type OrderingChoice
	} from './controls/dataControlsStorage';
	import ControlsBar from './controls/ControlsBar.svelte';
	import ColumnControlsMenu from './table/ColumnControlsMenu.svelte';
	import BulkActionBar from './controls/BulkActionBar.svelte';
	import PaginationBar from './controls/PaginationBar.svelte';
	import EntityTable from './table/EntityTable.svelte';
	import ColumnVisibilityMenu from './table/ColumnVisibilityMenu.svelte';
	import TagCell from './TagCell.svelte';
	import { tagItems } from '$lib/features/tags/columns';
	import {
		fieldsToColumns,
		reconcileColumnState,
		visibleColumns,
		buildTagColumn,
		withTagColumn,
		type EntityColumn
	} from './table/columns';
	import { onMount } from 'svelte';
	import {
		common_all,
		common_clearAll,
		common_noEntityMatchesFilters,
		common_ungrouped,
		common_tableCaption,
		common_tags,
		common_item,
		common_items,
		common_no,
		common_yes
	} from '$lib/paraglide/messages';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import { SearchX } from 'lucide-svelte';
	import {
		useTagsQuery,
		useBulkAddTagMutation,
		useBulkRemoveTagMutation,
		type EntityDiscriminants
	} from '$lib/features/tags/queries';
	import { computeCommonTags } from '$lib/shared/utils/tags';
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';
	import throttle from 'just-throttle';
	import type { components } from '$lib/api/schema';

	type PaginationMeta = components['schemas']['PaginationMeta'];

	/** Debounce window for the search box, in ms. */
	const SEARCH_THROTTLE_MS = 300;

	let {
		items = $bindable([]),
		fields = $bindable([]),
		storageKey = null,
		onBulkDelete = null,
		allowBulkDelete = true,
		entityType = null,
		getItemTags = null,
		getItemId,
		// Server-side pagination (optional)
		serverPagination = null,
		onPageChange = null,
		// Server-side ordering callback (optional)
		// Called when grouping or sorting changes, allowing parent to update query params
		onOrderChange = null,
		// Server-side tag filtering callback (optional)
		// Called when tag filter selection changes, with array of selected tag IDs
		onTagFilterChange = null,
		// Server-side field filter callback (optional)
		// Called when a `serverFiltered` field's selection changes
		onFilterChange = null,
		// Server-side staleness filtering callback (optional)
		// Called when the "Stale only" toggle changes
		onStaleFilterChange = null,
		// Server-side search callback (optional)
		// Called (debounced) when the search box changes
		onSearchChange = null,
		// CSV export callback (optional, default behavior)
		// Called when user clicks export button; parent handles the actual export
		onCsvExport = null,
		// Export button click override (optional)
		// If provided, replaces onCsvExport entirely - use for custom export UI (e.g., modal with options)
		onExportClick = null,
		// Row actions, rendered in the table's trailing column.
		getActions = null,
		// Names the table for screen readers, e.g. "Hosts".
		entityLabel = null,
		// The grouping and sort the tab opens with until the user picks their own.
		defaults = {}
	}: {
		items: T[];
		fields: FieldConfig<T>[];
		storageKey?: string | null;
		onBulkDelete?: ((ids: string[]) => Promise<void>) | null;
		allowBulkDelete?: boolean;
		entityType?: EntityDiscriminants | null;
		getItemTags?: ((item: T) => string[]) | null;
		getItemId: (item: T) => string;
		// Server-side pagination: when provided, pagination is server-controlled
		// Callback receives both page and pageSize so parent can use in query
		serverPagination?: PaginationMeta | null;
		onPageChange?: ((page: number, pageSize: number) => void) | null;
		// Server-side ordering: called when group/sort changes
		// Args: (groupBy field key, orderBy field key, direction)
		onOrderChange?:
			((groupBy: string | null, orderBy: string | null, direction: 'asc' | 'desc') => void) | null;
		// Server-side tag filtering: called when tag filter changes
		// Args: array of tag IDs to filter by
		onTagFilterChange?: ((tagIds: string[]) => void) | null;
		// Server-side field filter: called when a field marked `serverFiltered`
		// changes. Args: (fieldKey, selected values). The field opts in explicitly
		// rather than this applying to every filter, so a key the parent doesn't
		// handle keeps its client-side filtering instead of silently doing nothing.
		onFilterChange?: ((fieldKey: string, values: string[]) => void) | null;
		// Server-side staleness filtering: called when the "Stale only" toggle
		// changes. `true` = only stale, `null` = no staleness constraint.
		// Server-side because these lists are server-paginated — a client-side
		// filter would only ever filter the page currently loaded.
		onStaleFilterChange?: ((stale: boolean | null) => void) | null;
		// Server-side search: called with the current query, debounced.
		// Server-side for the same reason as the filters above — searching the
		// loaded page would silently miss every match on another page. Lists
		// that load everything omit this and keep the client-side search.
		onSearchChange?: ((query: string) => void) | null;
		// CSV export: default behavior when user clicks export button
		onCsvExport?: (() => void | Promise<void>) | null;
		// Export button click override: if provided, replaces onCsvExport entirely
		onExportClick?: (() => void | Promise<void>) | null;
		// Row actions.
		getActions?: ((item: T) => CardAction[]) | null;
		// Accessible name for the table, e.g. "Hosts".
		entityLabel?: string | null;
		defaults?: TableDefaults;
	} = $props();

	// Tags query for filter display
	const tagsQuery = useTagsQuery();
	let allTags = $derived(tagsQuery.data ?? []);

	// Bulk tag mutations
	const bulkAddTagMutation = useBulkAddTagMutation();
	const bulkRemoveTagMutation = useBulkRemoveTagMutation();

	// Search state
	let searchQuery = $state('');

	// Filter state. The shape lives with the filtering logic that reads it, so
	// there is one definition of what a filter selection is.
	let filterState = $state<FilterState>({});
	// Staleness lives outside `filterState`: it's a server-side constraint
	// rather than a set of values matched against loaded rows.
	let staleOnly = $state(false);

	// Sort and grouping: only what the user picked is held and stored, and the
	// tab's defaults fill whatever they didn't.
	let ordering = $state<OrderingChoice>({});
	let fieldKeys = $derived(new Set(fields.map((f) => getFieldKey(f))));
	let resolvedOrdering = $derived(resolveOrdering(ordering, defaults, fieldKeys));
	let sortState = $derived(resolvedOrdering.sortState);
	let selectedGroupField = $derived(resolvedOrdering.groupField);

	// Column state — owned here so it persists alongside every other control,
	// and handed to table-core as controlled state rather than kept in parallel.
	let columnVisibility = $state<Record<string, boolean>>({});
	let columnOrder = $state<string[]>([]);
	let columnSizing = $state<Record<string, number>>({});

	// Pagination state
	let currentPage = $state(1);
	let pageSize = $state<PageSizeOption>(20);

	// Bulk selection state (always enabled when onBulkDelete is provided)
	let selectedIds = new SvelteSet<string>();

	// Load state from localStorage
	// Returns the restored pageSize if one was found, otherwise null
	function loadState(): PageSizeOption | null {
		if (!storageKey || typeof localStorage === 'undefined') return null;

		const state = parseStoredState(localStorage.getItem(storageKey));
		if (!state) return null;

		searchQuery = state.searchQuery;
		filterState = reviveFilterState(state.filterState);
		ordering = { sort: state.sortState, group: state.selectedGroupField };
		currentPage = state.currentPage;

		if (state.columnVisibility) columnVisibility = state.columnVisibility;
		if (state.columnOrder) columnOrder = state.columnOrder;
		if (state.columnSizing) columnSizing = state.columnSizing;

		// Before the restored filters are replayed to the parent, so a dropped
		// one is simply never sent.
		dropHiddenColumns(false);

		if (state.pageSize) {
			pageSize = state.pageSize;
			return state.pageSize;
		}

		return null;
	}

	// Save state to localStorage
	function saveState() {
		if (!storageKey || typeof localStorage === 'undefined') return;

		try {
			localStorage.setItem(
				storageKey,
				serializeState({
					version: STORED_STATE_VERSION,
					searchQuery,
					filterState: toStoredFilterState(filterState),
					sortState: ordering.sort,
					selectedGroupField: ordering.group,
					currentPage,
					pageSize,
					columnVisibility,
					columnOrder,
					columnSizing
				})
			);
		} catch (e) {
			console.warn('Failed to save DataControls state to localStorage:', e);
		}
	}

	// Seed a filter for any field that lacks one, leaving restored state alone.
	$effect(() => {
		const seeded = blankFilterState(fields, true);
		for (const [key, filter] of Object.entries(seeded)) {
			if (!filterState[key]) filterState[key] = filter;
		}
	});

	// Load state on mount and set up auto-save
	onMount(() => {
		const restoredPageSize = loadState();

		// Notify parent of restored state for server-side pagination
		// This ensures the parent's query uses the restored pageSize
		if (restoredPageSize && onPageChange) {
			onPageChange(currentPage, restoredPageSize);
		}

		// Notify parent of the ordering in force, restored or default. Always, so
		// a saved "no grouping" also replaces the default the parent started with.
		onOrderChange?.(activeGroupField, activeSort.field, activeSort.direction);

		// Notify parent of restored search state
		if (onSearchChange && searchQuery.trim()) {
			onSearchChange(searchQuery);
		}

		// Notify parent of restored tag filter state
		const tagFilter = filterState['tags'];
		if (onTagFilterChange && tagFilter && tagFilter.values.size > 0) {
			onTagFilterChange(Array.from(tagFilter.values));
		}

		// Notify parent of restored server-side filter state
		if (onFilterChange) {
			for (const { key, values } of restoredServerFilters(fields, filterState)) {
				onFilterChange(key, values);
			}
		}

		// Set up reactive save (debounced)
		let saveTimeout: ReturnType<typeof setTimeout>;

		const unsubscribe = $effect.root(() => {
			$effect(() => {
				if (storageKey) {
					// Track all state that should trigger saves
					void searchQuery;
					void filterState;
					void ordering;
					void currentPage;
					void pageSize;
					void columnVisibility;
					void columnOrder;
					void columnSizing;

					// Debounce saves
					clearTimeout(saveTimeout);
					saveTimeout = setTimeout(saveState, 100);
				}
			});
		});

		return () => {
			clearTimeout(saveTimeout);
			unsubscribe();
		};
	});

	// Get unique string values for a field (handles arrays by flattening)
	function getUniqueValues(field: FieldConfig<T>): string[] {
		return uniqueValuesOf(items, field);
	}

	// Check if using server-side pagination
	let useServerPagination = $derived(serverPagination !== null && onPageChange !== null);

	// ---- Table columns -------------------------------------------------------

	let allColumns = $derived(fieldsToColumns(fields, useServerPagination));
	/**
	 * Tags are appended by the list itself rather than declared per tab.
	 *
	 * Every taggable entity gets the same editable column in the same place —
	 * last, next to the actions — instead of each tab remembering to add one, so
	 * a tab cannot silently end up without it. It is editable only when the
	 * parent supplied an `entityType`, which is also how it gates permission.
	 */
	let tagColumn = $derived(
		getItemTags ? buildTagColumn<T>(common_tags(), getItemTags, tagsCell) : null
	);

	let columnState = $derived(
		reconcileColumnState(
			allColumns,
			{ visibility: columnVisibility, order: columnOrder, sizing: columnSizing },
			tagColumn ? [tagColumn] : []
		)
	);

	let renderedColumns = $derived(withTagColumn(visibleColumns(allColumns, columnState), tagColumn));
	let renderedColumnIds = $derived(new Set(renderedColumns.map((c) => c.id)));
	/** The fields the list filters, sorts and groups by: those with a rendered column. */
	let columnFields = $derived(renderedFields(fields, renderedColumnIds));
	let staleAvailable = $derived(onStaleFilterChange !== null);

	// Under server pagination only server-orderable fields are offered: the
	// client holds one page, so sorting or grouping it here would describe that
	// page alone while the pager walks the server's order. Only fields whose
	// column renders are offered: hiding a column takes its controls too.
	let groupableFields = $derived(groupableFieldsOf(columnFields, useServerPagination));
	let sortableFields = $derived(sortableFieldsOf(columnFields, useServerPagination));

	// The group and sort actually in force. A choice restored from storage can
	// name a field this list no longer offers, which then applies nothing rather
	// than grouping by it here or sending the server a key it cannot order by.
	let activeGroupField = $derived(
		groupableFields.some((f) => getFieldKey(f) === selectedGroupField) ? selectedGroupField : null
	);
	let activeSort = $derived<SortState>(
		sortableFields.some((f) => getFieldKey(f) === sortState.field)
			? sortState
			: { field: null, direction: sortState.direction }
	);

	// Apply all filters, sorting, and grouping
	let processedItems = $derived.by(() => {
		const serverMode = {
			tags: onTagFilterChange !== null,
			fields: onFilterChange !== null
		};

		const result = items.filter((item) => {
			// Search is skipped when the parent searches server-side — the rows that
			// arrived are already the matches.
			if (!onSearchChange && !matchesSearch(item, fields, searchQuery)) return false;
			return matchesFilters(item, fields, filterState, serverMode, getItemTags);
		});

		return sortItems(result, fields, activeSort, onOrderChange !== null, useServerPagination);
	});

	// Per-group totals across every page, when the server supplied them.
	let serverGroupCounts = $derived(serverPagination?.group_counts ?? null);

	// Group items by selected field
	let groupedItems = $derived.by(() => {
		if (!activeGroupField) {
			return new SvelteMap([[common_all(), processedItems]]);
		}

		// A server-paginated page arrives in the server's group order, which the
		// cumulative offsets from `serverGroupCounts` are indexed by, so its
		// buckets keep that order.
		return groupItemsBy(
			processedItems,
			fields,
			activeGroupField,
			{ ungrouped: common_ungrouped(), yes: common_yes(), no: common_no() },
			serverGroupCounts !== null || useServerPagination
		);
	});

	/**
	 * Each group's nested sections when the grouping is a tree, built here where the order is
	 * decided rather than in the renderers. Null otherwise.
	 */
	let treeEntries = $derived.by(() => {
		if (!activeTree) return null;
		const { tree, compare } = activeTree;
		const entries = new SvelteMap<string, TreeEntry<T>[]>();
		for (const [name, rows] of groupedItems) {
			entries.set(name, buildTreeSections(rows, tree, useServerPagination, name, compare));
		}
		return entries;
	});

	/** The active group field, when its groups are trees. */
	let activeTree = $derived.by(() => {
		const field = fields.find((f) => getFieldKey(f) === activeGroupField);
		return field?.tree ? { tree: field.tree, compare: field.compare } : null;
	});

	/**
	 * Collapsed groups and tree sections, by key. Group keys and section paths can't collide:
	 * section paths are a group key plus NUL-separated row keys.
	 */
	const collapsedSections = new SvelteSet<string>();

	function toggleSection(key: string) {
		if (collapsedSections.has(key)) collapsedSections.delete(key);
		else collapsedSections.add(key);
	}

	let groupOffsets = $derived(computeGroupOffsets(serverGroupCounts));

	function serverGroupKey(rows: T[]): string {
		return serverGroupKeyOf(rows, fields, activeGroupField);
	}

	/**
	 * How much of a group this page is showing, and how big the group really
	 * is. Null when the server didn't supply totals — on an unpaginated list
	 * the rendered count is already the whole group.
	 */
	function groupRange(groupItems: T[]): GroupSlice | null {
		if (!serverPagination) return null;

		const group = groupOffsets.get(serverGroupKey(groupItems));
		if (!group) return null;

		return groupPageSlice(group, serverPagination.offset, items.length);
	}

	/** The groups as the table renders them. */
	let groupList = $derived<RenderGroup<T>[]>(
		[...groupedItems.entries()].map(([name, groupItems]) => {
			const entries = treeEntries?.get(name) ?? null;
			return {
				key: name,
				name,
				items: entries ? flattenTreeEntries(entries) : groupItems,
				range: groupRange(groupItems),
				entries
			};
		})
	);

	function toggleSort(fieldKey: string) {
		ordering = { ...ordering, sort: nextSortState(sortState, fieldKey) };
	}

	/**
	 * A narrowed filter invalidates the current offset — page 7 of the old
	 * result set is not page 7 of the new one, and is often past its end.
	 */
	function resetToFirstPage() {
		goToPage(1);
	}

	const filters = createFilterActions<T>({
		get fields() {
			return fields;
		},
		get filterState() {
			return filterState;
		},
		set filterState(next) {
			filterState = next;
		},
		get staleOnly() {
			return staleOnly;
		},
		set staleOnly(next) {
			staleOnly = next;
		},
		get onFilterChange() {
			return onFilterChange;
		},
		get onStaleFilterChange() {
			return onStaleFilterChange;
		},
		resetToFirstPage
	});

	/**
	 * Clear whatever hidden columns filter, sort or group by. A list only
	 * narrows and orders by columns it renders, so hiding one takes its state
	 * with it. `notify` is false while restoring, before anything reached the
	 * parent; the order and tag effects pick up their own changes.
	 *
	 * Only the user's own sort and group are cleared. A default on a hidden
	 * column is already inert, since the list orders by rendered columns only,
	 * and it comes back with the column.
	 */
	function dropHiddenColumns(notify: boolean) {
		const next = dropHiddenColumnState(
			{
				filterState,
				staleOnly,
				sortState: ordering.sort ?? { field: null, direction: 'asc' },
				groupField: ordering.group ?? null
			},
			fields,
			renderedColumnIds
		);
		if (!next) return;

		filterState = next.filterState;
		staleOnly = next.staleOnly;
		ordering = {
			sort: ordering.sort && next.sortState,
			group: ordering.group === undefined ? undefined : next.groupField
		};

		if (!notify) return;
		if (next.staleCleared) onStaleFilterChange?.(null);
		filters.notifyServerCleared(next.serverKeys);
	}

	function clearSearch() {
		searchQuery = '';
	}

	/**
	 * Everything that can narrow the list, in one action — including the
	 * server-side ones, which `clearAll` and the search effect notify the
	 * parent about. This is what the filtered-empty state offers, so a filter
	 * that matches nothing is always reversible from where the user is looking.
	 */
	function clearAllNarrowing() {
		filters.clearAll();
		clearSearch();
	}

	/** A header's group button: group by this column, or ungroup if it already is. */
	function toggleGroup(fieldKey: string) {
		ordering = { ...ordering, group: activeGroupField === fieldKey ? null : fieldKey };
	}

	// Deselect all items
	function selectNone() {
		selectedIds.clear();
	}

	// Handle bulk delete
	async function handleBulkDelete() {
		const deleted = await runBulkAction(
			'Bulk delete',
			selectedIds,
			Boolean(allowBulkDelete && onBulkDelete),
			(ids) => onBulkDelete!(ids)
		);
		// Only on success: a failed delete leaves the rows, so clearing here
		// would drop the selection the user still needs to retry.
		if (deleted) selectedIds.clear();
	}

	function bulkTagAction(description: string, mutate: typeof bulkAddTagMutation) {
		return (tagId: string) =>
			runBulkAction(description, selectedIds, entityType !== null, (ids) =>
				mutate.mutateAsync({ entity_ids: ids, entity_type: entityType!, tag_id: tagId })
			);
	}

	const handleBulkTagAdd = $derived(bulkTagAction('Bulk tag add', bulkAddTagMutation));
	const handleBulkTagRemove = $derived(bulkTagAction('Bulk tag remove', bulkRemoveTagMutation));

	// Compute common tags across selected items (intersection)
	let commonTags = $derived.by(() => {
		if (!getItemTags || selectedIds.size === 0) return [];

		const selectedItems = items.filter((item) => selectedIds.has(getItemId(item)));
		if (selectedItems.length === 0) return [];

		return computeCommonTags(selectedItems.map((item) => ({ tags: getItemTags!(item) })));
	});

	let hasBulkTagging = $derived(entityType !== null && getItemTags !== null);

	let hasActiveFilters = $derived(hasActiveFiltersOf(fields, filterState, staleOnly));

	let hasActiveSearch = $derived(searchQuery.trim().length > 0);
	let hasActiveGrouping = $derived(activeGroupField !== null);

	guardServerPaginatedConfig(() => ({
		fields,
		serverPaginated: useServerPagination,
		server: { tags: onTagFilterChange !== null, fields: onFilterChange !== null }
	}));
	guardTableDefaults(() => ({ fields, defaults, serverPaginated: useServerPagination }));

	// Every pagination number comes from one resolution of "who is paginating",
	// so the count and the rows beneath it cannot fall out of step. The server's
	// total already accounts for search and filters, so there is no
	// filtered-vs-unfiltered discrepancy left to paper over here.
	let page = $derived(
		paginationView(
			useServerPagination ? serverPagination : null,
			currentPage,
			pageSize,
			processedItems.length
		)
	);
	let effectiveCurrentPage = $derived(page.currentPage);
	let totalPages = $derived(page.totalPages);
	let totalCount = $derived(page.totalCount);

	let paginatedItems = $derived(
		pageSlice(processedItems, effectiveCurrentPage, pageSize, useServerPagination)
	);

	function setRowSelected(itemId: string, selected: boolean) {
		if (selected) {
			selectedIds.add(itemId);
		} else {
			selectedIds.delete(itemId);
		}
	}

	/** Select-all scoped to one rendered block — a group's rows, or the page. */
	function toggleAllIn(rows: T[]) {
		if (isAllSelected(rows, selectedIds, getItemId)) {
			visibleIds(rows, getItemId).forEach((id) => selectedIds.delete(id));
		} else {
			visibleIds(rows, getItemId).forEach((id) => selectedIds.add(id));
		}
	}

	let showSelection = $derived(Boolean(onBulkDelete) || hasBulkTagging);

	let tableCaptionText = $derived(
		common_tableCaption({
			entity: entityLabel ?? '',
			count: totalCount,
			itemLabel: totalCount === 1 ? common_item() : common_items()
		})
	);

	function toggleColumn(id: string) {
		columnVisibility = { ...columnState.visibility, [id]: columnState.visibility[id] === false };
		dropHiddenColumns(true);
	}

	function reorderColumns(order: string[]) {
		columnOrder = order;
	}

	function resizeColumns(sizing: Record<string, number>) {
		columnSizing = sizing;
	}

	function resetColumns() {
		columnVisibility = {};
		columnOrder = [];
		columnSizing = {};
		dropHiddenColumns(true);
	}

	// Reset to page 1 when filters/search change and current page would be out of bounds
	$effect(() => {
		if (effectiveCurrentPage > totalPages && totalPages > 0) {
			resetToFirstPage();
		}
	});

	// Each of the three trackers below skips its first run, which only restores
	// saved state: firing there would reset the restored page to 1 on every mount.
	const orderTracker = createChangeTracker<[string | null, string | null, 'asc' | 'desc']>((a, b) =>
		sameOrder(a, b)
	);

	// Notify parent of ordering changes and reset pagination
	$effect(() => {
		const ordering: [string | null, string | null, 'asc' | 'desc'] = [
			activeGroupField,
			activeSort.field,
			activeSort.direction
		];

		if (orderTracker.changed(ordering)) {
			resetToFirstPage();
			onOrderChange?.(...ordering);
		}
	});

	// Trailing throttle so a burst of keystrokes costs one request and the last
	// one is never dropped. Built once — rebuilding it per keystroke would
	// defeat the debounce entirely.
	const notifySearchChange = throttle(
		(query: string) => {
			onSearchChange?.(query);
			// Page 3 of the old result set is meaningless against the new one.
			resetToFirstPage();
		},
		SEARCH_THROTTLE_MS,
		{ leading: false, trailing: true }
	);

	const searchTracker = createChangeTracker<string>();

	// Notify parent of search changes. `notifySearchChange` resets the page
	// itself, on the throttle's trailing edge rather than per keystroke.
	$effect(() => {
		if (searchTracker.changed(searchQuery)) {
			notifySearchChange(searchQuery);
		}
	});

	const tagFilterTracker = createChangeTracker<string[]>(sameOrder);

	// Notify parent of tag filter changes
	$effect(() => {
		const tagFilter = filterState['tags'];
		const tagIds = tagFilter ? Array.from(tagFilter.values).sort() : [];

		if (tagFilterTracker.changed(tagIds)) {
			resetToFirstPage();
			onTagFilterChange?.(tagIds);
		}
	});

	/** Move to a page, through the parent when it paginates. */
	function goToPage(target: number, size: PageSizeOption = pageSize) {
		if (useServerPagination && onPageChange) {
			onPageChange(target, size);
		} else {
			currentPage = target;
		}
	}

	function handlePageSizeChange(newSize: PageSizeOption) {
		pageSize = newSize;
		goToPage(1, newSize);
	}
	// Sticky detection
	let isStuck = $state(false);
	let sentinelRef: HTMLDivElement | null = $state(null);

	$effect(() => {
		if (!sentinelRef) return;
		return observeStuck(sentinelRef, (stuck) => (isStuck = stuck));
	});
</script>

<div class="space-y-4">
	<!-- Sentinel for sticky detection -->
	<div bind:this={sentinelRef} class="h-0 w-full"></div>

	<!-- Sticky Controls Bar -->
	<div
		class="sticky top-0 z-20 -mx-4 border-b bg-[var(--color-bg-body)] px-4 pb-4 {isStuck
			? 'border-gray-700 pt-4 shadow-lg'
			: 'border-transparent'}"
	>
		<ControlsBar
			bind:searchQuery
			{hasActiveSearch}
			onClearSearch={clearSearch}
			onExport={onExportClick ?? onCsvExport}
		>
			{#snippet columnMenu()}
				<ColumnVisibilityMenu
					columns={allColumns}
					order={columnState.order}
					visibility={columnState.visibility}
					onToggle={toggleColumn}
					onReorder={reorderColumns}
					onReset={resetColumns}
				/>
			{/snippet}
			{#snippet filterActions()}
				{#if hasActiveFilters}
					<button onclick={filters.clearAll} class="btn-secondary h-[42px] whitespace-nowrap">
						{common_clearAll()}
					</button>
				{/if}
			{/snippet}
		</ControlsBar>
	</div>

	<!-- Bulk Action Bar (shown when items are selected) -->
	{#if (onBulkDelete || hasBulkTagging) && selectedIds.size > 0}
		<BulkActionBar
			selectedCount={selectedIds.size}
			showDelete={Boolean(allowBulkDelete && onBulkDelete)}
			showTagging={hasBulkTagging}
			{commonTags}
			onClearSelection={selectNone}
			onBulkDelete={handleBulkDelete}
			onTagAdd={handleBulkTagAdd}
			onTagRemove={handleBulkTagRemove}
		/>
	{/if}

	<!-- Results Count and Pagination -->
	<PaginationBar
		{totalCount}
		{totalPages}
		currentPage={effectiveCurrentPage}
		{pageSize}
		showingStart={page.showingStart}
		showingEnd={page.showingEnd}
		canGoPrev={page.canGoPrev}
		canGoNext={page.canGoNext}
		groupCount={hasActiveGrouping ? groupedItems.size : null}
		{useServerPagination}
		processedCount={processedItems.length}
		itemCount={items.length}
		onPrevPage={() => page.canGoPrev && goToPage(effectiveCurrentPage - 1)}
		onNextPage={() => page.canGoNext && goToPage(effectiveCurrentPage + 1)}
		onPageSizeChange={handlePageSizeChange}
	/>

	<!-- Content -->
	{#if totalCount === 0 && (hasActiveFilters || hasActiveSearch)}
		<!--
			A filter that matches nothing is not an empty inventory, and must not be
			reported as one. It also has to stay reversible from here: a tab that swaps
			its "nothing configured yet" state in for this whole component takes the
			filter controls with it, which is how "Stale only" could strand a user with
			no way to undo it (GH #677 follow-up).

			`totalCount`, not this page's length — under server pagination a page can be
			empty while the filter still matches rows on another one.
		-->
		<EmptyState
			IconComponent={SearchX}
			title={common_noEntityMatchesFilters({ entity: entityLabel ?? common_items() })}
		>
			<button onclick={clearAllNarrowing} class="btn-secondary">{common_clearAll()}</button>
		</EmptyState>
	{:else}
		<!--
			Grouped or not, one table with one header row. Splitting a grouped list
			into a table per group gave each group its own header and its own column
			widths, so columns stopped lining up across the very groups you were
			comparing — which is the whole reason to use a table.
		-->
		{@render tableFor(
			hasActiveGrouping ? null : paginatedItems,
			hasActiveGrouping ? null : tableCaptionText
		)}
	{/if}
</div>

{#snippet tagsCell(item: T)}
	{@const ids = getItemTags ? getItemTags(item) : []}
	<TagCell
		items={tagItems(ids, allTags)}
		tagIds={ids}
		entityId={getItemId(item)}
		entityType={entityType ?? undefined}
		editable={Boolean(entityType)}
	/>
{/snippet}

{#snippet tableFor(rows: T[] | null, caption: string | null)}
	{@const flat = rows ?? groupList.flatMap((group) => group.items)}
	<EntityTable
		items={rows}
		groups={rows ? null : groupList}
		collapsed={collapsedSections}
		onToggleCollapse={toggleSection}
		columns={renderedColumns}
		columnSizing={columnState.sizing}
		onColumnSizingChange={resizeColumns}
		sortState={activeSort}
		selectable={showSelection}
		{selectedIds}
		allSelected={isAllSelected(flat, selectedIds, getItemId)}
		someSelected={isPartiallySelected(flat, selectedIds, getItemId)}
		{getItemId}
		{getActions}
		caption={caption ?? tableCaptionText}
		onToggleSort={toggleSort}
		onToggleRow={setRowSelected}
		onToggleAll={() => toggleAllIn(flat)}
		{headerControl}
	/>
{/snippet}

{#snippet headerControl(column: EntityColumn<T>)}
	{@const control = columnControls(column.id, fields, useServerPagination, staleAvailable)}
	{#if control}
		<ColumnControlsMenu
			{control}
			label={column.label}
			{filterState}
			{staleOnly}
			{activeGroupField}
			{allTags}
			{getUniqueValues}
			{filters}
			onToggleGroup={toggleGroup}
		/>
	{/if}
{/snippet}
