import { getFieldKey, type FieldConfig, type GroupPosition, type TreeConfig } from '../types';
import { getFieldValue, type FieldValue } from './fieldValues';

/**
 * Stands in for a null group key, which has no string form of its own.
 *
 * Prefixed with NUL because Postgres text values cannot contain one, so this
 * can never collide with a real group value. Built with `fromCharCode` rather
 * than a literal to keep a raw NUL byte out of the source file.
 */
export const UNGROUPED_KEY = `${String.fromCharCode(0)}ungrouped`;

/** A per-group total from the server, across every page. */
export interface ServerGroupCount {
	value?: string | null;
	count: number;
}

/** The translated headers for groups whose raw value has no readable form. */
export interface GroupLabels {
	/** Rows with no value for the grouped field. */
	ungrouped: string;
	/** A boolean field's `true` group. */
	yes: string;
	/** A boolean field's `false` group. */
	no: string;
}

/**
 * Bucket items by the grouped field, keyed by the header each group shows.
 *
 * A boolean field's groups read as the `yes`/`no` labels rather than the
 * stringified `true`/`false`. The header is only a display key: matching a group
 * to its server total goes through `serverGroupKey`, which reads the raw value.
 *
 * When the server supplied group totals the rows already arrive in the server's
 * group order, so the buckets are left in insertion order — re-sorting here
 * would desync the headers from the cumulative offsets those totals index by.
 */
export function groupItems<T>(
	items: T[],
	fields: FieldConfig<T>[],
	groupFieldKey: string | null,
	labels: GroupLabels,
	preserveOrder: boolean
): Map<string, T[]> {
	if (!groupFieldKey) return new Map();

	const field = fields.find((f) => getFieldKey(f) === groupFieldKey);
	if (!field) return new Map();

	const groups = new Map<string, T[]>();

	items.forEach((item) => {
		const value = getFieldValue(item, field);
		const groupKey = groupLabel(value, field, labels);

		if (!groups.has(groupKey)) {
			groups.set(groupKey, []);
		}
		groups.get(groupKey)!.push(item);
	});

	if (preserveOrder) return groups;

	return new Map([...groups.entries()].sort((a, b) => a[0].localeCompare(b[0])));
}

/** A group's rows in tree order, and each row's depth keyed by its `tree.key`. */
export interface TreeLayout<T> {
	items: T[];
	depths: Map<string, number>;
}

/**
 * One group's rows arranged as the tree `tree` describes.
 *
 * With `serverPaginated` the rows are left in the order they arrived, which the server already
 * made parent-first across every page, and depth comes from `tree.depth`.
 *
 * Otherwise every row is in hand, so both are derived: roots are the rows with no parent in this
 * group (a parent filtered out of the list makes its children roots), each followed by its
 * descendants. Siblings keep the order they arrived in unless `compare` is given.
 */
export function arrangeTree<T>(
	items: T[],
	tree: TreeConfig<T>,
	serverPaginated: boolean,
	compare?: (a: T, b: T) => number
): TreeLayout<T> {
	if (serverPaginated) {
		return {
			items,
			depths: new Map(items.map((item) => [tree.key(item), tree.depth?.(item) ?? 0]))
		};
	}

	const present = new Set(items.map((item) => tree.key(item)));
	const children = new Map<string | null, T[]>();
	for (const item of items) {
		const parent = tree.parentKey(item);
		const slot = parent !== null && present.has(parent) ? parent : null;
		if (!children.has(slot)) children.set(slot, []);
		children.get(slot)!.push(item);
	}
	if (compare) {
		for (const siblings of children.values()) siblings.sort(compare);
	}

	const ordered: T[] = [];
	const depths = new Map<string, number>();
	const visit = (parent: string | null, depth: number) => {
		for (const item of children.get(parent) ?? []) {
			const key = tree.key(item);
			// A key seen twice would be a cycle; stop rather than recurse forever.
			if (depths.has(key)) continue;
			depths.set(key, depth);
			ordered.push(item);
			visit(key, depth + 1);
		}
	};
	visit(null, 0);

	return { items: ordered, depths };
}

/**
 * Tree fields a server-paginated list cannot draw: without `depth` the client would have to
 * derive depth from the loaded page, which misses any parent on another page.
 */
export function treeDepthViolations<T>(
	fields: FieldConfig<T>[],
	serverPaginated: boolean
): string[] {
	if (!serverPaginated) return [];
	return fields.filter((field) => field.tree && !field.tree.depth).map(getFieldKey);
}

function groupLabel<T>(value: FieldValue, field: FieldConfig<T>, labels: GroupLabels): string {
	if (value === null || value === undefined) return labels.ungrouped;
	if (field.type === 'boolean') return value ? labels.yes : labels.no;
	return String(value);
}

/**
 * Where each group starts in the full ordered result set.
 *
 * The server returns groups in the same order it orders rows, so a running sum
 * of the counts gives every group's global offset — which is what turns "this
 * page holds rows 100-199" into "this is rows 1-40 of that group".
 */
export function computeGroupOffsets(counts: ServerGroupCount[] | null): Map<string, GroupPosition> {
	const offsets = new Map<string, GroupPosition>();
	let cursor = 0;

	for (const group of counts ?? []) {
		offsets.set(group.value ?? UNGROUPED_KEY, { start: cursor, count: group.count });
		cursor += group.count;
	}

	return offsets;
}

/**
 * The value the server grouped these rows under, which is not always what the
 * header displays — a site group reads as a name but groups by id.
 */
export function serverGroupKey<T>(
	groupItems: T[],
	fields: FieldConfig<T>[],
	groupFieldKey: string | null
): string {
	const field = fields.find((f) => getFieldKey(f) === groupFieldKey);
	if (!field || groupItems.length === 0) return UNGROUPED_KEY;

	const raw = field.getGroupValue
		? field.getGroupValue(groupItems[0])
		: getFieldValue(groupItems[0], field);

	return raw === null || raw === undefined ? UNGROUPED_KEY : String(raw);
}
