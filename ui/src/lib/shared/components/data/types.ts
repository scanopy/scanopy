import type { IconComponent } from '$lib/shared/utils/types';
import type { Snippet } from 'svelte';
import type { Color } from '$lib/shared/utils/styling';
import type { EntityDiscriminants } from '$lib/api/entities';
import type { AttributeSource } from '$lib/shared/utils/attribute-source';
import type { TooltipContent } from '$lib/shared/actions/tooltip';
import type { components } from '$lib/api/schema';

/** The heading a column is listed under in the column menu; see `field-groups.json`. */
export type FieldGroup = components['schemas']['FieldGroup'];

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
	/** A paid feature's badge (`upgradeTagProps`): an amber chip around its crown, instead of `color`. */
	upgrade?: boolean;
	/** Hover text, or structured tooltip content. */
	title?: string | TooltipContent;
	/**
	 * What the tag says about the row (a display's own vocabulary, e.g. `subnet`, `guest`). A
	 * display context's `hideTags`, or `compact` through the display's `compactHides`, drops tags
	 * by role where they repeat what the surrounding view already shows.
	 */
	role?: string;
	onmouseenter?: () => void;
	onmouseleave?: () => void;
	onclick?: () => void;
}

/** An action's colour. Omitted, it renders neutral. The row and the toolbar both render from it. */
export type ActionTone = 'danger' | 'warning' | 'success' | 'info';

export interface RowAction {
	label: string;
	icon: IconComponent; // Svelte component
	tone?: ActionTone;
	/** This action deletes the row: it takes the Delete key, and running it clears the selection. */
	isDelete?: boolean;
	/** This action opens the row's modal: double-clicking the row runs it. */
	isOpen?: boolean;
	onClick: () => void;
	/**
	 * Runs the action on every selected row, given their ids. With it, the action runs in the
	 * toolbar for a selection of any size through this; without it, the action runs only while its
	 * one row is selected, and shows disabled for several.
	 */
	onBulk?: (ids: string[]) => Promise<unknown> | void;
	/** Why the action can't run on this row, and whether the row still shows it. Absent when it can. */
	unavailable?: Unavailable;
	/**
	 * The toolbar key this action prefers, when free. One control in two states (Enable and
	 * Disable) names the same key, so the key stays put as the label changes.
	 */
	shortcut?: string;
	/** A hint shown while the action is available. */
	tooltip?: string;
	animation?: string;
}

export interface Unavailable {
	/**
	 * `disabled`: the row shows the action greyed out, for an action most rows of this type
	 * support. `hidden`: the row leaves it out, for an action that exists only in a special state
	 * (Resolve Range on a provisional subnet). A selection of several rows still shows a hidden
	 * action, disabled with this reason, when another selected row shows it.
	 */
	state: 'disabled' | 'hidden';
	/** Reads for this one row: "This host has no IP addresses to scan". */
	reason: string;
	/**
	 * Reads for a selection of several rows, where `reason` would not: "A selected host has no IP
	 * addresses to scan". Falls back to `reason`.
	 */
	bulkReason?: string;
}

/** `RowAction.unavailable` for an action most rows support: shown, greyed out, while `when`. */
export function disabledWhen(
	when: boolean,
	reason: string,
	bulkReason?: string
): Unavailable | undefined {
	return when ? { state: 'disabled', reason, bulkReason } : undefined;
}

/** `RowAction.unavailable` for an action of a special state: left off the row while `when`. */
export function hiddenWhen(
	when: boolean,
	reason: string,
	bulkReason?: string
): Unavailable | undefined {
	return when ? { state: 'hidden', reason, bulkReason } : undefined;
}

export interface EntityRef {
	entityType: EntityDiscriminants;
	entityId: string;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	data: any;
}

/** Shorthand to build an EntityRef — avoids casting at every call site. */
export function entityRef(
	entityType: EntityDiscriminants,
	entityId: string,
	data: object
): EntityRef {
	return { entityType, entityId, data };
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
	/** Hover text, or structured tooltip content. */
	title?: string | TooltipContent;
	/** Opens in a new tab: a docs link that explains the value. */
	href?: string;
	entityRef?: EntityRef;
}

// ============================================================================
// Field Configuration for Data Controls
// ============================================================================

/**
 * How a field renders in the table.
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
	 * The heading the column is listed under in the column menu. Registry columns take their
	 * field's group; a tab sets it on its own columns. A column moves only among columns of its
	 * own group, and a group moves as a block.
	 */
	group?: FieldGroup;
	/**
	 * The heading for a column that is no entity field, such as a tag group's: listed after every
	 * field group by default. `id` keeps the user's heading order across languages. Ignored when
	 * `group` is set.
	 */
	section?: { id: string; label: string };
	/**
	 * Rich chips: `EntityTag` when an item carries an `entityRef`, `Tag`
	 * otherwise. Prefer this over `cell`: it is data rather than markup, so
	 * `getValue` can derive from the same builder.
	 *
	 * Returning `undefined` — as opposed to `[]` — means "no chips for this row",
	 * and the cell falls back to the field's plain value. That is what lets a
	 * date column carry a Stale tag on the rows that are stale and the date
	 * everywhere else, rather than needing a second, mostly-empty column.
	 */
	getItems?: (item: T) => CardFieldItem[] | undefined;
	/**
	 * Escape hatch for genuinely bespoke content: a status tag, a link, an icon. Called with the
	 * row and the column's id, so one snippet can serve several columns.
	 */
	cell?: Snippet<[T, string]>;
	/**
	 * Where a plain value came from, for a field that records its provenance (a host's
	 * `hostname_source`). The value's tooltip explains the source under the value, the way
	 * the OS tag does.
	 */
	getSource?: (item: T) => AttributeSource | null | undefined;
	/** Plain text wraps within the column instead of truncating: free text such as a description. */
	wrap?: boolean;
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
	 * This field sits after the tag column, immediately before the row actions —
	 * the far end of the row. For content that reads as the row's live state
	 * rather than one of its attributes, like a running scan's progress.
	 */
	trailing?: boolean;
	/**
	 * This date is when something last happened (last seen, last used), so it renders as
	 * recent activity: `3h ago`, or the full timestamp when the user turned relative times off.
	 * Other date fields render as a compact date.
	 */
	recency?: boolean;
}

/**
 * How the rows of a group nest, for a groupable field whose groups are trees: subnets inside the
 * range that contains them, guests under the host that runs them.
 *
 * When the list is grouped by that field, every row with children gets a chevron that collapses
 * its children, which sit one level in beneath it, as deep as the tree goes. See
 * `buildTreeSections`.
 *
 * A parent can sit on another page, so nesting can't be derived from the loaded rows: the server
 * returns each group's rows parent-first, with each row's `depth`.
 */
export interface TreeConfig<T> {
	/** The row's identity, as `parentKey` refers to it. */
	key: (item: T) => string;
	/** The identity of the row this one nests under, or `null` for a row nested in nothing. */
	parentKey: (item: T) => string | null;
	/** The row's depth as the server computed it. */
	depth: (item: T) => number;
	/**
	 * The identity of the row at the top of this row's tree (the row itself when it heads one), or
	 * `null` for a row in no tree. Tells a tree whose root is on another page from a lone row.
	 */
	rootKey: (item: T) => string | null;
	/**
	 * The header of the one group that collects every tree under a real root ("Virtualized",
	 * "Nested"), set apart from the rows outside any tree. Each root row inside it carries its own
	 * chevron and count.
	 */
	rootsLabel: () => string;
}

/**
 * How a filter's selected values become query params and back, for a filter the API takes as
 * more than one param. `decode` returns undefined when the params carry no selection.
 */
export interface FilterCodec {
	keys: readonly string[];
	encode: (values: string[]) => [string, string][];
	decode: (params: URLSearchParams) => string[] | undefined;
}

/**
 * Base configuration shared by all field types.
 */
export interface BaseFieldConfig<T, Q extends string = string> {
	/** How this field renders in the table. Omit for plain text. */
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
	 * The server applies this filter, through the query `onQueryChange` hands the parent. Required
	 * for any filter: the list holds one page, so a filter the server ignores would narrow nothing.
	 */
	serverFiltered?: boolean;
	/**
	 * The value a filter matches an item on: an id, an enum id or a stored string, the same value
	 * the API filters on. Defaults to `getValue`, which suits a field that already shows the raw
	 * value. A field showing a name (a site's) or a translated label returns the id here and renders
	 * it through `filterLabel`, so a filter, its URL and its API request all hold the same value.
	 */
	getFilterValue?: (item: T) => string | boolean | string[] | null;
	/** The text a filter option shows for a value. Defaults to the value itself. */
	filterLabel?: (value: string) => string;
	/**
	 * The query param this filter is sent and linked as: the endpoint's own name where it has one
	 * (`site_ids`), so a list's URL is the request it makes. Defaults to the field key, and to
	 * `tag_ids` for the `tags` field.
	 */
	queryParam?: Q;
	/**
	 * For a filter the API takes as more than one param, such as "Virtualized By": service names in
	 * one, "not virtualized" as a flag in another. Absent, the values go in `queryParam`, repeated.
	 */
	filterCodec?: FilterCodec;
	/** The values a filter offers, when they can't be read off the loaded page. */
	filterOptions?: string[];
	/**
	 * The raw value the server groups this item under, when it differs from
	 * what `getValue` renders (e.g. `site_id` is a UUID in the database but
	 * a site name in the UI). Only needed on groupable
	 * fields: it's the key that matches a group to its total in
	 * the response's `group_counts`. Defaults to the displayed value.
	 */
	getGroupValue?: (item: T) => string | null;
	/**
	 * The header a row groups under, when grouping follows something other than the value the
	 * column shows. "Virtualized By" shows the runtime a host runs under but groups under the host
	 * at the top of its chain. Defaults to the displayed value.
	 */
	getGroupLabel?: (item: T) => string | null;
	/** Default checked values for the filter (applied on first load if no localStorage state). */
	filterDefaults?: string[];
	/**
	 * Grouping by this field draws each group as nested tree sections. See {@link TreeConfig}.
	 */
	tree?: TreeConfig<T>;
	/**
	 * This column carries the list's "Stale only" toggle. Staleness is a
	 * server-side constraint rather than a value filter, so it has no field of
	 * its own; marking the last-seen column gives it the same home as every
	 * other filter, in the column's header popover. Only takes effect
	 * when the server applies staleness from the list's query (`serverQuery.stale`).
	 */
	staleFilter?: boolean;
}

/**
 * Field with server-side ordering support.
 * The orderField property IS the backend OrderField value (e.g., 'name', 'created_at').
 * Presence of this property implies the field is sortable/groupable.
 */
export interface OrderableFieldConfig<
	T,
	O extends string,
	Q extends string = string
> extends BaseFieldConfig<T, Q> {
	orderField: O;
	/** Whether this field can be used for grouping. Defaults to true for string types. */
	groupable?: boolean;
}

/**
 * Display-only field (no server-side ordering): shown and filtered, never sorted or grouped.
 */
export interface DisplayFieldConfig<T, Q extends string = string> extends BaseFieldConfig<T, Q> {
	key: string;
}

/**
 * Union type for field configuration.
 * - Fields with `orderField` are sortable/groupable via backend
 * - Fields with `key` are display-only
 */
export type FieldConfig<T, O extends string = string, Q extends string = string> =
	OrderableFieldConfig<T, O, Q> | DisplayFieldConfig<T, Q>;

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

/**
 * The grouping and sort a tab opens with until the user picks their own.
 *
 * Declared beside the tab's fields and keyed by the same field keys. Each must
 * name a field whose column renders by default, since a list only groups and
 * sorts by rendered columns; the dev guard in `DataControls` enforces that.
 */
export interface TableDefaults<K extends string = string> {
	group?: K;
	sort?: { field: K; direction: 'asc' | 'desc' };
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
export type OrderableFieldEntry<T, Q extends string = string> = BaseFieldConfig<T, Q> & {
	groupable?: boolean;
};

/**
 * A map that REQUIRES an entry for every value in the OrderField union O.
 * TypeScript will error if any OrderField value is missing.
 */
type OrderableFieldsMap<T, O extends string, Q extends string = string> = {
	[K in O]: OrderableFieldEntry<T, Q>;
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
export function defineFields<T, O extends string, Q extends string = string>(
	orderableFields: OrderableFieldsMap<T, O, Q>,
	displayFields?: DisplayFieldConfig<T, Q>[]
): FieldConfig<T, O, Q>[] {
	const orderable = (Object.entries(orderableFields) as [O, OrderableFieldEntry<T, Q>][]).map(
		([orderField, config]) => ({
			...config,
			orderField
		})
	) as OrderableFieldConfig<T, O, Q>[];

	return [...orderable, ...(displayFields ?? [])];
}
