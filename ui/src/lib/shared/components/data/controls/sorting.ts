import {
	getFieldKey,
	isDisplayField,
	isOrderableField,
	type FieldConfig,
	type TableDefaults
} from '../types';
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
	if (field.compare) {
		const comparison = field.compare(a, b);
		return direction === 'asc' ? comparison : -comparison;
	}

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
 * MAC's leading `14` as a number, and knows nothing of the server's SQL). A
 * display field opted in with `sortable` still sorts here when the list holds
 * every row, since the server never saw it.
 *
 * With `serverPaginated`, nothing sorts here. The client holds one page, so
 * sorting it would reorder that page alone and present it as the order of the
 * whole list.
 */
export function sortItems<T>(
	items: T[],
	fields: FieldConfig<T>[],
	sort: SortState,
	serverOrdered = false,
	serverPaginated = false
): T[] {
	if (!sort.field || serverPaginated) return items;

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

/**
 * Whether the user can sort by this field: server-orderable, or a display field
 * opted in with `sortable` on a list that holds every row.
 *
 * The sort control and the table headers both read this, so a header can never
 * offer a sort the control doesn't, and vice versa.
 */
export function isSortableField<T>(field: FieldConfig<T>, serverPaginated: boolean): boolean {
	if (isOrderableField(field)) return true;
	return !serverPaginated && field.sortable === true;
}

/**
 * Whether the user can group by this field.
 *
 * String and boolean orderable fields group by default, unless `groupable:
 * false`. A display field must opt in with `groupable: true`, and only on a list
 * that holds every row. An array field never groups: its value has no single
 * bucket, and stringifying it would bucket by the joined list.
 */
export function isGroupableField<T>(field: FieldConfig<T>, serverPaginated: boolean): boolean {
	if (field.type === 'array') return false;
	if (isOrderableField(field)) {
		return (field.type === 'string' || field.type === 'boolean') && field.groupable !== false;
	}
	return !serverPaginated && field.groupable === true;
}

/** Fields offered in the sort control. */
export function sortableFields<T>(
	fields: FieldConfig<T>[],
	serverPaginated = false
): FieldConfig<T>[] {
	return fields.filter((f) => isSortableField(f, serverPaginated));
}

/** Fields offered in the group-by control. */
export function groupableFields<T>(
	fields: FieldConfig<T>[],
	serverPaginated = false
): FieldConfig<T>[] {
	return fields.filter((f) => isGroupableField(f, serverPaginated));
}

/**
 * Display fields a server-paginated list would wrongly sort or group client-side.
 *
 * Under server pagination the client holds one page, so a client-side sort
 * reorders that page alone and a client-side group buckets it alone, while the
 * pager walks the server's order. Only an orderable field, which the server
 * sorts and groups across every page, is coherent there.
 *
 * Returns the offending field keys so a caller can name them; empty when the
 * list is not server-paginated, where client-side sorting and grouping are
 * correct.
 */
export function serverOrderViolations<T>(
	fields: FieldConfig<T>[],
	serverPaginated: boolean
): string[] {
	if (!serverPaginated) return [];

	return fields
		.filter(
			(field) => isDisplayField(field) && (field.sortable === true || field.groupable === true)
		)
		.map(getFieldKey);
}

/**
 * Default group and sort keys a list cannot apply.
 *
 * A list groups and sorts only by rendered columns, so a default on a missing
 * field, a field that cannot group or sort, or a column hidden until the user
 * shows it would silently open the tab ungrouped or unsorted.
 */
export function defaultOrderingViolations<T>(
	fields: FieldConfig<T>[],
	defaults: TableDefaults,
	serverPaginated: boolean
): string[] {
	const rendersByDefault = (field: FieldConfig<T>) =>
		!field.display?.hidden && !field.display?.hiddenByDefault;
	const fieldFor = (key: string) => fields.find((f) => getFieldKey(f) === key);

	const offenders: string[] = [];
	if (defaults.group) {
		const field = fieldFor(defaults.group);
		if (!field || !rendersByDefault(field) || !isGroupableField(field, serverPaginated)) {
			offenders.push(`group "${defaults.group}"`);
		}
	}
	if (defaults.sort) {
		const field = fieldFor(defaults.sort.field);
		if (!field || !rendersByDefault(field) || !isSortableField(field, serverPaginated)) {
			offenders.push(`sort "${defaults.sort.field}"`);
		}
	}
	return offenders;
}
