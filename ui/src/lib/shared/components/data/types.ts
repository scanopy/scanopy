import type { IconComponent } from '$lib/shared/utils/types';
import type { Snippet } from 'svelte';
import type { Color } from '$lib/shared/utils/styling';
import type { EntityDiscriminants } from '$lib/api/entities';

// ============================================================================
// Page Size Configuration
// ============================================================================

export const PAGE_SIZE_OPTIONS = [20, 50, 100] as const;
export type PageSizeOption = (typeof PAGE_SIZE_OPTIONS)[number];

/**
 * How many chips a compact cell shows before collapsing the rest behind "+N".
 *
 * A cap is load-bearing for accessibility, not just layout: `EntityTag` is
 * focusable, so an uncapped tag column would put hundreds of tab stops between
 * a table's first row and its last.
 */
export const MAX_ITEMS_IN_CELL = 3;

export interface TagProps {
	/**
	 * Omit to render the tag as its icon alone — the text element collapses rather than rendering
	 * empty. A tag with neither label nor icon has nothing to show and reads as unknown.
	 */
	label?: string;
	textColor?: string;
	bgColor?: string;
	color?: Color;
	icon?: IconComponent;
	href?: string;
	entityRef?: EntityRef;
	pill?: boolean;
	title?: string;
	onmouseenter?: () => void;
	onmouseleave?: () => void;
	onclick?: () => void;
}

export interface CardAction {
	label: string;
	icon: IconComponent; // Svelte component
	class?: string;
	onClick: () => void;
	disabled?: boolean;
	tooltip?: string | ((disabled: boolean) => string | null);
	animation?: string;
	forceLabel?: boolean;
}

export interface EntityRef {
	entityType: EntityDiscriminants;
	entityId: string;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	data: any;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	context?: any;
}

/** Shorthand to build an EntityRef — avoids casting at every call site. */
export function entityRef(
	entityType: EntityDiscriminants,
	entityId: string,
	data: object,
	context?: object
): EntityRef {
	return { entityType, entityId, data, context };
}

/**
 * A `CardFieldItem` whose label is guaranteed.
 *
 * The shape to return when the label is not only display text but the field's searchable,
 * groupable value — several columns derive `getValue` from `getItems(...).map((i) => i.label)`,
 * and that is only sound if every item has one.
 */
export type LabelledCardFieldItem = CardFieldItem & { label: string };

export interface CardFieldItem {
	id: string;
	/** Omit to render the item as its icon alone — see `TagProps.label`. */
	label?: string;
	icon?: IconComponent; // Svelte component instead of HTML
	iconColor?: string;
	bgColor?: string;
	color?: Color;
	disabled?: boolean;
	metadata?: Record<string, unknown>;
	badge?: string; // For things like "5m", "Critical", etc.
	badgeColor?: string;
	title?: string;
	entityRef?: EntityRef;
}

export interface CardField {
	label: string;
	value?: string | CardFieldItem[] | undefined | null;
	snippet?: Snippet; // Allow snippet as an alternative to value
	color?: Color; // Used for tags when value is an array
	emptyText?: string; // Used when value is empty array
}

// ============================================================================
// Field Configuration for Data Controls
// ============================================================================

/**
 * How a field renders, in the card and the table alike.
 *
 * Omit it and the value renders as the stringified `getValue` — the same value
 * search, filtering and grouping already match against, so what is shown can
 * never disagree with the filter that produced the row.
 */
export interface DisplayConfig<T> {
	/** Filter-only field that never becomes a column (e.g. a port number filter). */
	hidden?: boolean;
	/** A real column, but unchecked in the column menu until the user asks for it. */
	hiddenByDefault?: boolean;
	/**
	 * Rich chips, in the vocabulary the card already renders: `EntityTag` when
	 * an item carries an `entityRef`, `Tag` otherwise. Prefer this over `cell` —
	 * it is data rather than markup, so the card can reuse the same builder.
	 *
	 * Returning `undefined` — as opposed to `[]` — means "no chips for this row",
	 * and the cell falls back to the field's plain value. That is what lets a
	 * date column carry a Stale tag on the rows that are stale and the date
	 * everywhere else, rather than needing a second, mostly-empty column.
	 */
	getItems?: (item: T) => CardFieldItem[] | undefined;
	/** Escape hatch for genuinely bespoke content: a status tag, a link, an icon. */
	cell?: Snippet<[T]>;
	align?: 'left' | 'right';
	/**
	 * Where this field sits among the columns, low to high.
	 *
	 * Needed because `defineFields` groups server-orderable fields ahead of
	 * display-only ones, which is the right shape for exhaustiveness checking but
	 * has nothing to do with the order a reader wants to scan. Fields without one
	 * keep their declared order, after those that have one.
	 */
	order?: number;
	/** Starting width in px. A user's resize persists over this. */
	width?: number;
	/** Row identity: pinned left, carries the checkbox, renders as `<th scope="row">`. */
	primary?: boolean;
	/**
	 * This field is the row's secondary line, so the card renders it under the
	 * title rather than as another labelled row — a subnet's CIDR, a VLAN's
	 * number, what a host is virtualized by.
	 */
	subtitle?: boolean;
	/**
	 * This field sits after the tag column, immediately before the row actions —
	 * the far end of the row. For content that reads as the row's live state
	 * rather than one of its attributes, like a running scan's progress.
	 */
	trailing?: boolean;
	/**
	 * This field is the row's status, so the card renders it as the tag beside
	 * the title instead of as another labelled row.
	 *
	 * Marking it rather than letting the card compute its own is what stops the
	 * two views disagreeing: a card that derived its own status tag showed
	 * "Healthy" where the table's separate computation said "Active".
	 */
	statusTag?: boolean;
}

/**
 * Base configuration shared by all field types.
 */
interface BaseFieldConfig<T> {
	/** How this field renders, in both the card and the table. Omit for plain text. */
	display?: DisplayConfig<T>;
	type: 'string' | 'boolean' | 'date' | 'array';
	label: string;
	/**
	 * Whether the search box matches against this field. Opt-in: leave it off for
	 * date and boolean fields, whose stringified values match far too broadly
	 * (a date field turns "2026" into a hit on every row).
	 *
	 * Ignored by lists that search server-side — those match against the
	 * entity's `Storable::search_predicates` instead.
	 */
	searchable?: boolean;
	filterable?: boolean;
	getValue?: (item: T) => string | boolean | Date | string[] | null;
	/** 'include' (default): checked values shown. 'exclude': checked values hidden. */
	filterMode?: 'include' | 'exclude';
	/**
	 * The parent applies this filter server-side via `onFilterChange`, so the
	 * client-side pass leaves it alone. Required for any filter on a
	 * server-paginated list: filtering the loaded rows would only ever narrow the
	 * page in hand, silently hiding matches on every other page.
	 */
	serverFiltered?: boolean;
	/** External filter options (bypasses getUniqueValues). Use when values aren't derivable from current items (e.g., server-side pagination). */
	filterOptions?: string[];
	/**
	 * The raw value the server groups this item under, when it differs from
	 * what `getValue` renders (e.g. `network_id` is a UUID in the database but
	 * a network name in the UI). Only needed on groupable fields of
	 * server-paginated lists: it's the key that matches a group to its total in
	 * the response's `group_counts`. Defaults to the displayed value.
	 */
	getGroupValue?: (item: T) => string | null;
	/** Default checked values for the filter (applied on first load if no localStorage state). */
	filterDefaults?: string[];
}

/**
 * Field with server-side ordering support.
 * The orderField property IS the backend OrderField value (e.g., 'name', 'created_at').
 * Presence of this property implies the field is sortable/groupable.
 */
export interface OrderableFieldConfig<T, O extends string> extends BaseFieldConfig<T> {
	orderField: O;
	/** Whether this field can be used for grouping. Defaults to true for string types. */
	groupable?: boolean;
}

/**
 * Display-only field (no server-side ordering).
 * Used for fields that are shown in the UI but can't be sorted/grouped on the backend.
 * Can opt-in to client-side sorting/grouping via `sortable` and `groupable` flags.
 */
export interface DisplayFieldConfig<T> extends BaseFieldConfig<T> {
	key: string;
	/** Whether this field can be used for client-side sorting. */
	sortable?: boolean;
	/** Whether this field can be used for client-side grouping. */
	groupable?: boolean;
}

/**
 * Union type for field configuration.
 * - Fields with `orderField` are sortable/groupable via backend
 * - Fields with `key` are display-only
 */
export type FieldConfig<T, O extends string = string> =
	OrderableFieldConfig<T, O> | DisplayFieldConfig<T>;

/**
 * Type guard to check if a field supports server-side ordering.
 */
export function isOrderableField<T, O extends string>(
	field: FieldConfig<T, O>
): field is OrderableFieldConfig<T, O> {
	return 'orderField' in field;
}

/**
 * Type guard to check if a field is a display-only field.
 */
export function isDisplayField<T, O extends string>(
	field: FieldConfig<T, O>
): field is DisplayFieldConfig<T> {
	return 'key' in field;
}

/**
 * Get the field identifier (orderField for orderable fields, key for display fields).
 * Used for client-side operations like search and localStorage keys.
 */
export function getFieldKey<T, O extends string>(field: FieldConfig<T, O>): string {
	return isOrderableField(field) ? field.orderField : field.key;
}

// ============================================================================
// Grouped Pagination
// ============================================================================

/** Where a group sits in the full ordered result set. */
export interface GroupPosition {
	/** Index of the group's first row across all pages. */
	start: number;
	/** How many rows the group holds in total. */
	count: number;
}

/** The portion of a group visible on the current page. */
export interface GroupSlice {
	total: number;
	/** 1-based index within the group of its first row on this page. */
	start: number;
	/** 1-based index within the group of its last row on this page. */
	end: number;
}

/**
 * Which slice of a group the current page is showing.
 *
 * A page can straddle group boundaries, so this intersects the page's span with
 * the group's rather than assuming a group starts where the page does.
 *
 * Returns `null` when the group is shown in full — there is no "1–5 of 5" worth
 * saying, and the caller falls back to a plain count.
 */
export function groupPageSlice(
	group: GroupPosition,
	pageOffset: number,
	pageLength: number
): GroupSlice | null {
	const pageEnd = pageOffset + pageLength;
	const start = Math.max(pageOffset, group.start) - group.start + 1;
	const end = Math.min(pageEnd, group.start + group.count) - group.start;

	if (start === 1 && end === group.count) return null;
	return { total: group.count, start, end };
}

// ============================================================================
// Type-Safe Field Definition Helper
// ============================================================================

/**
 * Configuration for a single orderable field entry.
 * The orderField value is derived from the map key, so it's not included here.
 */
type OrderableFieldEntry<T> = BaseFieldConfig<T> & {
	groupable?: boolean;
};

/**
 * A map that REQUIRES an entry for every value in the OrderField union O.
 * TypeScript will error if any OrderField value is missing.
 */
type OrderableFieldsMap<T, O extends string> = {
	[K in O]: OrderableFieldEntry<T>;
};

/**
 * Creates a type-safe field configuration array.
 *
 * @param orderableFields - MUST include an entry for every backend OrderField value.
 *   TypeScript will error if any OrderField value is missing or if an invalid key is used.
 * @param displayFields - Optional display-only fields (no backend ordering support).
 *   These are for UI-only fields like computed values or nested properties.
 *
 * @example
 * ```typescript
 * type SubnetOrderField = components['schemas']['SubnetOrderField'];
 *
 * const fields = defineFields<Subnet, SubnetOrderField>(
 *   {
 *     name: { label: 'Name', type: 'string', searchable: true },
 *     created_at: { label: 'Created', type: 'date' },
 *     // ... all other SubnetOrderField values required
 *   },
 *   [
 *     { key: 'description', label: 'Description', type: 'string' }
 *   ]
 * );
 * ```
 */
export function defineFields<T, O extends string>(
	orderableFields: OrderableFieldsMap<T, O>,
	displayFields?: DisplayFieldConfig<T>[]
): FieldConfig<T, O>[] {
	const orderable = (Object.entries(orderableFields) as [O, OrderableFieldEntry<T>][]).map(
		([orderField, config]) => ({
			...config,
			orderField
		})
	) as OrderableFieldConfig<T, O>[];

	return [...orderable, ...(displayFields ?? [])];
}
