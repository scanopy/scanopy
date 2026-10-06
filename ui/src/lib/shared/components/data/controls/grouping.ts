import {
	getFieldKey,
	type FieldConfig,
	type GroupPosition,
	type GroupSlice,
	type TreeConfig
} from '../types';
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
		const value = field.getGroupLabel ? field.getGroupLabel(item) : getFieldValue(item, field);
		const groupKey = groupLabel(value, field, labels);

		if (!groups.has(groupKey)) {
			groups.set(groupKey, []);
		}
		groups.get(groupKey)!.push(item);
	});

	if (preserveOrder) return groups;

	return new Map([...groups.entries()].sort((a, b) => a[0].localeCompare(b[0])));
}

/**
 * A row of a tree group. `depth` is how many sections (counting the group itself when its top row
 * heads it) the row sits inside: it is indented one step per level and draws one guide line per
 * level, so each section's line runs from just under its header down to its last row.
 */
export interface TreeRow<T> {
	type: 'row';
	item: T;
	depth: number;
}

/**
 * A row that has children, drawn as a collapsible section: a header, then the row itself, then
 * its children. `key` is the path from the group down to this row, so two rows with the same
 * label never share collapse state.
 */
export interface TreeSection<T> {
	type: 'section';
	key: string;
	label: string;
	/** Rows in the loaded subtree, this row included. */
	count: number;
	/**
	 * Sections this header sits inside. Its chevron sits at this level, directly above the guide
	 * line its own rows draw one level in.
	 */
	depth: number;
	entries: TreeEntry<T>[];
}

export type TreeEntry<T> = TreeRow<T> | TreeSection<T>;

/** A group as the card and table views draw it. */
export interface RenderGroup<T> {
	/** Collapse-state key, unique per group. */
	key: string;
	/** The header. */
	name: string;
	/** Every row of the group, in the order drawn. */
	items: T[];
	range: GroupSlice | null;
	/** The group's nested sections when the grouping is a tree, else null. */
	entries: TreeEntry<T>[] | null;
}

interface TreeNode<T> {
	item: T;
	children: TreeNode<T>[];
}

/** Separates the keys of a section path; NUL cannot occur in a group label or an id. */
const PATH_SEPARATOR = String.fromCharCode(0);

/**
 * One group's rows as the nested sections the tree `tree` describes.
 *
 * Every row with children becomes a section headed by `tree.label`, holding the row itself and
 * then its children, all one level in, so the section's line starts at its own row. A group whose
 * rows all descend from one top-level row (no parent at all) is already headed by that row, so
 * the group header serves as its section header: the row and its children start one level in.
 *
 * - With `serverPaginated` the rows keep the order they arrived in, which the server made
 *   parent-first across every page, and nesting comes from `tree.depth`. A row whose ancestors are
 *   on an earlier page starts at the top of the group.
 * - Otherwise every row is in hand, so nesting comes from `tree.parentKey`: a row whose parent is
 *   not in the group (filtered out, say) starts at the top. Siblings are ordered by `compare`
 *   when given, else kept in arrival order. A cycle is cut where it would repeat.
 *
 * `groupKey` prefixes every section key, so the same row under two groupings keeps two states.
 */
export function buildTreeSections<T>(
	items: T[],
	tree: TreeConfig<T>,
	serverPaginated: boolean,
	groupKey: string,
	compare?: (a: T, b: T) => number
): TreeEntry<T>[] {
	const forest = serverPaginated
		? forestFromDepths(items, tree)
		: forestFromParents(items, tree, compare);

	// Only a real root heads its group: a row whose parent is filtered out, or on another page,
	// still gets a section of its own.
	if (
		forest.length === 1 &&
		forest[0].children.length > 0 &&
		tree.parentKey(forest[0].item) === null
	) {
		const [root] = forest;
		const path = groupKey + PATH_SEPARATOR + tree.key(root.item);
		return [
			{ type: 'row', item: root.item, depth: 1 },
			...root.children.map((child) => toEntry(child, 1, path, tree))
		];
	}
	return forest.map((node) => toEntry(node, 0, groupKey, tree));
}

/** Every row of a group's entries, in the order they are drawn. */
export function flattenTreeEntries<T>(entries: TreeEntry<T>[]): T[] {
	return entries.flatMap((entry) =>
		entry.type === 'row' ? [entry.item] : flattenTreeEntries(entry.entries)
	);
}

function toEntry<T>(
	node: TreeNode<T>,
	depth: number,
	parentPath: string,
	tree: TreeConfig<T>
): TreeEntry<T> {
	if (node.children.length === 0) return { type: 'row', item: node.item, depth };

	const key = parentPath + PATH_SEPARATOR + tree.key(node.item);
	return {
		type: 'section',
		key,
		label: tree.label(node.item),
		count: subtreeSize(node),
		depth,
		entries: [
			{ type: 'row', item: node.item, depth: depth + 1 },
			...node.children.map((child) => toEntry(child, depth + 1, key, tree))
		]
	};
}

function subtreeSize<T>(node: TreeNode<T>): number {
	return 1 + node.children.reduce((sum, child) => sum + subtreeSize(child), 0);
}

function forestFromParents<T>(
	items: T[],
	tree: TreeConfig<T>,
	compare?: (a: T, b: T) => number
): TreeNode<T>[] {
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

	const seen = new Set<string>();
	const build = (parent: string | null): TreeNode<T>[] => {
		const nodes: TreeNode<T>[] = [];
		for (const item of children.get(parent) ?? []) {
			const key = tree.key(item);
			// A key seen twice would be a cycle; stop rather than recurse forever.
			if (seen.has(key)) continue;
			seen.add(key);
			nodes.push({ item, children: build(key) });
		}
		return nodes;
	};
	return build(null);
}

function forestFromDepths<T>(items: T[], tree: TreeConfig<T>): TreeNode<T>[] {
	const roots: TreeNode<T>[] = [];
	// Open ancestors, deepest last. A row nests under the nearest one shallower than itself.
	const stack: { depth: number; node: TreeNode<T> }[] = [];
	for (const item of items) {
		const depth = tree.depth?.(item) ?? 0;
		while (stack.length > 0 && stack[stack.length - 1].depth >= depth) stack.pop();
		const node: TreeNode<T> = { item, children: [] };
		if (stack.length > 0) stack[stack.length - 1].node.children.push(node);
		else roots.push(node);
		stack.push({ depth, node });
	}
	return roots;
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
