/**
 * The metadata filters a view declares, read from the generated views fixture.
 *
 * Each filter names the entities it reads and hides (`entities`). The hide-set stays keyed by
 * entity and filter type, `hide_metadata_values[view][entity][filter_type]`: a filter covering
 * several entities writes a hidden value under each of them, and the backend guarantees a view
 * covers each (entity, filter type) pair with at most one filter, so the key is unambiguous.
 */

import type { components } from '$lib/api/schema';
import { views } from '$lib/shared/stores/metadata';

export type ViewElementConfig = components['schemas']['ViewElementConfig'];
export type MetadataFilter = components['schemas']['MetadataFilter'];
export type EntityType = components['schemas']['EntityDiscriminants'];

/** Hidden value ids keyed by entity type, then filter type: one view's slice of the hide-set. */
export type HiddenMetadataValues = Record<string, Record<string, string[]>>;

export function viewElementConfig(view: string): ViewElementConfig | undefined {
	return (views.getMetadata(view) as { element_config?: ViewElementConfig } | null)?.element_config;
}

export function declaredMetadataFilters(view: string): MetadataFilter[] {
	return viewElementConfig(view)?.metadata_filters ?? [];
}

/** The filters that read and hide `entityType` in `view`. */
export function filtersFor(view: string, entityType: string): MetadataFilter[] {
	return declaredMetadataFilters(view).filter((f) => (f.entities as string[]).includes(entityType));
}

/** The filter of `filterType` covering `entityType` in `view`, if the view declares one. */
export function filterFor(
	view: string,
	entityType: string,
	filterType: string
): MetadataFilter | undefined {
	return filtersFor(view, entityType).find((f) => f.filter_type === filterType);
}

/** Whether `filter` covers more than one entity, so it renders in the view-wide section. */
export function coversSeveralEntities(filter: MetadataFilter): boolean {
	return filter.entities.length > 1;
}

/**
 * The values `filter` hides: the union across the entities it covers. A chip reads as hidden when
 * any of them hides it, and toggling it writes every one, so the entries converge on first use.
 */
export function hiddenValuesFor(
	filter: MetadataFilter,
	hidden: HiddenMetadataValues | undefined
): string[] {
	const out = new Set<string>();
	for (const entity of filter.entities) {
		for (const value of hidden?.[entity]?.[filter.filter_type] ?? []) out.add(value);
	}
	return [...out];
}

/** `hidden` with `filter`'s hidden values set to `values` under every entity it covers. */
export function withFilterValues(
	filter: MetadataFilter,
	hidden: HiddenMetadataValues | undefined,
	values: string[]
): HiddenMetadataValues {
	const next: HiddenMetadataValues = { ...(hidden ?? {}) };
	for (const entity of filter.entities) {
		next[entity] = { ...(next[entity] ?? {}), [filter.filter_type]: [...values] };
	}
	return next;
}
