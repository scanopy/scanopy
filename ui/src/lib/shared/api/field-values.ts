/**
 * The distinct values of one orderable field, with how many rows hold each.
 *
 * Any entity that mounts the generic `GET /api/v1/<entity>/field-values/{field}` endpoint can
 * use this. The path, the field and the query parameters are all typed from that entity's
 * operation in the generated schema, so an entity that has not mounted the endpoint, or a field
 * its `*OrderField` lacks, fails to compile.
 *
 * A server-paginated list's filter options come from here: the loaded page only holds the values
 * that happen to appear on it, and a fixture or registry lists values no row may hold.
 */

import { createQuery } from '@tanstack/svelte-query';
import { apiClient, type components, type paths } from '$lib/api/client';
import { queryKeys } from '$lib/api/query-client';
import { unwrapData } from '$lib/api/query-helpers';

/** Every API path that serves field values. */
export type FieldValuesPath = {
	[P in keyof paths]: P extends `${string}/field-values/{field}` ? P : never;
}[keyof paths];

type FieldValuesParameters<P extends FieldValuesPath> = paths[P]['get']['parameters'];

/** The fields a path can count: its entity's `*OrderField`. */
export type FieldValuesField<P extends FieldValuesPath> = FieldValuesParameters<P>['path']['field'];

/** The list filters a path accepts, which narrow the rows counted. */
export type FieldValuesParams<P extends FieldValuesPath> = NonNullable<
	FieldValuesParameters<P>['query']
>;

/** One distinct value and how many rows hold it. `value` is null for rows with none. */
export type FieldValueCount = components['schemas']['GroupCount'];

/** How one path's counts are cached and fetched. */
interface FieldValuesEndpoint<P extends FieldValuesPath> {
	/** The key the counts live under: inside the entity's key, so every invalidation of that
	 *  entity refetches them too. */
	key: () => readonly string[];
	fetch: (field: FieldValuesField<P>, query?: FieldValuesParams<P>) => Promise<FieldValueCount[]>;
}

/**
 * Each mounted path's cache key and request. Keyed over the path union, so mounting the endpoint
 * on a new entity fails to compile until it is named here. Each request names its literal path,
 * which openapi-fetch types fully, where it cannot resolve a generic one.
 */
const FIELD_VALUES_ENDPOINTS: { [P in FieldValuesPath]: FieldValuesEndpoint<P> } = {
	'/api/v1/hosts/field-values/{field}': {
		key: queryKeys.hosts.fieldValues,
		fetch: async (field, query) =>
			unwrapData(
				await apiClient.GET('/api/v1/hosts/field-values/{field}', {
					params: { path: { field }, query }
				})
			)
	},
	'/api/v1/services/field-values/{field}': {
		key: queryKeys.services.fieldValues,
		fetch: async (field, query) =>
			unwrapData(
				await apiClient.GET('/api/v1/services/field-values/{field}', {
					params: { path: { field }, query }
				})
			)
	},
	'/api/v1/discovery/field-values/{field}': {
		key: queryKeys.discovery.fieldValues,
		fetch: async (field, query) =>
			unwrapData(
				await apiClient.GET('/api/v1/discovery/field-values/{field}', {
					params: { path: { field }, query }
				})
			)
	}
};

const fieldValuesQueryKey = (path: FieldValuesPath, field: string, params: object | undefined) =>
	[...FIELD_VALUES_ENDPOINTS[path].key(), field, params] as const;

/**
 * Counts for one field. Pass only the scope the list always carries (such as `historical`), never
 * the tab's active filters: the options would otherwise shrink to the rows already selected.
 */
export function useFieldValuesQuery<P extends FieldValuesPath>(
	path: P,
	field: FieldValuesField<P>,
	params?: () => FieldValuesParams<P>,
	enabled: () => boolean = () => true
) {
	const endpoint: FieldValuesEndpoint<P> = FIELD_VALUES_ENDPOINTS[path];
	return createQuery(() => {
		const query = params?.();
		return {
			queryKey: fieldValuesQueryKey(path, field, query),
			queryFn: () => endpoint.fetch(field, query),
			enabled: enabled()
		};
	});
}

/** The non-null, non-empty values, sorted for a filter panel. */
export function fieldValueOptions(counts: FieldValueCount[] | undefined): string[] {
	return labelledFieldValueOptions(counts, (value) => value);
}

/**
 * The present values rendered as the labels a field's `getValue` returns, deduplicated and
 * sorted. `label` returns null for a value it cannot render, which is then left out.
 */
export function labelledFieldValueOptions(
	counts: FieldValueCount[] | undefined,
	label: (value: string) => string | null | undefined
): string[] {
	const labels = presentFieldValues(counts)
		.map(label)
		.filter((name): name is string => name !== null && name !== undefined && name !== '');
	return [...new Set(labels)].sort((a, b) => a.localeCompare(b));
}

/** The raw values that occur, without nulls and empty strings. */
function presentFieldValues(counts: FieldValueCount[] | undefined): string[] {
	return (counts ?? [])
		.map((count) => count.value)
		.filter((value): value is string => value !== null && value !== undefined && value !== '');
}

/** Whether some row holds no value: a null, or the empty string a `COALESCE(…, '')` yields. */
export function hasEmptyFieldValue(counts: FieldValueCount[] | undefined): boolean {
	return (counts ?? []).some((count) => count.value === null || count.value === '');
}
