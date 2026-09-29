import { getFieldKey, isDisplayField, isOrderableField, type FieldConfig } from '../types';
import { getFieldValue } from './fieldValues';

export type SortDirection = 'asc' | 'desc';

export interface SortState {
	field: string | null;
	direction: SortDirection;
}

/**
 * Compare two items by one field, already signed for `direction`.
 *
 * Null and undefined always sort last, in *both* directions: the null branches
 * return before the direction is applied, so they are deliberately not negated
 * for a descending sort. A row with no value is missing data, not an extreme —
 * surfacing a page of blanks at the top of a descending sort would bury the
 * rows the user asked to see.
 */
export function compareByField<T>(
	a: T,
	b: T,
	field: FieldConfig<T>,
	direction: SortDirection
): number {
	const aVal = getFieldValue(a, field);
	const bVal = getFieldValue(b, field);

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
		// Arrays sort by length first, then by their first element.
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
		// `numeric` keeps host9 ahead of host10 rather than ordering by codepoint.
		comparison = String(aVal).localeCompare(String(bVal), undefined, {
			sensitivity: 'base',
			numeric: true
		});
	}

	return direction === 'asc' ? comparison : -comparison;
}

/**
 * Sort a copy of `items` by the field matching `sort.field`.
 *
 * With `serverOrdered`, an orderable field is left in arrival order: the server
 * already sorted every page by it, and this comparator can disagree (it reads a
 * MAC's leading `14` as a number, and knows nothing of the server's SQL). Display
 * fields opted in with `sortable` still sort here, since the server never saw them.
 */
export function sortItems<T>(
	items: T[],
	fields: FieldConfig<T>[],
	sort: SortState,
	serverOrdered = false
): T[] {
	if (!sort.field) return items;

	const field = fields.find((f) => getFieldKey(f) === sort.field);
	if (!field) return items;
	if (serverOrdered && isOrderableField(field)) return items;

	return [...items].sort((a, b) => compareByField(a, b, field, sort.direction));
}

/**
 * Where a click on `fieldKey` moves the sort.
 *
 * Clicking the active field flips direction; clicking any other field starts it
 * ascending. The result always names exactly one field, which is what lets the
 * table set a non-`none` `aria-sort` on exactly one header.
 */
export function nextSortState(current: SortState, fieldKey: string): SortState {
	if (current.field === fieldKey) {
		return { field: fieldKey, direction: current.direction === 'asc' ? 'desc' : 'asc' };
	}
	return { field: fieldKey, direction: 'asc' };
}

/** Fields offered in the sort control: server-orderable, or opted in client-side. */
export function sortableFields<T>(fields: FieldConfig<T>[]): FieldConfig<T>[] {
	return fields.filter((f) => isOrderableField(f) || (isDisplayField(f) && f.sortable === true));
}

/** Fields offered in the group-by control. String orderable fields group by default. */
export function groupableFields<T>(fields: FieldConfig<T>[]): FieldConfig<T>[] {
	return fields.filter(
		(f) =>
			(f.type === 'string' && isOrderableField(f) && f.groupable !== false) ||
			(isDisplayField(f) && f.groupable === true)
	);
}
