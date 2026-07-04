<script lang="ts" generics="T">
	import {
		Search,
		X,
		ChevronLeft,
		ChevronRight,
		LayoutGrid,
		List,
		Trash2,
		CheckSquare,
		Square,
		Download,
		Filter,
		ArrowUpNarrowWide,
		ArrowDownWideNarrow
	} from 'lucide-svelte';
	import {
		type FieldConfig,
		isOrderableField,
		isDisplayField,
		getFieldKey,
		PAGE_SIZE_OPTIONS,
		type PageSizeOption
	} from './types';
	import { onMount, type Snippet } from 'svelte';
	import Tag from './Tag.svelte';
	import {
		common_active,
		common_all,
		common_clearAll,
		common_clearSelection,
		common_deleteSelected,
		common_deselectAll,
		common_filters,
		common_group,
		common_groupByLabel,
		common_groups,
		common_item,
		common_items,
		common_itemsSelected,
		common_nextPage,
		common_noCommonTags,
		common_noItems,
		common_noTagsAvailable,
		common_noValuesAvailable,
		common_none,
		common_pageOf,
		common_previousPage,
		common_searchPlaceholder,
		common_selectAll,
		common_show,
		common_showFalse,
		common_showTrue,
		common_showingRange,
		common_showingTotal,
		common_sortByLabel,
		common_switchToCardView,
		common_switchToListView,
		common_tags,
		common_ungrouped
	} from '$lib/paraglide/messages';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import {
		useTagsQuery,
		useBulkAddTagMutation,
		useBulkRemoveTagMutation,
		type EntityDiscriminants
	} from '$lib/features/tags/queries';
	import type { Color } from '$lib/shared/utils/styling';
	import { scrollFade } from '$lib/shared/utils/scrollFade';
	import { computeCommonTags } from '$lib/shared/utils/tags';
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';
	import type { components } from '$lib/api/schema';

	type PaginationMeta = components['schemas']['PaginationMeta'];

	let {
		items = $bindable([]),
		fields = $bindable([]),
		storageKey = null,
		onBulkDelete = null,
		allowBulkDelete = true,
		entityType = null,
		getItemTags = null,
		children,
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
		// Server-side exclude filter callback (optional)
		// Called when an exclude-mode filter changes, with field key and excluded values
		onExcludeFilterChange = null,
		// CSV export callback (optional, default behavior)
		// Called when user clicks export button; parent handles the actual export
		onCsvExport = null,
		// Export button click override (optional)
		// If provided, replaces onCsvExport entirely - use for custom export UI (e.g., modal with options)
		onExportClick = null
	}: {
		items: T[];
		fields: FieldConfig<T>[];
		storageKey?: string | null;
		onBulkDelete?: ((ids: string[]) => Promise<void>) | null;
		allowBulkDelete?: boolean;
		entityType?: EntityDiscriminants | null;
		getItemTags?: ((item: T) => string[]) | null;
		children: Snippet<[T, 'card' | 'list', boolean, (selected: boolean) => void]>;
		getItemId: (item: T) => string;
		// Server-side pagination: when provided, pagination is server-controlled
		// Callback receives both page and pageSize so parent can use in query
		serverPagination?: PaginationMeta | null;
		onPageChange?: ((page: number, pageSize: number) => void) | null;
		// Server-side ordering: called when group/sort changes
		// Args: (groupBy field key, orderBy field key, direction)
		onOrderChange?:
			| ((groupBy: string | null, orderBy: string | null, direction: 'asc' | 'desc') => void)
			| null;
		// Server-side tag filtering: called when tag filter changes
		// Args: array of tag IDs to filter by
		onTagFilterChange?: ((tagIds: string[]) => void) | null;
		// Server-side exclude filter: called when exclude-mode filter changes
		// Args: (fieldKey, array of excluded values)
		onExcludeFilterChange?: ((fieldKey: string, values: string[]) => void) | null;
		// CSV export: default behavior when user clicks export button
		onCsvExport?: (() => void | Promise<void>) | null;
		// Export button click override: if provided, replaces onCsvExport entirely
		onExportClick?: (() => void | Promise<void>) | null;
	} = $props();

	// Tags query for filter display
	const tagsQuery = useTagsQuery();
	let allTags = $derived(tagsQuery.data ?? []);

	// Bulk tag mutations
	const bulkAddTagMutation = useBulkAddTagMutation();
	const bulkRemoveTagMutation = useBulkRemoveTagMutation();

	// Search state
	let searchQuery = $state('');

	// Filter state
	interface FilterState {
		[key: string]: {
			type: 'string' | 'boolean' | 'array';
			values: SvelteSet<string>;
			showTrue?: boolean;
			showFalse?: boolean;
		};
	}

	let filterState = $state<FilterState>({});
	let showFilters = $state(false);

	// Sort state
	interface SortState {
		field: string | null;
		direction: 'asc' | 'desc';
	}

	let sortState = $state<SortState>({
		field: null,
		direction: 'asc'
	});

	// Grouping state
	let selectedGroupField = $state<string | null>(null);

	// View mode state
	let viewMode = $state<'card' | 'list'>('card');

	// Pagination state
	let currentPage = $state(1);
	let pageSize = $state<PageSizeOption>(20);

	// Bulk selection state (always enabled when onBulkDelete is provided)
	let selectedIds = new SvelteSet<string>();

	// Serializable version of state for localStorage
	interface SerializableState {
		searchQuery: string;
		filterState: {
			[key: string]: {
				type: 'string' | 'boolean' | 'array';
				values: string[];
				showTrue?: boolean;
				showFalse?: boolean;
			};
		};
		sortState: SortState;
		selectedGroupField: string | null;
		showFilters: boolean;
		viewMode: 'card' | 'list';
		currentPage: number;
		pageSize?: PageSizeOption;
	}

	// Load state from localStorage
	// Returns the restored pageSize if one was found, otherwise null
	function loadState(): PageSizeOption | null {
		if (!storageKey || typeof localStorage === 'undefined') return null;

		try {
			const saved = localStorage.getItem(storageKey);
			if (!saved) return null;

			const state: SerializableState = JSON.parse(saved);

			// Restore search
			searchQuery = state.searchQuery || '';

			// Restore filters
			if (state.filterState) {
				const restoredFilterState: FilterState = {};
				Object.keys(state.filterState).forEach((key) => {
					const saved = state.filterState[key];
					restoredFilterState[key] = {
						...saved,
						values: new SvelteSet(saved.values)
					};
				});
				filterState = restoredFilterState;
			}

			// Restore sort
			if (state.sortState) {
				sortState = state.sortState;
			}

			// Restore grouping
			if (state.selectedGroupField) {
				selectedGroupField = state.selectedGroupField;
			}

			// Restore filter panel state
			if (state.showFilters !== undefined) {
				showFilters = state.showFilters;
			}

			// Restore view mode
			if (state.viewMode) {
				viewMode = state.viewMode;
			}

			// Restore current page
			if (state.currentPage) {
				currentPage = state.currentPage;
			}

			// Restore page size
			if (state.pageSize && PAGE_SIZE_OPTIONS.includes(state.pageSize)) {
				pageSize = state.pageSize;
				return state.pageSize;
			}

			return null;
		} catch (e) {
			console.warn('Failed to load DataControls state from localStorage:', e);
			return null;
		}
	}

	// Save state to localStorage
	function saveState() {
		if (!storageKey || typeof localStorage === 'undefined') return;

		try {
			const serializableFilterState: SerializableState['filterState'] = {};
			Object.keys(filterState).forEach((key) => {
				const filter = filterState[key];
				serializableFilterState[key] = {
					...filter,
					values: Array.from(filter.values)
				};
			});

			const state: SerializableState = {
				searchQuery,
				filterState: serializableFilterState,
				sortState,
				selectedGroupField,
				showFilters,
				viewMode,
				currentPage,
				pageSize
			};

			localStorage.setItem(storageKey, JSON.stringify(state));
		} catch (e) {
			console.warn('Failed to save DataControls state to localStorage:', e);
		}
	}

	// Initialize filter state from fields
	$effect(() => {
		fields.forEach((field) => {
			const key = getFieldKey(field);
			if (field.filterable && !filterState[key]) {
				if (field.type === 'boolean') {
					filterState[key] = {
						type: 'boolean',
						values: new SvelteSet(),
						showTrue: true,
						showFalse: true
					};
				} else if (field.type === 'array') {
					filterState[key] = {
						type: 'array',
						values: new SvelteSet()
					};
				} else {
					filterState[key] = {
						type: 'string',
						values: new SvelteSet(field.filterDefaults)
					};
				}
			}
		});
	});

	// Load state on mount and set up auto-save
	onMount(() => {
		const restoredPageSize = loadState();

		// Notify parent of restored state for server-side pagination
		// This ensures the parent's query uses the restored pageSize
		if (restoredPageSize && onPageChange) {
			onPageChange(currentPage, restoredPageSize);
		}

		// Notify parent of restored ordering state
		if (onOrderChange && (selectedGroupField || sortState.field)) {
			onOrderChange(selectedGroupField, sortState.field, sortState.direction);
		}

		// Notify parent of restored tag filter state
		const tagFilter = filterState['tags'];
		if (onTagFilterChange && tagFilter && tagFilter.values.size > 0) {
			onTagFilterChange(Array.from(tagFilter.values));
		}

		// Notify parent of restored exclude filter state
		if (onExcludeFilterChange) {
			for (const field of fields) {
				if (field.filterable && field.filterMode === 'exclude') {
					const key = getFieldKey(field);
					const filter = filterState[key];
					if (filter && filter.values.size > 0) {
						onExcludeFilterChange(key, Array.from(filter.values));
					}
				}
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
					void sortState.field;
					void sortState.direction;
					void selectedGroupField;
					void showFilters;
					void viewMode;
					void currentPage;
					void pageSize;

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

	// Get value from item using field config
	function getFieldValue(
		item: T,
		field: FieldConfig<T>
	): string | boolean | Date | string[] | null {
		if (field.getValue) {
			return field.getValue(item);
		}
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		return (item as any)[getFieldKey(field)] ?? null;
	}

	// Get unique string values for a field (handles arrays by flattening)
	function getUniqueValues(field: FieldConfig<T>): string[] {
		const values = new SvelteSet<string>();
		items.forEach((item) => {
			const value = getFieldValue(item, field);
			if (value === null || value === undefined) return;

			if (field.type === 'array' && Array.isArray(value)) {
				value.forEach((v) => {
					if (v !== null && v !== undefined && v !== '') {
						values.add(String(v));
					}
				});
			} else if (value !== '') {
				values.add(String(value));
			}
		});
		return Array.from(values).sort();
	}

	// Get groupable fields (orderable string fields with groupable !== false, or display fields with groupable === true)
	let groupableFields = $derived(
		fields.filter(
			(f) =>
				(f.type === 'string' && isOrderableField(f) && f.groupable !== false) ||
				(isDisplayField(f) && f.groupable === true)
		)
	);

	// Get sortable fields (orderable fields, or display fields with sortable === true)
	let sortableFields = $derived(
		fields.filter((f) => isOrderableField(f) || (isDisplayField(f) && f.sortable === true))
	);

	// Apply all filters, sorting, and grouping
	let processedItems = $derived.by(() => {
		let result = items.filter((item) => {
			// Search filter
			if (searchQuery.trim()) {
				const q = searchQuery.toLowerCase();
				const searchableFields = fields.filter((f) => f.searchable !== false);
				const matchesQ = searchableFields.some((field) => {
					const value = getFieldValue(item, field);
					if (value === null || value === undefined) return false;

					// Handle array values in search
					if (field.type === 'array' && Array.isArray(value)) {
						return value.some((v) => String(v).toLowerCase().includes(q));
					}

					return String(value).toLowerCase().includes(q);
				});
				if (!matchesQ) return false;
			}

			// Field filters
			const matchesF = fields.every((field) => {
				if (!field.filterable) return true;

				const fieldKey = getFieldKey(field);
				const filterConfig = filterState[fieldKey];
				if (!filterConfig) return true;

				// Skip client-side tag filtering when server-side filtering is enabled
				// (the parent handles filtering via onTagFilterChange callback)
				if (fieldKey === 'tags' && onTagFilterChange) {
					return true;
				}

				// Skip client-side exclude filtering when server-side filtering is enabled
				if (field.filterMode === 'exclude' && onExcludeFilterChange) {
					return true;
				}

				const value = getFieldValue(item, field);

				if (field.type === 'boolean') {
					if (value === null || value === undefined) return true;
					const boolValue = Boolean(value);
					if (boolValue && !filterConfig.showTrue) return false;
					if (!boolValue && !filterConfig.showFalse) return false;
					return true;
				} else if (field.type === 'array') {
					// Array filter: item matches if ANY of its values are in the filter set
					if (filterConfig.values.size === 0) return true;
					if (!Array.isArray(value) || value.length === 0) return false;
					return value.some((v) => filterConfig.values.has(String(v)));
				} else if (field.type === 'string') {
					if (filterConfig.values.size === 0) return true;
					if (field.filterMode === 'exclude') {
						// Exclude mode: checked values are hidden
						return value == null || !filterConfig.values.has(String(value));
					}
					if (value === null || value === undefined) return false;
					return filterConfig.values.has(String(value));
				}

				return true;
			});

			return matchesF;
		});

		// Sort
		if (sortState.field) {
			const field = fields.find((f) => getFieldKey(f) === sortState.field);
			if (field) {
				result = [...result].sort((a, b) => {
					const aVal = getFieldValue(a, field);
					const bVal = getFieldValue(b, field);

					// Handle nulls
					if (aVal === null || aVal === undefined) return 1;
					if (bVal === null || bVal === undefined) return -1;

					let comparison: number;

					if (field.type === 'date') {
						const aDate = aVal instanceof Date ? aVal : new Date(String(aVal));
						const bDate = bVal instanceof Date ? bVal : new Date(String(bVal));
						comparison = aDate.getTime() - bDate.getTime();
					} else if (field.type === 'boolean') {
						comparison = (aVal ? 1 : 0) - (bVal ? 1 : 0);
					} else if (field.type === 'array') {
						// Sort arrays by length, then by first element
						const aArr = aVal as string[];
						const bArr = bVal as string[];
						comparison = aArr.length - bArr.length;
						if (comparison === 0 && aArr.length > 0 && bArr.length > 0) {
							comparison = aArr[0].localeCompare(bArr[0], undefined, {
								sensitivity: 'base',
								numeric: true
							});
						}
					} else {
						// String comparison
						comparison = String(aVal).localeCompare(String(bVal), undefined, {
							sensitivity: 'base',
							numeric: true
						});
					}

					return sortState.direction === 'asc' ? comparison : -comparison;
				});
			}
		}

		return result;
	});

	// Group items by selected field
	let groupedItems = $derived.by(() => {
		if (!selectedGroupField) {
			return new SvelteMap([[common_all(), processedItems]]);
		}

		const field = fields.find((f) => getFieldKey(f) === selectedGroupField);
		if (!field) {
			return new SvelteMap([[common_all(), processedItems]]);
		}

		const groups = new SvelteMap<string, T[]>();

		processedItems.forEach((item) => {
			const value = getFieldValue(item, field);
			const groupKey = value !== null && value !== undefined ? String(value) : common_ungrouped();

			if (!groups.has(groupKey)) {
				groups.set(groupKey, []);
			}
			groups.get(groupKey)!.push(item);
		});

		// Sort groups by key
		return new SvelteMap([...groups.entries()].sort((a, b) => a[0].localeCompare(b[0])));
	});

	// Toggle sort
	function toggleSort(fieldKey: string) {
		if (sortState.field === fieldKey) {
			sortState = {
				...sortState,
				direction: sortState.direction === 'asc' ? 'desc' : 'asc'
			};
		} else {
			sortState = {
				field: fieldKey,
				direction: 'asc'
			};
		}
	}

	// Toggle string/array filter value
	function toggleStringFilter(fieldKey: string, value: string) {
		const filter = filterState[fieldKey];
		if (!filter || (filter.type !== 'string' && filter.type !== 'array')) return;

		const newValues = new SvelteSet(filter.values);
		if (newValues.has(value)) {
			newValues.delete(value);
		} else {
			newValues.add(value);
		}

		filterState = {
			...filterState,
			[fieldKey]: {
				...filter,
				values: newValues
			}
		};

		// Notify parent of exclude filter changes
		if (onExcludeFilterChange) {
			const field = fields.find((f) => getFieldKey(f) === fieldKey);
			if (field?.filterMode === 'exclude') {
				onExcludeFilterChange(fieldKey, Array.from(newValues));
				// Reset pagination
				if (useServerPagination && onPageChange) {
					onPageChange(1, pageSize);
				} else {
					currentPage = 1;
				}
			}
		}
	}

	// Toggle boolean filter
	function toggleBooleanFilter(fieldKey: string, type: 'showTrue' | 'showFalse') {
		const filter = filterState[fieldKey];
		if (!filter || filter.type !== 'boolean') return;

		filterState = {
			...filterState,
			[fieldKey]: {
				...filter,
				[type]: !filter[type]
			}
		};
	}

	// Toggle tag filter (uses tag ID for server-side filtering)
	function toggleTagFilter(tagId: string) {
		const filter = filterState['tags'];
		if (!filter || filter.type !== 'array') return;

		const newValues = new SvelteSet(filter.values);
		if (newValues.has(tagId)) {
			newValues.delete(tagId);
		} else {
			newValues.add(tagId);
		}

		filterState = {
			...filterState,
			tags: {
				...filter,
				values: newValues
			}
		};
	}

	// Clear all filters (restores defaults for exclude filters)
	function clearFilters() {
		const newFilterState: FilterState = {};

		fields.forEach((field) => {
			if (field.filterable) {
				const key = getFieldKey(field);
				if (field.type === 'boolean') {
					newFilterState[key] = {
						type: 'boolean',
						values: new SvelteSet(),
						showTrue: true,
						showFalse: true
					};
				} else if (field.type === 'array') {
					newFilterState[key] = {
						type: 'array',
						values: new SvelteSet()
					};
				} else {
					newFilterState[key] = {
						type: 'string',
						values: new SvelteSet()
					};
				}
			}
		});

		filterState = newFilterState;

		// Notify parent that exclude filters were cleared
		if (onExcludeFilterChange) {
			fields.forEach((field) => {
				if (field.filterable && field.filterMode === 'exclude') {
					onExcludeFilterChange(getFieldKey(field), []);
				}
			});
		}
	}

	// Clear search
	function clearSearch() {
		searchQuery = '';
	}

	// Clear grouping
	function clearGrouping() {
		selectedGroupField = null;
	}

	// Select all visible items
	function selectAll() {
		processedItems.forEach((item) => {
			const itemId = getItemId(item);
			if (itemId) selectedIds.add(itemId);
		});
	}

	// Deselect all items
	function selectNone() {
		selectedIds.clear();
	}

	// Handle bulk delete
	async function handleBulkDelete() {
		if (!allowBulkDelete) return;
		if (!onBulkDelete || selectedIds.size === 0) return;

		try {
			await onBulkDelete(Array.from(selectedIds));
			selectedIds.clear();
		} catch (error) {
			console.error('Bulk delete failed:', error);
		}
	}

	// Handle bulk tag add
	async function handleBulkTagAdd(tagId: string) {
		if (!entityType || selectedIds.size === 0) return;

		try {
			await bulkAddTagMutation.mutateAsync({
				entity_ids: Array.from(selectedIds),
				entity_type: entityType,
				tag_id: tagId
			});
		} catch (error) {
			console.error('Bulk tag add failed:', error);
		}
	}

	// Handle bulk tag remove
	async function handleBulkTagRemove(tagId: string) {
		if (!entityType || selectedIds.size === 0) return;

		try {
			await bulkRemoveTagMutation.mutateAsync({
				entity_ids: Array.from(selectedIds),
				entity_type: entityType,
				tag_id: tagId
			});
		} catch (error) {
			console.error('Bulk tag remove failed:', error);
		}
	}

	// Compute common tags across selected items (intersection)
	let commonTags = $derived.by(() => {
		if (!getItemTags || selectedIds.size === 0) return [];

		const selectedItems = items.filter((item) => selectedIds.has(getItemId(item)));
		if (selectedItems.length === 0) return [];

		return computeCommonTags(selectedItems.map((item) => ({ tags: getItemTags!(item) })));
	});

	// Check if bulk tagging is enabled
	let hasBulkTagging = $derived(entityType !== null && getItemTags !== null);

	// Derived states
	let allSelected = $derived(
		processedItems.length > 0 && selectedIds.size === processedItems.length
	);

	// Check if any filters are active
	let hasActiveFilters = $derived(
		fields.some((field) => {
			if (!field.filterable) return false;
			const filter = filterState[getFieldKey(field)];
			if (!filter) return false;

			if (field.type === 'boolean') {
				return !filter.showTrue || !filter.showFalse;
			} else {
				return filter.values.size > 0;
			}
		})
	);

	let hasActiveSearch = $derived(searchQuery.trim().length > 0);
	let hasActiveGrouping = $derived(selectedGroupField !== null);

	// Check if using server-side pagination
	let useServerPagination = $derived(serverPagination !== null && onPageChange !== null);

	// Effective current page: derived from server offset when using server-side pagination
	let effectiveCurrentPage = $derived(
		useServerPagination && serverPagination
			? Math.floor(serverPagination.offset / pageSize) + 1
			: currentPage
	);

	// Pagination derived values (server-side or client-side)
	let totalPages = $derived(
		useServerPagination && serverPagination
			? Math.ceil(serverPagination.total_count / pageSize)
			: Math.ceil(processedItems.length / pageSize)
	);
	let canGoPrev = $derived(effectiveCurrentPage > 1);
	let canGoNext = $derived(
		useServerPagination && serverPagination
			? serverPagination.has_more
			: effectiveCurrentPage < totalPages
	);
	// When server pagination is active but client-side search filtering reduces items,
	// use the filtered count instead of the server's unfiltered total
	let hasClientSideSearch = $derived(useServerPagination && serverPagination && searchQuery.trim());
	let showingStart = $derived(
		useServerPagination && serverPagination
			? hasClientSideSearch
				? Math.min(1, processedItems.length)
				: Math.min(serverPagination.offset + 1, serverPagination.total_count)
			: Math.min((effectiveCurrentPage - 1) * pageSize + 1, processedItems.length)
	);
	let showingEnd = $derived(
		useServerPagination && serverPagination
			? hasClientSideSearch
				? processedItems.length
				: Math.min(serverPagination.offset + processedItems.length, serverPagination.total_count)
			: Math.min(effectiveCurrentPage * pageSize, processedItems.length)
	);
	let totalCount = $derived(
		useServerPagination && serverPagination
			? hasClientSideSearch
				? processedItems.length
				: serverPagination.total_count
			: processedItems.length
	);

	// Paginated items for display
	// Server-side: items are already paginated, just apply client-side filtering
	// Client-side: slice the processed items
	let paginatedItems = $derived(
		useServerPagination
			? processedItems
			: processedItems.slice((effectiveCurrentPage - 1) * pageSize, effectiveCurrentPage * pageSize)
	);

	// Reset to page 1 when filters/search change and current page would be out of bounds
	$effect(() => {
		if (effectiveCurrentPage > totalPages && totalPages > 0) {
			if (useServerPagination && onPageChange) {
				onPageChange(1, pageSize);
			} else {
				currentPage = 1;
			}
		}
	});

	// Track previous ordering state to detect changes and reset pagination
	let prevGroupBy: string | null = null;
	let prevOrderBy: string | null = null;
	let prevDirection: 'asc' | 'desc' = 'asc';
	let orderChangeInitialized = false;

	// Notify parent of ordering changes and reset pagination
	$effect(() => {
		const groupBy = selectedGroupField;
		const orderBy = sortState.field;
		const direction = sortState.direction;

		// Skip the initial run (state restoration)
		if (!orderChangeInitialized) {
			prevGroupBy = groupBy;
			prevOrderBy = orderBy;
			prevDirection = direction;
			orderChangeInitialized = true;
			return;
		}

		// Check if ordering actually changed
		const orderChanged =
			groupBy !== prevGroupBy || orderBy !== prevOrderBy || direction !== prevDirection;

		if (orderChanged) {
			prevGroupBy = groupBy;
			prevOrderBy = orderBy;
			prevDirection = direction;

			// Reset to page 1 when ordering changes
			if (useServerPagination && onPageChange) {
				onPageChange(1, pageSize);
			} else {
				currentPage = 1;
			}

			// Notify parent of the change
			if (onOrderChange) {
				onOrderChange(groupBy, orderBy, direction);
			}
		}
	});

	// Track previous tag filter state to detect changes
	let prevTagFilterValues: string[] = [];
	let tagFilterInitialized = false;

	// Notify parent of tag filter changes
	$effect(() => {
		const tagFilter = filterState['tags'];
		const currentTagIds = tagFilter ? Array.from(tagFilter.values).sort() : [];

		// Skip the initial run (state restoration)
		if (!tagFilterInitialized) {
			prevTagFilterValues = currentTagIds;
			tagFilterInitialized = true;
			return;
		}

		// Check if tag filter actually changed
		const tagFilterChanged =
			currentTagIds.length !== prevTagFilterValues.length ||
			currentTagIds.some((id, i) => id !== prevTagFilterValues[i]);

		if (tagFilterChanged) {
			prevTagFilterValues = currentTagIds;

			// Reset to page 1 when tag filter changes
			if (useServerPagination && onPageChange) {
				onPageChange(1, pageSize);
			} else {
				currentPage = 1;
			}

			// Notify parent of the change
			if (onTagFilterChange) {
				onTagFilterChange(currentTagIds);
			}
		}
	});

	// Pagination handlers
	function goToPrevPage() {
		if (canGoPrev) {
			if (useServerPagination && onPageChange) {
				onPageChange(effectiveCurrentPage - 1, pageSize);
			} else {
				currentPage = currentPage - 1;
			}
		}
	}

	function goToNextPage() {
		if (canGoNext) {
			if (useServerPagination && onPageChange) {
				onPageChange(effectiveCurrentPage + 1, pageSize);
			} else {
				currentPage = currentPage + 1;
			}
		}
	}

	// Page size change handler
	function handlePageSizeChange(newSize: PageSizeOption) {
		pageSize = newSize;
		// Reset to page 1 when page size changes
		if (useServerPagination && onPageChange) {
			onPageChange(1, newSize);
		} else {
			currentPage = 1;
		}
	}

	// Export button state and handler
	let isExporting = $state(false);

	async function handleExportClick() {
		// Use onExportClick override if provided, otherwise fall back to onCsvExport
		const handler = onExportClick ?? onCsvExport;
		if (!handler || isExporting) return;

		isExporting = true;
		try {
			await handler();
		} finally {
			isExporting = false;
		}
	}

	// Show export button if either handler is provided
	let hasExportHandler = $derived(onExportClick !== null || onCsvExport !== null);

	// Sticky detection
	let isStuck = $state(false);
	let sentinelRef: HTMLDivElement | null = $state(null);

	$effect(() => {
		const sentinel = sentinelRef;
		if (!sentinel) return;

		// Find the scroll container (the main element with overflow-auto)
		const scrollContainer = sentinel.closest('main');

		const observer = new IntersectionObserver(
			([entry]) => {
				// Only set stuck if actually scrolled down (prevents flash on tab switch)
				const scrollTop = scrollContainer?.scrollTop ?? 0;
				isStuck = !entry.isIntersecting && scrollTop > 0;
			},
			{ threshold: 0, root: scrollContainer }
		);
		observer.observe(sentinel);

		return () => observer.disconnect();
	});
</script>

<div class="space-y-4">
	<!-- Sentinel for sticky detection -->
	<div bind:this={sentinelRef} class="h-0 w-full"></div>

	<!-- Sticky Controls Bar -->
	<div
		class="sticky top-0 z-20 -mx-8 border-b bg-[var(--color-bg-body)] px-8 pb-4 {isStuck
			? 'border-gray-700 pt-4 shadow-lg'
			: 'border-transparent'}"
	>
		<div class="flex items-end justify-between">
			<!-- Left: Search + Filter/Group/Sort -->
			<div class="flex items-end gap-4">
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
							onclick={clearSearch}
							class="text-tertiary hover:text-secondary absolute right-3 top-1/2 -translate-y-1/2 transition-colors"
						>
							<X class="h-4 w-4" />
						</button>
					{/if}
				</div>

				<!-- Data Controls Group (Filter, Group, Sort) -->
				<div class="flex items-end gap-3">
					<!-- Filter Toggle -->
					{#if fields.some((f) => f.filterable)}
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
										onclick={clearGrouping}
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
										onclick={() => toggleSort(sortState.field || '')}
										class="btn-secondary h-[42px]"
										title={sortState.direction === 'asc' ? 'Ascending' : 'Descending'}
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
			</div>

			<!-- Right: View & Actions Group -->
			<div class="flex items-end gap-2">
				<!-- View Mode Toggle -->
				<button
					onclick={() => (viewMode = viewMode === 'card' ? 'list' : 'card')}
					class="btn-secondary h-[42px]"
					title={viewMode === 'card' ? common_switchToListView() : common_switchToCardView()}
				>
					{#if viewMode === 'card'}
						<List class="h-5 w-5" />
					{:else}
						<LayoutGrid class="h-5 w-5" />
					{/if}
				</button>

				<!-- Select All/None -->
				{#if onBulkDelete || hasBulkTagging}
					<button
						onclick={allSelected ? selectNone : selectAll}
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
				{#if hasExportHandler}
					<button
						onclick={handleExportClick}
						disabled={isExporting}
						class="btn-secondary h-[42px] disabled:cursor-not-allowed disabled:opacity-50"
						title={isExporting ? 'Exporting...' : 'Export'}
					>
						<Download class="h-5 w-5" />
					</button>
				{/if}
			</div>
		</div>

		<!-- Filter Panel (inside sticky wrapper) -->
		{#if showFilters}
			<div class="card mt-4 !rounded-lg !p-5">
				<div class="flex items-center justify-between">
					<h3 class="text-primary text-sm font-semibold">{common_filters()}</h3>
					{#if hasActiveFilters}
						<button
							onclick={clearFilters}
							class="text-tertiary hover:text-secondary text-xs transition-colors"
						>
							{common_clearAll()}
						</button>
					{/if}
				</div>

				<div class="mt-4 grid grid-cols-1 gap-x-8 gap-y-5 md:grid-cols-2 lg:grid-cols-3">
					{#each fields.filter((f) => f.filterable) as field (getFieldKey(field))}
						{@const fieldKey = getFieldKey(field)}
						<div class="space-y-2">
							<div class="text-secondary text-sm font-medium">{field.label}</div>

							{#if field.type === 'boolean'}
								{@const filter = filterState[fieldKey]}
								<div class="space-y-1.5">
									<label class="flex cursor-pointer items-center gap-2">
										<input
											type="checkbox"
											checked={filter?.showTrue}
											onchange={() => toggleBooleanFilter(fieldKey, 'showTrue')}
											class="checkbox-card h-4 w-4 rounded"
										/>
										<span class="text-secondary text-sm">{common_showTrue()}</span>
									</label>
									<label class="flex cursor-pointer items-center gap-2">
										<input
											type="checkbox"
											checked={filter?.showFalse}
											onchange={() => toggleBooleanFilter(fieldKey, 'showFalse')}
											class="checkbox-card h-4 w-4 rounded"
										/>
										<span class="text-secondary text-sm">{common_showFalse()}</span>
									</label>
								</div>
							{:else if fieldKey === 'tags'}
								<!-- Special tag filter with colored tags (stores tag IDs for server-side filtering) -->
								{@const filter = filterState[fieldKey]}
								<div
									use:scrollFade
									class="flex max-h-32 flex-wrap gap-1.5 overflow-y-scroll rounded-md bg-black/5 p-2 dark:bg-white/5"
								>
									{#if allTags.length === 0}
										<p class="text-tertiary text-xs">{common_noTagsAvailable()}</p>
									{:else}
										{#each allTags as tag (tag.id)}
											{@const isSelected = filter?.values.has(tag.id)}
											<button
												onclick={() => toggleTagFilter(tag.id)}
												class="transition-opacity {isSelected
													? 'opacity-100'
													: 'opacity-50 hover:opacity-75'}"
											>
												<Tag label={tag.name} color={tag.color as Color} />
											</button>
										{/each}
									{/if}
								</div>
							{:else}
								{@const uniqueValues = field.filterOptions ?? getUniqueValues(field)}
								{@const filter = filterState[fieldKey]}
								<div
									use:scrollFade
									class="max-h-32 space-y-1.5 overflow-y-scroll rounded-md bg-black/5 p-2 dark:bg-white/5"
								>
									{#if uniqueValues.length === 0}
										<p class="text-tertiary text-xs">{common_noValuesAvailable()}</p>
									{:else}
										{#each uniqueValues as value (value)}
											<label class="flex cursor-pointer items-center gap-2">
												<input
													type="checkbox"
													checked={filter?.values.has(value)}
													onchange={() => toggleStringFilter(fieldKey, value)}
													class="checkbox-card h-4 w-4 rounded"
												/>
												<span class="text-secondary truncate text-sm" title={value}>{value}</span>
											</label>
										{/each}
									{/if}
								</div>
							{/if}
						</div>
					{/each}
				</div>
			</div>
		{/if}
	</div>

	<!-- Bulk Action Bar (shown when items are selected) -->
	{#if (onBulkDelete || hasBulkTagging) && selectedIds.size > 0}
		<div class="card space-y-3 p-4">
			<div class="flex items-center justify-between">
				<div class="flex items-center gap-4">
					<span class="text-primary text-sm font-medium">
						{common_itemsSelected({
							count: selectedIds.size,
							itemLabel: selectedIds.size === 1 ? common_item() : common_items()
						})}
					</span>
					<button
						onclick={selectNone}
						class="text-tertiary hover:text-secondary text-sm transition-colors"
					>
						{common_clearSelection()}
					</button>
				</div>
				{#if allowBulkDelete && onBulkDelete}
					<button onclick={handleBulkDelete} class="btn-danger flex items-center gap-2">
						<Trash2 class="h-4 w-4" />
						{common_deleteSelected()}
					</button>
				{/if}
			</div>

			<!-- Bulk Tagging -->
			{#if hasBulkTagging}
				<div class="flex items-center gap-3 border-t border-gray-700 pt-3">
					<span class="text-secondary text-sm">{common_tags()}:</span>
					<TagPickerInline
						selectedTagIds={commonTags}
						onAdd={handleBulkTagAdd}
						onRemove={handleBulkTagRemove}
					/>
					{#if commonTags.length === 0 && selectedIds.size > 1}
						<span class="text-tertiary text-xs">{common_noCommonTags()}</span>
					{/if}
				</div>
			{/if}
		</div>
	{/if}

	<!-- Results Count and Pagination -->
	<div class="text-tertiary flex items-center justify-between text-sm">
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
					count: processedItems.length,
					total: items.length,
					itemLabel: items.length === 1 ? common_item() : common_items()
				})}
			{/if}
		</span>
		<div class="flex items-center gap-4">
			{#if hasActiveGrouping}
				<span>
					{groupedItems.size}
					{groupedItems.size === 1 ? common_group() : common_groups()}
				</span>
			{/if}
			<!-- Page size selector (only show when there are more than 20 items) -->
			{#if totalCount > 20}
				<div class="flex items-center gap-2">
					<span class="text-tertiary whitespace-nowrap text-sm">{common_show()}</span>
					<select
						value={pageSize}
						onchange={(e) =>
							handlePageSizeChange(parseInt(e.currentTarget.value) as PageSizeOption)}
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
						onclick={goToPrevPage}
						disabled={!canGoPrev}
						class="btn-secondary p-1 disabled:cursor-not-allowed disabled:opacity-50"
						title={common_previousPage()}
					>
						<ChevronLeft class="h-5.5 w-5.5" />
					</button>
					<span class="text-secondary min-w-[80px] text-center">
						{common_pageOf({ current: effectiveCurrentPage, total: totalPages })}
					</span>
					<button
						onclick={goToNextPage}
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

	<!-- Content -->
	{#if hasActiveGrouping}
		<!-- Grouped view -->
		<div class="space-y-6">
			{#each [...groupedItems.entries()] as [groupName, groupItems] (groupName)}
				<div class="space-y-3">
					<!-- Group Header -->
					<div class="flex items-center gap-3">
						<h3 class="text-primary text-lg font-semibold">{groupName}</h3>
						<span class="text-tertiary text-sm">({groupItems.length})</span>
					</div>

					<!-- Group Items -->
					<div
						class={viewMode === 'list'
							? 'space-y-2'
							: 'grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3'}
					>
						{#each groupItems as item (getItemId(item))}
							<!-- eslint-disable-next-line @typescript-eslint/no-explicit-any -->
							{@const itemId = getItemId(item)}
							{@const isSelected = selectedIds.has(itemId)}
							{@render children(item, viewMode, isSelected, (selected) => {
								if (selected) {
									selectedIds.add(itemId);
								} else {
									selectedIds.delete(itemId);
								}
							})}
						{/each}
					</div>
				</div>
			{/each}
		</div>
	{:else}
		<!-- Ungrouped view (paginated) -->
		<div
			class={viewMode === 'list'
				? 'space-y-2'
				: 'grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3'}
		>
			{#each paginatedItems as item (getItemId(item))}
				{@const itemId = getItemId(item)}
				{@const isSelected = selectedIds.has(itemId)}
				{@render children(item, viewMode, isSelected, (selected) => {
					if (selected) {
						selectedIds.add(itemId);
					} else {
						selectedIds.delete(itemId);
					}
				})}
			{/each}
		</div>
	{/if}
</div>
