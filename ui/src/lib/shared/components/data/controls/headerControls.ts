import { getFieldKey, type FieldConfig } from '../types';
import { blankFilter, isFieldFilterActive, type FilterState } from './filtering';
import { isGroupableField, type SortState } from './sorting';

/**
 * What a column's header control offers.
 *
 * Derived from the field's own flags, never from per-tab header config, so a
 * field marked filterable or groupable gets its control wherever its column
 * renders.
 */
export interface ColumnControl<T> {
	field: FieldConfig<T>;
	filter: boolean;
	group: boolean;
	/** The column carries the list's "Stale only" toggle. */
	stale: boolean;
}

/**
 * The header control for a column, or null when it offers nothing.
 *
 * Looks the field up in `fields` by column id rather than reading the column's
 * own field, because the tags column the list appends carries a synthetic
 * field: the tab's `tags` field is the one that holds the filter.
 *
 * `staleAvailable` is whether the parent applies staleness at all; without it a
 * `staleFilter` mark is inert.
 */
export function columnControls<T>(
	columnId: string,
	fields: FieldConfig<T>[],
	serverPaginated: boolean,
	staleAvailable: boolean
): ColumnControl<T> | null {
	const field = fields.find((f) => getFieldKey(f) === columnId);
	if (!field) return null;

	const control = {
		field,
		filter: field.filterable === true,
		group: isGroupableField(field, serverPaginated),
		stale: staleAvailable && field.staleFilter === true
	};
	return control.filter || control.group || control.stale ? control : null;
}

/** What a header icon shows as active. */
export function columnControlState<T>(
	control: ColumnControl<T>,
	filterState: FilterState,
	staleOnly: boolean,
	activeGroupField: string | null
): { filtered: boolean; grouped: boolean } {
	const key = getFieldKey(control.field);
	return {
		filtered:
			(control.filter && isFieldFilterActive(control.field, filterState[key])) ||
			(control.stale && staleOnly),
		grouped: control.group && activeGroupField === key
	};
}

/** Fields whose column is rendered — the only ones a list filters, sorts or groups by. */
export function renderedFields<T>(
	fields: FieldConfig<T>[],
	renderedColumnIds: ReadonlySet<string>
): FieldConfig<T>[] {
	return fields.filter((f) => renderedColumnIds.has(getFieldKey(f)));
}

/** The narrowing and ordering state a hidden column can hold. */
export interface ColumnBoundState {
	filterState: FilterState;
	staleOnly: boolean;
	sortState: SortState;
	groupField: string | null;
}

/**
 * Clear whatever a hidden column was filtering, sorting or grouping by.
 *
 * A list only filters, sorts and groups by columns it renders, so hiding one
 * drops its state rather than leaving a constraint nobody can see. Returns null
 * when nothing changes. `serverKeys` names the cleared filters the parent
 * applies, so the caller can tell it; `staleCleared` likewise.
 */
export function dropHiddenColumnState<T>(
	state: ColumnBoundState,
	fields: FieldConfig<T>[],
	renderedColumnIds: ReadonlySet<string>
): (ColumnBoundState & { serverKeys: string[]; staleCleared: boolean }) | null {
	const hidden = fields.filter((f) => !renderedColumnIds.has(getFieldKey(f)));
	const hiddenKeys = new Set(hidden.map(getFieldKey));

	const filterState = { ...state.filterState };
	const serverKeys: string[] = [];
	let changed = false;

	for (const field of hidden) {
		const key = getFieldKey(field);
		if (!isFieldFilterActive(field, filterState[key])) continue;
		filterState[key] = blankFilter(field, false);
		if (field.serverFiltered) serverKeys.push(key);
		changed = true;
	}

	const staleCleared = state.staleOnly && hidden.some((f) => f.staleFilter === true);
	const sortCleared = state.sortState.field !== null && hiddenKeys.has(state.sortState.field);
	const groupCleared = state.groupField !== null && hiddenKeys.has(state.groupField);

	if (!changed && !staleCleared && !sortCleared && !groupCleared) return null;

	return {
		filterState,
		staleOnly: staleCleared ? false : state.staleOnly,
		sortState: sortCleared ? { field: null, direction: 'asc' } : state.sortState,
		groupField: groupCleared ? null : state.groupField,
		serverKeys,
		staleCleared
	};
}
