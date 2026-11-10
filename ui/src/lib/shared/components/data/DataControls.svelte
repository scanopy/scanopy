<script lang="ts" generics="T extends RowData">
	import type { RowData } from '@tanstack/table-core';
	import type { Snippet } from 'svelte';
	import {
		type FieldConfig,
		getFieldKey,
		groupPageSlice,
		type GroupSlice,
		type PageSizeOption,
		type RowAction,
		type TableDefaults,
		type TagProps
	} from './types';
	import { filterOptionsOf } from './controls/fieldValues';
	import {
		nextSortState,
		sortableFields as sortableFieldsOf,
		groupableFields as groupableFieldsOf,
		type SortState
	} from './controls/sorting';
	import {
		blankFilterState,
		hasActiveFilters as hasActiveFiltersOf,
		activeFilterCount as activeFilterCountOf,
		type FilterState
	} from './controls/filtering';
	import { createFilterActions } from './controls/filterActions';
	import { guardServerFilters, guardTableDefaults } from './controls/devGuards.svelte';
	import { columnControls, renderedFields, dropHiddenColumnState } from './controls/headerControls';
	import { paginationView } from './controls/pagination';
	import { createChangeTracker } from './controls/changeTracker';
	import {
		groupItems as groupItemsBy,
		buildTreeSections,
		mergeRootedTrees,
		rootedTreesPosition,
		flattenTreeEntries,
		computeGroupOffsets,
		serverGroupKey as serverGroupKeyOf,
		type RenderGroup,
		type TreeEntry
	} from './controls/grouping';
	import {
		isAllSelected,
		rangeSelect,
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
	import {
		decodeFilters,
		decodeListParams,
		hasListParams,
		listParamKeys,
		listParams,
		PAGE_PARAM,
		type ServerQuery
	} from './controls/listQuery';
	import { registerUrlOwner, syncUrl, takeInitialParams } from '$lib/shared/stores/url-state';
	import ControlsBar from './controls/ControlsBar.svelte';
	import ColumnControlsMenu from './table/ColumnControlsMenu.svelte';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import PaginationBar from './controls/PaginationBar.svelte';
	import SelectionMenus from './controls/SelectionMenus.svelte';
	import EntityTable from './table/EntityTable.svelte';
	import ColumnVisibilityMenu from './table/ColumnVisibilityMenu.svelte';
	import TagCell from './TagCell.svelte';
	import { tagItems } from '$lib/features/tags/columns';
	import { tagGroupFields, tagGroupIdOf } from '$lib/features/tags/groupFields';
	import {
		fieldsToColumns,
		reconcileColumnState,
		toggledColumnVisibility,
		visibleColumns,
		columnsWhileGrouped,
		buildTagColumn,
		withTagColumn,
		type EntityColumn
	} from './table/columns';
	import { onMount } from 'svelte';
	import {
		common_all,
		common_clear,
		common_noEntityMatchesFiltersOrSearch,
		common_ungrouped,
		common_tableCaption,
		common_tags,
		common_notAvailableForAllSelected,
		common_oneRowAtATime,
		common_noCommonTags,
		common_commonTags,
		tags_hotkeyEdit,
		tags_hotkeyRemove,
		common_shortcutsPausedInSearch,
		common_item,
		common_items,
		common_no,
		common_yes
	} from '$lib/paraglide/messages';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import { SearchX } from '@lucide/svelte';
	import {
		useTagsQuery,
		useTagGroupsQuery,
		useBulkAddTagMutation,
		useBulkRemoveTagMutation,
		type EntityDiscriminants
	} from '$lib/features/tags/queries';
	import { computeCommonTags } from '$lib/shared/utils/tags';
	import { MediaQuery, SvelteMap, SvelteSet } from 'svelte/reactivity';
	import throttle from 'just-throttle';
	import { isOnVisiblePage, registerEntityList } from '$lib/shared/stores/modal-registry';
	import { isShiftShortcut, keyLabel, shiftShortcutLabel } from '$lib/shared/utils/shortcuts';
	import KbdKey from '$lib/shared/components/feedback/KbdKey.svelte';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import {
		isFreeKeydown,
		isPlainKey,
		provideListShortcuts,
		tableSearchFocused
	} from './controls/listShortcuts';
	import {
		assignActionKeys,
		matchesActionKey,
		resolveTooltip,
		selectionActions,
		toolbarButtonClass,
		type KeyedAction
	} from './controls/actionShortcuts';
	import type { components } from '$lib/api/schema';

	type PaginationMeta = components['schemas']['PaginationMeta'];

	/** Debounce window for the search box, in ms. */
	const SEARCH_THROTTLE_MS = 300;

	let {
		items = $bindable([]),
		fields: tabFields = $bindable([]),
		storageKey = null,
		entityType = null,
		canEditTags = true,
		getItemTags = null,
		getItemId,
		// The page the server returned, and how the parent fetches another.
		serverPagination,
		onPageChange,
		// The request the parent sends its endpoint, in the endpoint's own param names. Called once
		// the view is restored and on every change after.
		onQueryChange,
		// Which parts of the view the server applies from that query. Absent: every part.
		serverQuery = null,
		// CSV export callback (optional, default behavior)
		// Called when user clicks export button; parent handles the actual export
		onCsvExport = null,
		// Export button click override (optional)
		// If provided, replaces onCsvExport entirely - use for custom export UI (e.g., modal with options)
		onExportClick = null,
		// Row actions, rendered in the table's trailing column.
		getActions = null,
		getRowBadges = null,
		// Names the table for screen readers, e.g. "Hosts".
		entityLabel = null,
		// The grouping and sort the tab opens with until the user picks their own.
		defaults = {},
		// The page's own actions (Create), last in the toolbar.
		toolbarActions = undefined,
		// The page title and its hover note, first in the toolbar row.
		title = null,
		subtitle = null
	}: {
		items: T[];
		fields: FieldConfig<T>[];
		storageKey?: string | null;
		entityType?: EntityDiscriminants | null;
		// Whether the user may add and remove tags here. `entityType` names what the rows are; this
		// gates editing them.
		canEditTags?: boolean;
		getItemTags?: ((item: T) => string[]) | null;
		getItemId: (item: T) => string;
		// The server's pagination for the page in `items`; null until the first page arrives.
		// `onPageChange` receives both page and pageSize so the parent can use them in its query.
		serverPagination: PaginationMeta | null;
		onPageChange: (page: number, pageSize: number) => void;
		// Server-side query: search, tags, staleness, ordering and `serverFiltered` fields, as the
		// endpoint takes them. The same params, plus `page` and `selected`, are the list's URL, so a
		// link and an API call with this query return the same rows. Server-side because the client
		// would only ever narrow the page it holds. Search is throttled.
		onQueryChange: (query: URLSearchParams) => void;
		// The parts of the view the server applies, where an endpoint takes no staleness or tags.
		serverQuery?: Partial<ServerQuery> | null;
		// CSV export: default behavior when user clicks export button
		onCsvExport?: (() => void | Promise<void>) | null;
		// Export button click override: if provided, replaces onCsvExport entirely
		onExportClick?: (() => void | Promise<void>) | null;
		// Row actions.
		getActions?: ((item: T) => RowAction[]) | null;
		/** Icon-only status badges ahead of each row's name, whatever columns are shown. */
		getRowBadges?: ((item: T) => TagProps[]) | null;
		// Accessible name for the table, e.g. "Hosts".
		entityLabel?: string | null;
		defaults?: TableDefaults;
		toolbarActions?: Snippet;
		title?: string | null;
		subtitle?: string | null;
	} = $props();

	// Tags query for filter display
	const tagsQuery = useTagsQuery();
	let allTags = $derived(tagsQuery.data ?? []);
	const tagGroupsQuery = useTagGroupsQuery();
	let tagGroups = $derived(tagGroupsQuery.data ?? []);

	/**
	 * The tab's fields, then one per tag group that applies to the rows, built here so every
	 * taggable table gets them: hidden until shown, and sorted, filtered and grouped like any other
	 * column.
	 */
	let fields = $derived<FieldConfig<T>[]>([
		...tabFields,
		...(entityType && getItemTags
			? tagGroupFields<T>({
					groups: tagGroups,
					tags: allTags,
					entityType,
					getItemTags,
					cell: canEditTags ? tagGroupCell : undefined
				})
			: [])
	]);

	/** What the server applies from the query. */
	let server = $derived<ServerQuery>({ tags: true, stale: true, search: true, ...serverQuery });

	// Bulk tag mutations
	const bulkAddTagMutation = useBulkAddTagMutation();
	const bulkRemoveTagMutation = useBulkRemoveTagMutation();

	// Search state
	let searchQuery = $state('');
	/** The search as last sent to the parent and the URL; trails `searchQuery` by the throttle. */
	let sentSearch = $state('');
	/** Whether the parent has had its first query, sent once the view is restored. */
	let queryNotified = false;

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
	let columnGroupOrder = $state<string[]>([]);
	let columnSizing = $state<Record<string, number>>({});

	// Pagination state. `currentPage` is the page restored from storage or a link, handed to the
	// parent on mount; after that the server's offset says which page is showing.
	let currentPage = $state(1);
	let pageSize = $state<PageSizeOption>(20);

	// Bulk selection state
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
		if (state.columnGroupOrder) columnGroupOrder = state.columnGroupOrder;
		if (state.columnSizing) columnSizing = state.columnSizing;

		if (state.pageSize) {
			pageSize = state.pageSize;
			return state.pageSize;
		}

		return null;
	}

	/**
	 * Apply the view a link carried, over what localStorage restored, for each param it carries.
	 * Only the list on screen takes it: every list tab is mounted, and the link describes one.
	 */
	function loadUrlView() {
		if (!isOnVisiblePage(rootRef)) return;
		applyLinkView(takeInitialParams('list'));
	}

	/** Apply a link's list view: one the page loaded with, or an in-app link to this tab. */
	function applyLinkView(params: URLSearchParams) {
		if (params.size === 0) return;
		linkParams = params;
		// Plain on purpose: the late-field effect writes it, so tracking it would re-run that effect.
		// eslint-disable-next-line svelte/prefer-svelte-reactivity
		linkFieldKeys = new Set(fields.filter(optionsReady).map(getFieldKey));
		if (!hasListParams(params, fields)) return;

		// A link describes the whole view: what it leaves out is at its default, not at whatever this
		// browser last saved.
		const view = decodeListParams(params, fields);
		searchQuery = view.search;
		filterState = reviveFilterState(view.filterState);
		staleOnly = view.staleOnly;
		ordering = { sort: view.sort, group: view.group };
		currentPage = view.page ?? 1;

		seedFilterDefaults = false;
	}

	/**
	 * The link the list opened with, kept for fields that arrive after mount (tag-group columns wait
	 * on the tag groups), whose filters it applies once they exist.
	 */
	let linkParams: URLSearchParams | null = null;
	let linkFieldKeys = new Set<string>();

	/** Whether a field can read its params: a tag group can't before its tags load. */
	function optionsReady(field: FieldConfig<T>): boolean {
		return field.filterOptions === undefined || field.filterOptions.length > 0;
	}

	$effect(() => {
		if (!linkParams) return;
		const late = fields.filter(
			(field) => !linkFieldKeys.has(getFieldKey(field)) && optionsReady(field)
		);
		if (late.length === 0) return;
		late.forEach((field) => linkFieldKeys.add(getFieldKey(field)));
		const filters = decodeFilters(linkParams, late);
		if (Object.keys(filters).length > 0) {
			filterState = { ...filterState, ...reviveFilterState(filters) };
		}
	});

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
					currentPage: effectiveCurrentPage,
					pageSize,
					columnVisibility,
					columnOrder,
					columnGroupOrder,
					columnSizing
				})
			);
		} catch (e) {
			console.warn('Failed to save DataControls state to localStorage:', e);
		}
	}

	/**
	 * Whether a field missing from the filters starts at its `filterDefaults`. Not under a link: it
	 * lists every active filter, so a field it leaves out had none.
	 */
	let seedFilterDefaults = true;

	// Seed a filter for any field that lacks one, leaving restored state alone.
	$effect(() => {
		const seeded = blankFilterState(fields, seedFilterDefaults);
		for (const [key, filter] of Object.entries(seeded)) {
			if (!filterState[key]) filterState[key] = filter;
		}
	});

	// Load state on mount and set up auto-save
	onMount(() => {
		const restoredPageSize = loadState();
		loadUrlView();

		// Before the first query reaches the parent, so a dropped filter is simply never sent.
		dropHiddenColumns();
		sentSearch = searchQuery;

		// The parent's query has to use the restored page and page size.
		if (restoredPageSize || currentPage !== 1) {
			onPageChange(currentPage, pageSize);
		}

		// Set up reactive save (debounced)
		let saveTimeout: ReturnType<typeof setTimeout>;
		let savePending = false;

		const unsubscribe = $effect.root(() => {
			$effect(() => {
				if (storageKey) {
					// Track all state that should trigger saves
					void searchQuery;
					void filterState;
					void ordering;
					void effectiveCurrentPage;
					void pageSize;
					void columnVisibility;
					void columnOrder;
					void columnGroupOrder;
					void columnSizing;

					// Debounce saves
					clearTimeout(saveTimeout);
					savePending = true;
					saveTimeout = setTimeout(() => {
						savePending = false;
						saveState();
					}, 100);
				}
			});
		});

		return () => {
			clearTimeout(saveTimeout);
			// Save a change still in its debounce, or a remount would restore the view before it.
			if (savePending) saveState();
			unsubscribe();
		};
	});

	// The values a field's filter offers, ordered by label
	function getFilterOptions(field: FieldConfig<T>): string[] {
		return filterOptionsOf(items, field);
	}

	// ---- Table columns -------------------------------------------------------

	let allColumns = $derived(fieldsToColumns(fields));
	/**
	 * Tags are appended by the list itself rather than declared per tab.
	 *
	 * Every taggable entity gets the same editable column in the same place —
	 * last, next to the actions — instead of each tab remembering to add one, so
	 * a tab cannot silently end up without it. It is editable only when the
	 * parent supplied an `entityType` and allows `canEditTags`.
	 */
	let tagColumn = $derived(
		getItemTags ? buildTagColumn<T>(common_tags(), getItemTags, tagsCell) : null
	);

	let columnState = $derived(
		reconcileColumnState(
			allColumns,
			{
				visibility: columnVisibility,
				order: columnOrder,
				groupOrder: columnGroupOrder,
				sizing: columnSizing
			},
			tagColumn ? [tagColumn] : []
		)
	);

	let renderedColumns = $derived(withTagColumn(visibleColumns(allColumns, columnState), tagColumn));
	let renderedColumnIds = $derived(new Set(renderedColumns.map((c) => c.id)));
	/** The fields the list filters, sorts and groups by: those with a rendered column. */
	let columnFields = $derived(renderedFields(fields, renderedColumnIds));
	let staleAvailable = $derived(server.stale);

	// Only server-orderable fields whose column renders are offered: the client holds one page, and
	// hiding a column takes its controls too.
	let groupableFields = $derived(groupableFieldsOf(columnFields));
	let sortableFields = $derived(sortableFieldsOf(columnFields));

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

	// The table leaves out the column it groups by, which still counts as rendered, so its filter,
	// sort and group stand. Ungrouping brings it back.
	let tableColumns = $derived(columnsWhileGrouped(renderedColumns, activeGroupField));
	/** The label of the column the list groups by, which the ungroup button names. */
	let activeGroupLabel = $derived(
		activeGroupField ? (allColumns.find((c) => c.id === activeGroupField)?.label ?? null) : null
	);

	// Per-group totals across every page, when the server supplied them.
	let serverGroupCounts = $derived(serverPagination?.group_counts ?? null);

	// Group items by selected field
	let groupedItems = $derived.by(() => {
		if (!activeGroupField) {
			return new SvelteMap([[common_all(), items]]);
		}

		// The page arrives in the server's group order, which the cumulative offsets from
		// `serverGroupCounts` are indexed by, so its buckets keep that order.
		return groupItemsBy(items, fields, activeGroupField, {
			ungrouped: common_ungrouped(),
			yes: common_yes(),
			no: common_no()
		});
	});

	/**
	 * Each group's nested sections when the grouping is a tree, built here where the order is
	 * decided rather than in the renderers. Null otherwise.
	 */
	let treeEntries = $derived.by(() => {
		if (!activeTree) return null;
		const tree = activeTree;
		const entries = new SvelteMap<string, TreeEntry<T>[]>();
		for (const [name, rows] of groupedItems) {
			entries.set(name, buildTreeSections(rows, tree, name));
		}
		return entries;
	});

	/** The active group field, when its groups are trees. */
	let activeTree = $derived.by(() => {
		const field = fields.find((f) => getFieldKey(f) === activeGroupField);
		return field?.tree ?? null;
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
	 * is. Null when the server didn't supply totals.
	 */
	function groupRange(groupItems: T[]): GroupSlice | null {
		if (!serverPagination) return null;

		const group = groupOffsets.get(serverGroupKey(groupItems));
		if (!group) return null;

		return groupPageSlice(group, serverPagination.offset, items.length);
	}

	/** The groups as the table renders them. */
	let groupList = $derived.by<RenderGroup<T>[]>(() => {
		const groups = [...groupedItems.entries()].map(([name, groupItems]) => {
			const entries = treeEntries?.get(name) ?? null;
			return {
				key: name,
				name,
				items: entries ? flattenTreeEntries(entries) : groupItems,
				range: groupRange(groupItems),
				entries,
				rootRanges: null
			};
		});
		if (!activeTree) return groups;
		// The trees together, across every page: the merged header's range.
		const trees = rootedTreesPosition(serverGroupCounts);
		const range =
			trees && serverPagination
				? groupPageSlice(trees, serverPagination.offset, items.length)
				: null;
		return mergeRootedTrees(groups, activeTree, range);
	});

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
		}
	});

	/**
	 * Clear whatever hidden columns filter, sort or group by. A list only
	 * narrows and orders by columns it renders, so hiding one takes its state
	 * with it. The query derived from that state carries the change to the parent.
	 *
	 * Only the user's own sort and group are cleared. A default on a hidden
	 * column is already inert, since the list orders by rendered columns only,
	 * and it comes back with the column.
	 */
	function dropHiddenColumns() {
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
	}

	function clearSearch() {
		searchQuery = '';
	}

	/**
	 * Everything that can narrow the list, in one action, server-side ones
	 * included. This is what the filtered-empty state offers, so a filter
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

	/** Runs a row action's `onBulk` on the selection. */
	function runOnSelection(action: RowAction) {
		return async () => {
			const done = await runBulkAction(action.label, selectedIds, Boolean(action.onBulk), (ids) =>
				Promise.resolve(action.onBulk!(ids))
			);
			// A delete clears the selection, its rows being gone. Only on success: a failed delete
			// leaves the rows, and the user still needs the selection to retry.
			if (done && action.isDelete) selectedIds.clear();
		};
	}

	function bulkTagAction(description: string, mutate: typeof bulkAddTagMutation) {
		return (tagId: string) =>
			runBulkAction(description, selectedIds, canEditTags && entityType !== null, (ids) =>
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

	let hasBulkTagging = $derived(canEditTags && entityType !== null && getItemTags !== null);

	let hasActiveFilters = $derived(hasActiveFiltersOf(fields, filterState, staleOnly));
	let activeFilterCount = $derived(activeFilterCountOf(fields, filterState, staleOnly));

	let hasActiveSearch = $derived(searchQuery.trim().length > 0);
	let hasActiveGrouping = $derived(activeGroupField !== null);

	guardServerFilters(() => ({
		fields,
		server: { tags: server.tags }
	}));
	guardTableDefaults(() => ({ fields, defaults }));

	// Every pagination number comes from the server's metadata, whose total already accounts for
	// search and filters, so the count and the rows beneath it cannot fall out of step.
	let page = $derived(paginationView(serverPagination, pageSize, items.length));
	let effectiveCurrentPage = $derived(page.currentPage);
	let totalPages = $derived(page.totalPages);
	let totalCount = $derived(page.totalCount);
	/** The filters and search match nothing, so the table gives way to an empty state with Clear. */
	let matchesNothing = $derived(totalCount === 0 && (hasActiveFilters || hasActiveSearch));

	/** Every row in the order the table renders it: the current page, or each group's rows in turn. */
	let orderedItems = $derived(
		hasActiveGrouping ? groupList.flatMap((group) => group.items) : items
	);

	// Lets an open entity modal step to the previous or next row with the arrow keys.
	$effect(() =>
		registerEntityList({
			ids: () => orderedItems.map(getItemId),
			isVisible: () => isOnVisiblePage(sentinelRef)
		})
	);

	// The view as URL params, so the address bar is a link to it. Written only while this list is
	// the one on screen; switching tabs puts the next list's view there instead.
	// The URL is the view's params plus the page, which the endpoint doesn't take: a page is an
	// offset only with the viewer's own page size. The selection stays out of the URL.
	let urlView = $derived.by(() => {
		// A fresh copy built and returned here; nothing mutates it later.
		// eslint-disable-next-line svelte/prefer-svelte-reactivity
		const params = new URLSearchParams(viewParams.url);
		if (effectiveCurrentPage > 1) params.set(PAGE_PARAM, String(effectiveCurrentPage));
		return params;
	});

	$effect(() =>
		registerUrlOwner({
			scope: 'list',
			isActive: () => isOnVisiblePage(rootRef),
			params: () => urlView,
			keys: () => listParamKeys(fields),
			// An in-app link to this tab (a nudge's filtered list), taken as the tab comes on screen.
			restore: (params) => applyLinkView(params)
		})
	);

	// Debounced like the localStorage save: Safari throttles a burst of history writes.
	let urlSyncTimeout: ReturnType<typeof setTimeout> | undefined;
	$effect(() => {
		void urlView;
		clearTimeout(urlSyncTimeout);
		urlSyncTimeout = setTimeout(() => {
			if (isOnVisiblePage(rootRef)) syncUrl();
		}, 100);
		return () => clearTimeout(urlSyncTimeout);
	});

	/** The row the last plain click toggled: one end of a Shift+click block. */
	let selectionAnchor: string | null = null;

	/**
	 * A click on a row or its checkbox. A plain click toggles the row and makes it the anchor; a
	 * Shift+click selects every row from the anchor to it, in the order on screen. With no anchor
	 * on screen (paged away, filtered out) a Shift+click toggles like a plain one.
	 */
	function clickRow(itemId: string, extend: boolean) {
		if (extend && selectionAnchor) {
			const block = rangeSelect(orderedItems, selectionAnchor, itemId, getItemId);
			if (block.length > 0) {
				block.forEach((id) => selectedIds.add(id));
				return;
			}
		}
		if (selectedIds.has(itemId)) selectedIds.delete(itemId);
		else selectedIds.add(itemId);
		selectionAnchor = itemId;
	}

	/** Select-all scoped to one rendered block — a group's rows, or the page. */
	function toggleAllIn(rows: T[]) {
		if (isAllSelected(rows, selectedIds, getItemId)) {
			visibleIds(rows, getItemId).forEach((id) => selectedIds.delete(id));
		} else {
			visibleIds(rows, getItemId).forEach((id) => selectedIds.add(id));
		}
	}

	// Rows are selectable when the selection has something to act on: tags, or a bulk row action.
	let hasBulkActions = $derived(
		items.some((item) => getActions?.(item).some((action) => action.onBulk) ?? false)
	);
	let showSelection = $derived(hasBulkActions || hasBulkTagging);

	let tableCaptionText = $derived(
		common_tableCaption({
			entity: entityLabel ?? '',
			count: totalCount,
			itemLabel: totalCount === 1 ? common_item() : common_items()
		})
	);

	function toggleColumn(id: string) {
		columnVisibility = toggledColumnVisibility(allColumns, columnState.visibility, id);
		dropHiddenColumns();
	}

	function reorderColumns(order: string[]) {
		columnOrder = order;
	}

	function reorderColumnGroups(groupOrder: string[]) {
		columnGroupOrder = groupOrder;
	}

	function resizeColumns(sizing: Record<string, number>) {
		columnSizing = sizing;
	}

	function resetColumns() {
		columnVisibility = {};
		columnOrder = [];
		columnGroupOrder = [];
		columnSizing = {};
		dropHiddenColumns();
	}

	// Reset to page 1 when filters/search change and current page would be out of bounds
	$effect(() => {
		if (effectiveCurrentPage > totalPages && totalPages > 0) {
			resetToFirstPage();
		}
	});

	// The view as the parent's endpoint and the URL take it, with the search as last sent: the box
	// reaches both on a trailing throttle, so a burst of keystrokes costs one request and the last
	// one is never dropped. Built once; rebuilding it per keystroke would defeat it.
	const sendSearch = throttle(
		(query: string) => {
			sentSearch = query;
		},
		SEARCH_THROTTLE_MS,
		{ leading: false, trailing: true }
	);

	const searchTracker = createChangeTracker<string>();
	$effect(() => {
		if (searchTracker.changed(searchQuery)) sendSearch(searchQuery);
	});

	let viewParams = $derived(
		listParams(
			{
				sort: activeSort,
				group: activeGroupField,
				filterState,
				staleOnly,
				search: sentSearch
			},
			fields,
			server
		)
	);

	// The first run sends the restored view and keeps its page. After that, any change to the view
	// starts again at page 1: page 3 of the old result set is not page 3 of the new one.
	const viewTracker = createChangeTracker<string>();
	$effect(() => {
		const changed = viewTracker.changed(viewParams.url.toString());
		if (changed) resetToFirstPage();
		if (changed || !queryNotified) {
			queryNotified = true;
			onQueryChange(viewParams.request);
		}
	});

	/** Move to a page: the parent fetches it. */
	function goToPage(target: number, size: PageSizeOption = pageSize) {
		onPageChange(target, size);
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

	// Leaving the list for another tab clears its selection, so coming back doesn't hold rows the
	// user stopped working with. A hidden tab's wrapper clips the list to nothing, which the
	// observer reports as an intersection ratio of 0; the tiny threshold makes that drop fire.
	$effect(() => {
		if (!rootRef) return;
		const observer = new IntersectionObserver(
			([entry]) => {
				if (entry.intersectionRatio === 0 && selectedIds.size > 0) {
					selectedIds.clear();
					selectionAnchor = null;
				}
			},
			{ root: rootRef.closest('main'), threshold: [0, 0.000001] }
		);
		observer.observe(rootRef);
		return () => observer.disconnect();
	});

	// The bulk-tag island is fixed to the bottom of the viewport, and centred over this list
	// rather than the window, so it tracks the list's horizontal extent (sidebar collapse, resize).
	let rootRef: HTMLDivElement | null = $state(null);
	/** The island's height, for the space the table keeps below its last row. */
	let islandHeight = $state(0);
	/** The island's gap above the viewport's bottom edge (`bottom-6`), plus 16px of clearance. */
	const ISLAND_INSET = 40;
	let islandFrame = $state({ left: 0, width: 0 });

	$effect(() => {
		if (!rootRef) return;
		const root = rootRef;
		const observer = new ResizeObserver(() => {
			const rect = root.getBoundingClientRect();
			islandFrame = { left: rect.left, width: rect.width };
		});
		observer.observe(root);
		return () => observer.disconnect();
	});

	// T focuses the island's tag input while rows are selected. From there Shift+Tab reaches the
	// held tags, and Delete or Backspace removes the focused one.
	const TAG_SHORTCUT = 'T';
	/** The table search has focus, so the shortcuts below are off; their hints dim to say so. */
	let searchFocused = $state(false);
	// Shared with the rest of the page; cleared if the list unmounts with its search focused.
	$effect(() => {
		tableSearchFocused.set(searchFocused);
		return () => {
			if (searchFocused) tableSearchFocused.set(false);
		};
	});
	/** Focus is in the tag island (T put it there, or a click): its tags can be removed from the keyboard. */
	let editingTags = $state(false);
	let bulkTagPicker: { focusInput: () => void } | undefined = $state();
	// Below `md` the selection's actions and tags fold into menus in the toolbar (`SelectionMenus`)
	// instead of a row of buttons and an island floating over a phone-sized table.
	const narrowViewport = new MediaQuery('max-width: 767px');
	let showTagIsland = $derived(hasBulkTagging && selectedIds.size > 0 && !narrowViewport.current);
	// The island unmounts without a focusout when the selection clears.
	$effect(() => {
		if (!showTagIsland) editingTags = false;
	});

	// List shortcuts, each shown on the control it triggers. Single keys and Shift+letter, so they
	// stand down while typing in a field, when a modal is over the list, or when the list isn't
	// the tab on screen. Esc clears the search from anywhere on the list; ControlsBar handles it
	// inside the search box.
	const CLEAR_FILTERS_KEY = 'F';
	const CLEAR_GROUPING_KEY = 'G';
	// Clear in the nothing-matches state: filters and search together.
	const CLEAR_ALL_KEY = 'X';
	// Left and Right page. Not while a wide table's scroll region has keyboard focus, where the
	// arrows scroll it sideways; a click's focus there pages, since the mouse scrolls it anyway.
	const PREV_PAGE_KEY = 'ArrowLeft';
	const NEXT_PAGE_KEY = 'ArrowRight';

	function arrowsScrollTable(event: KeyboardEvent): boolean {
		const region = (event.target as Element | null)?.closest?.('.entity-table');
		return (
			region instanceof HTMLElement &&
			region.scrollWidth > region.clientWidth &&
			!region.hasAttribute('data-pointer-focus')
		);
	}
	const clearShortcuts = {
		filters: shiftShortcutLabel(CLEAR_FILTERS_KEY),
		grouping: shiftShortcutLabel(CLEAR_GROUPING_KEY),
		selection: keyLabel('Escape')
	};

	// The selection's actions in the toolbar, each with a key, built from the selected rows' own
	// actions: one row's actions as its row shows them, or the bulk actions for several.
	let selectionToolbarActions = $derived.by((): KeyedAction[] => {
		if (!getActions || selectedIds.size === 0) return [];
		const rows = items.filter((item) => selectedIds.has(getItemId(item)));
		if (rows.length === 0) return [];
		return assignActionKeys(
			selectionActions(
				rows.map((item) => getActions(item)),
				runOnSelection,
				{
					missing: common_notAvailableForAllSelected(),
					oneRowAtATime: common_oneRowAtATime()
				}
			)
		);
	});

	// Toolbar controls (CreateButton) take their own keys through this.
	provideListShortcuts({
		isActive: () => isOnVisiblePage(rootRef),
		isPaused: () => searchFocused
	});

	/** The toolbar action a key press runs: the first matching one available, else the first match. */
	function selectionActionFor(event: KeyboardEvent): KeyedAction | undefined {
		const matches = selectionToolbarActions.filter(
			({ key }) => key !== null && matchesActionKey(event, key)
		);
		return matches.find(({ action }) => !action.unavailable) ?? matches[0];
	}

	function handleListShortcut(event: KeyboardEvent) {
		if (!isFreeKeydown(event) || !isOnVisiblePage(rootRef)) return;
		const plain = !event.metaKey && !event.ctrlKey && !event.altKey;
		if (
			showSelection &&
			(event.metaKey || event.ctrlKey) &&
			!event.altKey &&
			!event.shiftKey &&
			event.key.toLowerCase() === 'a'
		) {
			// Cmd/Ctrl+A selects every row on the page instead of the page's text.
			event.preventDefault();
			visibleIds(orderedItems, getItemId).forEach((id) => selectedIds.add(id));
		} else if (selectionActionFor(event)) {
			// The delete asks for confirmation, as its button does.
			const keyed = selectionActionFor(event)!;
			event.preventDefault();
			if (!keyed.action.unavailable) keyed.action.onClick();
		} else if (showTagIsland && isPlainKey(event, TAG_SHORTCUT)) {
			event.preventDefault();
			bulkTagPicker?.focusInput();
		} else if (showSelection && selectedIds.size > 0 && plain && event.key === 'Escape') {
			event.preventDefault();
			selectNone();
		} else if (matchesNothing && isPlainKey(event, CLEAR_ALL_KEY)) {
			event.preventDefault();
			clearAllNarrowing();
		} else if (plain && !event.shiftKey && event.key === 'Escape' && hasActiveSearch) {
			// Esc clears the search from anywhere on the list, not only from the search box. A
			// selection, when there is one, clears first (above).
			event.preventDefault();
			clearSearch();
		} else if (activeFilterCount > 0 && isShiftShortcut(event, CLEAR_FILTERS_KEY)) {
			event.preventDefault();
			filters.clearAll();
		} else if (hasActiveGrouping && isShiftShortcut(event, CLEAR_GROUPING_KEY)) {
			event.preventDefault();
			ordering = { ...ordering, group: null };
		} else if (
			plain &&
			!event.shiftKey &&
			event.key === PREV_PAGE_KEY &&
			page.canGoPrev &&
			!arrowsScrollTable(event)
		) {
			event.preventDefault();
			goToPage(effectiveCurrentPage - 1);
		} else if (
			plain &&
			!event.shiftKey &&
			event.key === NEXT_PAGE_KEY &&
			page.canGoNext &&
			!arrowsScrollTable(event)
		) {
			event.preventDefault();
			goToPage(effectiveCurrentPage + 1);
		}
	}

	/**
	 * A click on anything that looks like empty space clears the selection, including the
	 * toolbar's and the table header's background. Clicks that keep it: a row
	 * (`data-keeps-selection`), the tag island; any control (a link, button or field, including a
	 * dropdown portalled out of the page); anything while a modal is open; and anything outside
	 * the page's scroll area, such as the sidebar.
	 */
	function handleClickOutside(event: MouseEvent) {
		if (selectedIds.size === 0 || !rootRef || !isOnVisiblePage(rootRef)) return;
		const target = event.target as Element | null;
		if (!target || !rootRef.closest('main')?.contains(target)) return;
		if (
			target.closest(
				'[data-keeps-selection], a, button, input, select, textarea, label, [role="button"], [role="dialog"]'
			) ||
			document.querySelector('[aria-modal="true"]')
		) {
			return;
		}
		selectNone();
	}
</script>

<svelte:window onkeydown={handleListShortcut} onclick={handleClickOutside} />

<!-- With rows selected: the selection's actions, as their rows show them. -->
{#snippet selectionToolbar()}
	{#if narrowViewport.current}
		<SelectionMenus
			actions={selectionToolbarActions.map(({ action }) => action)}
			selectedCount={selectedIds.size}
			tagging={hasBulkTagging && entityType
				? {
						commonTags,
						entityType,
						onAdd: handleBulkTagAdd,
						onRemove: handleBulkTagRemove
					}
				: null}
		/>
	{:else}
		{#each selectionToolbarActions as { action, key } (action.label)}
			<button
				onclick={action.onClick}
				disabled={!!action.unavailable}
				use:tooltip
				data-tooltip={resolveTooltip(action)}
				class="{toolbarButtonClass(
					action.tone
				)} toolbar-control flex items-center gap-2 disabled:cursor-not-allowed disabled:opacity-50"
			>
				{action.label}
				{#if key}
					<span
						class="inline-flex transition-opacity {searchFocused ? 'opacity-40' : ''}"
						title={searchFocused ? common_shortcutsPausedInSearch() : undefined}
					>
						<KbdKey key={keyLabel(key)} size="xs" tone="inherit" />
					</span>
				{/if}
			</button>
		{/each}
	{/if}
{/snippet}

<div bind:this={rootRef} class="space-y-4">
	<!-- Sentinel for sticky detection -->
	<!-- `mb-0!`: `space-y-4` gives every child but the last a bottom margin, so the zero-height
	     sentinel would otherwise push the toolbar 16px below the sidebar's first row. -->
	<div bind:this={sentinelRef} class="mb-0! h-0 w-full"></div>

	<!-- Sticky Controls Bar -->
	<div
		class="z-20 -mx-4 border-b bg-(--color-bg-body) px-4 pb-4 md:sticky md:top-0 {isStuck
			? 'border-(--color-border) pt-4 shadow-lg'
			: 'border-transparent'}"
	>
		<ControlsBar
			bind:searchQuery
			search={server.search}
			onSearchFocusChange={(focused) => (searchFocused = focused)}
			onClearSearch={clearSearch}
			{entityLabel}
			actions={selectedIds.size > 0 ? undefined : toolbarActions}
			selectionActions={selectionToolbarActions.length > 0 ||
			(narrowViewport.current && hasBulkTagging && selectedIds.size > 0)
				? selectionToolbar
				: undefined}
			{title}
			{subtitle}
		/>
		<!-- Count, filter and grouping clears, and paging stay in view with the toolbar. -->
		<div class="mt-4">
			<PaginationBar
				{totalCount}
				{totalPages}
				{pageSize}
				showingStart={page.showingStart}
				showingEnd={page.showingEnd}
				canGoPrev={page.canGoPrev}
				canGoNext={page.canGoNext}
				groupCount={hasActiveGrouping ? groupedItems.size : null}
				groupLabel={activeGroupLabel}
				filterCount={activeFilterCount}
				onClearFilters={filters.clearAll}
				onClearGrouping={() => (ordering = { ...ordering, group: null })}
				selectedCount={showSelection ? selectedIds.size : 0}
				onClearSelection={selectNone}
				{clearShortcuts}
				shortcutsPaused={searchFocused}
				onPrevPage={() => page.canGoPrev && goToPage(effectiveCurrentPage - 1)}
				onNextPage={() => page.canGoNext && goToPage(effectiveCurrentPage + 1)}
				onPageSizeChange={handlePageSizeChange}
				onExport={onExportClick ?? onCsvExport}
				pageShortcuts={{ prev: keyLabel(PREV_PAGE_KEY), next: keyLabel(NEXT_PAGE_KEY) }}
			>
				{#snippet columnMenu()}
					<ColumnVisibilityMenu
						columns={allColumns}
						order={columnState.order}
						groupOrder={columnState.groupOrder}
						visibility={columnState.visibility}
						groupedColumnId={activeGroupField}
						onToggle={toggleColumn}
						onReorder={reorderColumns}
						onReorderGroups={reorderColumnGroups}
						onReset={resetColumns}
					/>
				{/snippet}
			</PaginationBar>
		</div>
	</div>

	<!-- Content -->
	{#if matchesNothing}
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
			title={common_noEntityMatchesFiltersOrSearch({ entity: entityLabel ?? common_items() })}
		>
			<button onclick={clearAllNarrowing} class="btn-secondary">
				{common_clear()}
				<KbdKey
					key={CLEAR_ALL_KEY}
					size="xs"
					tone="inherit"
					class="ml-1.5 transition-opacity {searchFocused ? 'opacity-40' : ''}"
				/>
			</button>
		</EmptyState>
	{:else}
		<!--
			Grouped or not, one table with one header row. Splitting a grouped list
			into a table per group gave each group its own header and its own column
			widths, so columns stopped lining up across the very groups you were
			comparing — which is the whole reason to use a table.
		-->
		{@render tableFor(
			hasActiveGrouping ? null : items,
			hasActiveGrouping ? null : tableCaptionText
		)}
	{/if}

	<!-- Bulk tagging floats at the bottom of the viewport, over the table, so a long tag list never
	     crowds the toolbar. A heavier shadow and an edge ring lift it off the rows; in dark mode an
	     opaque surface replaces the card's translucent one, which let the rows show through.
	     Fixed rather than sticky: sticky only pins once the table outgrows the viewport, so a short
	     table left it under the last row. A hidden tab's wrapper sets `contain: paint`, which
	     clips a fixed descendant. -->
	{#if showTagIsland}
		<div
			class="pointer-events-none fixed bottom-6 z-20 flex justify-center"
			style:left="{islandFrame.left}px"
			style:width="{islandFrame.width}px"
		>
			<div
				data-keeps-selection
				bind:offsetHeight={islandHeight}
				onfocusin={() => (editingTags = true)}
				onfocusout={(e) => {
					if (!e.currentTarget.contains(e.relatedTarget as Node | null)) editingTags = false;
				}}
				class="card-static card pointer-events-auto flex max-w-full items-center gap-2 px-4! py-2! shadow-[0_10px_40px_rgba(0,0,0,0.25)]! ring-1 ring-black/5 dark:border-gray-600! dark:bg-gray-800! dark:shadow-[0_10px_40px_rgba(0,0,0,0.7)]! dark:ring-white/10"
			>
				<span class="text-sm whitespace-nowrap text-secondary"
					>{selectedIds.size > 1 ? common_commonTags() : common_tags()}</span
				>
				<TagPickerInline
					bind:this={bulkTagPicker}
					selectedTagIds={commonTags}
					entityType={entityType ?? undefined}
					onAdd={handleBulkTagAdd}
					onRemove={handleBulkTagRemove}
				/>
				{#if commonTags.length === 0 && selectedIds.size > 1}
					<span class="text-xs whitespace-nowrap text-tertiary">{common_noCommonTags()}</span>
				{/if}
				<span
					class="ml-2 flex items-center gap-1.5 border-l border-(--color-border) pl-3 text-xs whitespace-nowrap text-tertiary"
				>
					<span
						class="flex items-center gap-1.5 transition-opacity {searchFocused ? 'opacity-40' : ''}"
						title={searchFocused ? common_shortcutsPausedInSearch() : undefined}
					>
						<KbdKey key={TAG_SHORTCUT} size="sm" />
						{tags_hotkeyEdit()}
					</span>
					{#if editingTags && commonTags.length > 0}
						<KbdKey key="{keyLabel('Shift')} Tab" size="sm" class="ml-2" />
						<KbdKey key={keyLabel('Delete')} size="sm" />
						{tags_hotkeyRemove()}
					{/if}
				</span>
			</div>
		</div>
	{/if}
</div>

{#snippet tagsCell(item: T)}
	{@const ids = getItemTags ? getItemTags(item) : []}
	<TagCell
		items={tagItems(ids, allTags, tagGroups)}
		tagIds={ids}
		entityId={getItemId(item)}
		entityType={entityType ?? undefined}
		editable={canEditTags && Boolean(entityType)}
	/>
{/snippet}

{#snippet tagGroupCell(item: T, columnId: string)}
	{@const groupId = tagGroupIdOf(columnId)}
	{@const groupTags = allTags.filter((tag) => tag.tag_group_id === groupId)}
	{@const ids = getItemTags ? getItemTags(item) : []}
	<TagPickerInline
		selectedTagIds={groupTags.filter((tag) => ids.includes(tag.id)).map((tag) => tag.id)}
		availableTags={groupTags}
		entityId={getItemId(item)}
		entityType={entityType ?? undefined}
		disabled={!entityType}
		allowCreate={false}
	/>
{/snippet}

{#snippet tableFor(rows: T[] | null, caption: string | null)}
	{@const flat = rows ?? orderedItems}
	<EntityTable
		items={rows}
		groups={rows ? null : groupList}
		collapsed={collapsedSections}
		onToggleCollapse={toggleSection}
		columns={tableColumns}
		columnSizing={columnState.sizing}
		onColumnSizingChange={resizeColumns}
		sortState={activeSort}
		selectable={showSelection}
		{selectedIds}
		allSelected={isAllSelected(flat, selectedIds, getItemId)}
		someSelected={isPartiallySelected(flat, selectedIds, getItemId)}
		{getItemId}
		{getActions}
		{getRowBadges}
		caption={caption ?? tableCaptionText}
		onToggleSort={toggleSort}
		onClickRow={clickRow}
		onToggleAll={() => toggleAllIn(flat)}
		{headerControl}
		bottomInset={showTagIsland ? islandHeight + ISLAND_INSET : 0}
	/>
{/snippet}

{#snippet headerControl(column: EntityColumn<T>)}
	{@const control = columnControls(column.id, fields, staleAvailable)}
	{#if control}
		<ColumnControlsMenu
			{control}
			label={column.label}
			{filterState}
			{staleOnly}
			{activeGroupField}
			{allTags}
			{tagGroups}
			{getFilterOptions}
			{filters}
			onToggleGroup={toggleGroup}
		/>
	{/if}
{/snippet}
