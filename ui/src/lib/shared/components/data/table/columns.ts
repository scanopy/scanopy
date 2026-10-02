import {
	getFieldKey,
	isDisplayField,
	isOrderableField,
	type DisplayConfig,
	type FieldConfig
} from '../types';

/**
 * A table column, derived from the field that already describes this data.
 *
 * There is deliberately no second list of columns: `FieldConfig` already
 * carries the label, the type and the value accessor, and `defineFields` forces
 * exhaustive coverage of the backend order-field union — so every column the
 * server can sort is guaranteed to exist here.
 */
export interface EntityColumn<T> {
	/**
	 * Always `getFieldKey(field)`.
	 *
	 * This is the load-bearing invariant of the whole table: it is what makes a
	 * header click dispatch a sort the backend actually accepts, because the same
	 * key is the field's `orderField` and therefore a valid `*OrderField` value.
	 */
	id: string;
	label: string;
	field: FieldConfig<T>;
	display: DisplayConfig<T>;
	/** Whether a header click can sort this column. */
	sortable: boolean;
	align: 'left' | 'right';
	width?: number;
	primary: boolean;
}

/**
 * Id of the tags column the list appends itself.
 *
 * Reserved so a field declaring the same key is treated as filter-only rather
 * than rendering a second, non-editable tags column beside the real one.
 */
export const TAG_COLUMN_ID = 'tags';

/** Column state the table owns but `DataControls` persists. */
export interface ColumnState {
	visibility: Record<string, boolean>;
	order: string[];
}

/**
 * Turn field configs into columns.
 *
 * Fields marked `display.hidden` produce nothing — they exist only to drive a
 * filter (a port number, say) and have no value worth a column of its own.
 */
export function fieldsToColumns<T>(fields: FieldConfig<T>[]): EntityColumn<T>[] {
	return (
		fields
			// A `tags` field still drives search and the filter panel, but the list
			// renders tags itself as an editable column pinned after everything else.
			.filter((field) => !field.display?.hidden && getFieldKey(field) !== TAG_COLUMN_ID)
			.map((field) => {
				const display = field.display ?? {};
				return {
					id: getFieldKey(field),
					label: field.label,
					field,
					display,
					// Mirrors the sort dropdown's rule, so a header can never offer a sort
					// the dropdown doesn't and vice versa.
					sortable: isOrderableField(field) || (isDisplayField(field) && field.sortable === true),
					align: display.align ?? 'left',
					width: display.width,
					primary: display.primary === true
				};
			})
	);
}

/** Default visibility: everything except fields that opted out of first paint. */
export function defaultColumnVisibility<T>(columns: EntityColumn<T>[]): Record<string, boolean> {
	const visibility: Record<string, boolean> = {};
	for (const column of columns) {
		visibility[column.id] = column.display.hiddenByDefault !== true;
	}
	return visibility;
}

/**
 * Default order: an explicit `display.order` first, then declaration order.
 *
 * Declaration order alone cannot express what a reader wants, because
 * `defineFields` groups every server-orderable field ahead of the display-only
 * ones — so a status field that belongs second ends up wherever its
 * sortability put it.
 */
export function defaultColumnOrder<T>(columns: EntityColumn<T>[]): string[] {
	return columns
		.map((column, index) => ({ column, index }))
		.sort((a, b) => {
			const orderA = a.column.display.order ?? Number.POSITIVE_INFINITY;
			const orderB = b.column.display.order ?? Number.POSITIVE_INFINITY;
			return orderA === orderB ? a.index - b.index : orderA - orderB;
		})
		.map(({ column }) => column.id);
}

/**
 * Whether the user can move this column.
 *
 * The pinning rule, enforced here and in `reconcileColumnState`: the primary
 * column always leads, because it carries the row's identity and checkbox;
 * `trailing` columns always end the row, because they read as its live state;
 * and the tags column, which is never in the order at all, sits between the
 * two (`withTagColumn`). Everything else is the user's to arrange.
 */
export function isMovableColumn<T>(column: EntityColumn<T>): boolean {
	return !column.primary && column.display.trailing !== true;
}

/** Primary columns first, trailing ones last, the rest in the order given. */
function pinColumns<T>(order: string[], columns: EntityColumn<T>[]): string[] {
	const byId = new Map(columns.map((c) => [c.id, c]));
	const rank = (id: string) => {
		const column = byId.get(id);
		if (column?.primary) return 0;
		if (column?.display.trailing) return 2;
		return 1;
	};
	// A stable sort, so each group keeps the order it arrived in.
	return [...order].sort((a, b) => rank(a) - rank(b));
}

/**
 * Fold persisted column state onto the columns that exist now.
 *
 * Renaming or removing a field must not leave a stale entry deciding anything,
 * a stored order cannot unpin a pinned column, and a newly added field lands
 * beside the column it is declared after rather than at the end.
 */
export function reconcileColumnState<T>(
	columns: EntityColumn<T>[],
	stored: Partial<ColumnState> | undefined
): ColumnState {
	const defaults = defaultColumnVisibility(columns);
	const visibility: Record<string, boolean> = {};

	for (const column of columns) {
		const persisted = stored?.visibility?.[column.id];
		visibility[column.id] = typeof persisted === 'boolean' ? persisted : defaults[column.id];
	}

	const defaultOrder = defaultColumnOrder(columns);
	const known = new Set(defaultOrder);
	const order = (stored?.order ?? []).filter((id) => known.has(id));
	const placed = new Set(order);

	// A column the stored order never knew about goes straight after its nearest
	// default-order predecessor, wherever the user has since moved that one. Its
	// default index would mean nothing in an order the user rearranged.
	defaultOrder.forEach((id, index) => {
		if (placed.has(id)) return;
		const anchor = defaultOrder
			.slice(0, index)
			.reverse()
			.find((candidate) => placed.has(candidate));
		order.splice(anchor === undefined ? 0 : order.indexOf(anchor) + 1, 0, id);
		placed.add(id);
	});

	return { visibility, order: pinColumns(order, columns) };
}

/**
 * Move a column one step among the movable columns (`delta` of -1 or 1).
 *
 * Returns the order unchanged for a pinned column or a step past either end.
 */
export function moveColumn<T>(
	order: string[],
	columns: EntityColumn<T>[],
	id: string,
	delta: -1 | 1
): string[] {
	const movable = movableIds(order, columns);
	const index = movable.indexOf(id);
	const target = movable[index + delta];
	if (index === -1 || target === undefined) return order;

	return moveColumnTo(order, columns, id, target, delta < 0 ? 'before' : 'after');
}

/**
 * Drop a column before or after another one.
 *
 * Returns the order unchanged when either column is pinned, so a drag can
 * never move the primary column or put something on the far side of it.
 */
export function moveColumnTo<T>(
	order: string[],
	columns: EntityColumn<T>[],
	id: string,
	targetId: string,
	position: 'before' | 'after'
): string[] {
	const movable = new Set(movableIds(order, columns));
	if (id === targetId || !movable.has(id) || !movable.has(targetId)) return order;

	const without = order.filter((candidate) => candidate !== id);
	const targetIndex = without.indexOf(targetId);
	without.splice(position === 'before' ? targetIndex : targetIndex + 1, 0, id);
	return without;
}

/** The ids in `order` the user can move, in that order. */
export function movableIds<T>(order: string[], columns: EntityColumn<T>[]): string[] {
	const byId = new Map(columns.map((c) => [c.id, c]));
	return order.filter((id) => {
		const column = byId.get(id);
		return column !== undefined && isMovableColumn(column);
	});
}

/** Columns to render, in persisted order, minus the hidden ones. */
export function visibleColumns<T>(
	columns: EntityColumn<T>[],
	state: ColumnState
): EntityColumn<T>[] {
	const byId = new Map(columns.map((c) => [c.id, c]));

	return state.order
		.map((id) => byId.get(id))
		.filter((c): c is EntityColumn<T> => Boolean(c) && state.visibility[c!.id] !== false);
}

/**
 * The tag column the list appends to every taggable entity.
 *
 * Built here rather than declared per tab so every taggable entity gets the
 * same column in the same place — a tab cannot silently end up without one.
 * `cell` is passed in because the snippet that renders it belongs to the
 * component that owns the tag data.
 */
export function buildTagColumn<T>(
	label: string,
	getTags: (item: T) => string[],
	cell: EntityColumn<T>['display']['cell']
): EntityColumn<T> {
	return {
		id: TAG_COLUMN_ID,
		label,
		field: { key: TAG_COLUMN_ID, label, type: 'array', getValue: (item: T) => getTags(item) },
		display: { cell },
		sortable: false,
		align: 'left',
		primary: false
	};
}

/**
 * Row order: ordinary columns, then tags, then anything marked `trailing`.
 *
 * Tags sit at the far end rather than in the user's column order, so they are
 * appended instead of ordered — except for trailing fields (actions), which
 * stay beyond them.
 */
export function withTagColumn<T>(
	visible: EntityColumn<T>[],
	tagColumn: EntityColumn<T> | null
): EntityColumn<T>[] {
	return [
		...visible.filter((column) => !column.display.trailing),
		...(tagColumn ? [tagColumn] : []),
		...visible.filter((column) => column.display.trailing)
	];
}
