import { SvelteSet } from 'svelte/reactivity';
import { PAGE_SIZE_OPTIONS, type PageSizeOption, type TableDefaults } from '../types';
import type { FilterState } from './filtering';
import type { SortState } from './sorting';

/**
 * The stored-state format this build writes.
 *
 * Builds before 2 wrote a null sort and a null group on a tab's first mount, so
 * in an older blob a null is the absence of a choice, not a choice of "none".
 */
export const STORED_STATE_VERSION = 2;

/** Filter selections as they sit in localStorage — sets marshalled to arrays. */
export interface StoredFieldFilter {
	type: 'string' | 'boolean' | 'array';
	values: string[];
	showTrue?: boolean;
	showFalse?: boolean;
}

/**
 * The sort and grouping the user picked. A missing entry means they never
 * picked one, so the tab's default applies and a later change to that default
 * still reaches them. `group: null` means they chose no grouping.
 */
export interface OrderingChoice {
	sort?: SortState;
	group?: string | null;
}

export interface StoredState {
	version?: number;
	searchQuery: string;
	filterState: Record<string, StoredFieldFilter>;
	sortState?: SortState;
	selectedGroupField?: string | null;
	currentPage: number;
	pageSize?: PageSizeOption;
	columnVisibility?: Record<string, boolean>;
	columnOrder?: string[];
	columnSizing?: Record<string, number>;
}

/**
 * The sort and grouping a tab applies: the user's choice, else the tab's default.
 *
 * A choice naming a field the tab no longer has counts as no choice, so a stale
 * key from an older build lands on the default rather than on nothing.
 */
export function resolveOrdering(
	choice: OrderingChoice,
	defaults: TableDefaults,
	fieldKeys: ReadonlySet<string>
): { sortState: SortState; groupField: string | null } {
	const sortKnown =
		choice.sort !== undefined && (choice.sort.field === null || fieldKeys.has(choice.sort.field));
	const groupKnown =
		choice.group !== undefined && (choice.group === null || fieldKeys.has(choice.group));

	return {
		sortState: sortKnown ? choice.sort! : (defaults.sort ?? { field: null, direction: 'asc' }),
		groupField: groupKnown ? choice.group! : (defaults.group ?? null)
	};
}

function isPageSize(raw: unknown): raw is PageSizeOption {
	return PAGE_SIZE_OPTIONS.includes(raw as PageSizeOption);
}

function parseSortState(raw: unknown, current: boolean): SortState | undefined {
	if (!raw || typeof raw !== 'object') return undefined;
	const value = raw as Partial<SortState>;
	const field = typeof value.field === 'string' ? value.field : null;
	if (field === null && !current) return undefined;
	return { field, direction: value.direction === 'desc' ? 'desc' : 'asc' };
}

function parseGroupField(raw: unknown, current: boolean): string | null | undefined {
	if (typeof raw === 'string') return raw;
	return raw === null && current ? null : undefined;
}

function parseFilterState(raw: unknown): Record<string, StoredFieldFilter> {
	if (!raw || typeof raw !== 'object') return {};

	const out: Record<string, StoredFieldFilter> = {};
	for (const [key, value] of Object.entries(raw as Record<string, unknown>)) {
		const filter = value as Partial<StoredFieldFilter> | undefined;
		if (!filter || typeof filter !== 'object') continue;
		out[key] = {
			type: filter.type === 'boolean' || filter.type === 'array' ? filter.type : 'string',
			values: Array.isArray(filter.values) ? filter.values.map(String) : [],
			showTrue: filter.showTrue,
			showFalse: filter.showFalse
		};
	}
	return out;
}

/**
 * Parse a stored blob into a fully-typed state.
 *
 * Every field is treated as optional so a blob written by an older build still
 * loads: a corrupt or partial key must never blank a tab. Returns `null` when
 * there is nothing usable, which the caller reads as "use defaults".
 */
export function parseStoredState(raw: string | null): StoredState | null {
	if (!raw) return null;

	let parsed: unknown;
	try {
		parsed = JSON.parse(raw);
	} catch {
		return null;
	}

	if (!parsed || typeof parsed !== 'object') return null;
	const state = parsed as Record<string, unknown>;
	const current = state.version === STORED_STATE_VERSION;

	// A retired `viewMode` (card view) or `showFilters` key is simply not read:
	// every tab renders the table.
	return {
		version: STORED_STATE_VERSION,
		searchQuery: typeof state.searchQuery === 'string' ? state.searchQuery : '',
		filterState: parseFilterState(state.filterState),
		sortState: parseSortState(state.sortState, current),
		selectedGroupField: parseGroupField(state.selectedGroupField, current),
		currentPage:
			typeof state.currentPage === 'number' && state.currentPage > 0 ? state.currentPage : 1,
		pageSize: isPageSize(state.pageSize) ? state.pageSize : undefined,
		columnVisibility: isRecordOf(state.columnVisibility, 'boolean')
			? (state.columnVisibility as Record<string, boolean>)
			: undefined,
		columnOrder: Array.isArray(state.columnOrder) ? state.columnOrder.map(String) : undefined,
		columnSizing: isRecordOf(state.columnSizing, 'number')
			? (state.columnSizing as Record<string, number>)
			: undefined
	};
}

function isRecordOf(raw: unknown, type: 'boolean' | 'number'): boolean {
	if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return false;
	return Object.values(raw as Record<string, unknown>).every((v) => typeof v === type);
}

export function serializeState(state: StoredState): string {
	return JSON.stringify(state);
}

/**
 * Rehydrate saved filter selections into the reactive sets the panel mutates.
 *
 * Stored as plain arrays because a Set has no JSON form; they have to come back
 * as `SvelteSet` or a restored filter renders but never reacts.
 */
export function reviveFilterState(stored: StoredState['filterState']): FilterState {
	const revived: FilterState = {};

	for (const [key, saved] of Object.entries(stored)) {
		revived[key] = { ...saved, values: new SvelteSet(saved.values) };
	}

	return revived;
}

/** The JSON-safe form of the live filter state. */
export function toStoredFilterState(live: FilterState): StoredState['filterState'] {
	const stored: StoredState['filterState'] = {};

	for (const [key, filter] of Object.entries(live)) {
		stored[key] = { ...filter, values: Array.from(filter.values) };
	}

	return stored;
}
