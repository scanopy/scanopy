import type { IconComponent } from '$lib/shared/utils/types';
import type { Snippet } from 'svelte';
import type { Color } from '$lib/shared/utils/styling';
import type { EntityDiscriminants } from '$lib/api/entities';

// ============================================================================
// Page Size Configuration
// ============================================================================

export const PAGE_SIZE_OPTIONS = [20, 50, 100] as const;
export type PageSizeOption = (typeof PAGE_SIZE_OPTIONS)[number];

export interface TagProps {
	label: string;
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

export interface CardFieldItem {
	id: string;
	label: string;
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
 * Base configuration shared by all field types.
 */
interface BaseFieldConfig<T> {
	type: 'string' | 'boolean' | 'date' | 'array';
	label: string;
	searchable?: boolean;
	filterable?: boolean;
	getValue?: (item: T) => string | boolean | Date | string[] | null;
	/** 'include' (default): checked values shown. 'exclude': checked values hidden. */
	filterMode?: 'include' | 'exclude';
	/** External filter options (bypasses getUniqueValues). Use when values aren't derivable from current items (e.g., server-side pagination). */
	filterOptions?: string[];
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
	| OrderableFieldConfig<T, O>
	| DisplayFieldConfig<T>;

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
