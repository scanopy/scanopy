/**
 * The distinct values of an entity's orderable fields, with how many rows hold each. One request
 * counts every field a tab names.
 *
 * Any entity that mounts the generic `GET /api/v1/<entity>/field-values` endpoint can use this.
 * The path, the fields and the query parameters are all typed from that entity's operation in the
 * generated schema, so an entity that has not mounted the endpoint, or a field its `*OrderField`
 * lacks, fails to compile.
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
	[P in keyof paths]: P extends `${string}/field-values` ? P : never;
}[keyof paths];

type FieldValuesQueryParameters<P extends FieldValuesPath> = paths[P]['get']['parameters']['query'];

/** The fields a path can count: its entity's `*OrderField`. */
export type FieldValuesField<P extends FieldValuesPath> =
	FieldValuesQueryParameters<P>['fields'][number];

/** The list filters a path accepts, which narrow the rows counted. */
export type FieldValuesParams<P extends FieldValuesPath> = Omit<
	FieldValuesQueryParameters<P>,
	'fields'
>;

/** One distinct value and how many rows hold it. `value` is null for rows with none. */
export type FieldValueCount = components['schemas']['GroupCount'];

/** The counts of each field a query named, keyed by field. */
export type FieldValuesByField<F extends string> = Record<F, FieldValueCount[]>;

/** How one path's counts are cached and fetched. */
interface FieldValuesEndpoint<P extends FieldValuesPath> {
	/** The key the counts live under: inside the entity's key, so every invalidation of that
	 *  entity refetches them too. */
	key: () => readonly string[];
	fetch: (
		fields: FieldValuesField<P>[],
		query?: FieldValuesParams<P>
	) => Promise<{ field: FieldValuesField<P>; values: FieldValueCount[] }[]>;
}

/**
 * Each mounted path's cache key and request. Keyed over the path union, so mounting the endpoint
 * on a new entity fails to compile until it is named here. Each request names its literal path,
 * which openapi-fetch types fully, where it cannot resolve a generic one.
 */
const FIELD_VALUES_ENDPOINTS: { [P in FieldValuesPath]: FieldValuesEndpoint<P> } = {
	'/api/v1/hosts/field-values': {
		key: queryKeys.hosts.fieldValues,
		fetch: async (fields, query) =>
			unwrapData(
				await apiClient.GET('/api/v1/hosts/field-values', {
					params: { query: { ...query, fields } }
				})
			)
	},
	'/api/v1/services/field-values': {
		key: queryKeys.services.fieldValues,
		fetch: async (fields, query) =>
			unwrapData(
				await apiClient.GET('/api/v1/services/field-values', {
					params: { query: { ...query, fields } }
				})
			)
	},
	'/api/v1/discoveries/field-values': {
		key: queryKeys.discovery.fieldValues,
		fetch: async (fields, query) =>
			unwrapData(
				await apiClient.GET('/api/v1/discoveries/field-values', {
					params: { query: { ...query, fields } }
				})
			)
	},
	'/api/v1/daemon-api-keys/field-values': {
		key: queryKeys.apiKeys.fieldValues,
		fetch: async (fields, query) =>
			unwrapData(
				await apiClient.GET('/api/v1/daemon-api-keys/field-values', {
					params: { query: { ...query, fields } }
				})
			)
	},
	'/api/v1/user-api-keys/field-values': {
		key: queryKeys.userApiKeys.fieldValues,
		fetch: async (fields, query) =>
			unwrapData(
				await apiClient.GET('/api/v1/user-api-keys/field-values', {
					params: { query: { ...query, fields } }
				})
			)
	},
	'/api/v1/daemons/field-values': {
		key: queryKeys.daemons.fieldValues,
		fetch: async (fields, query) =>
			unwrapData(
				await apiClient.GET('/api/v1/daemons/field-values', {
					params: { query: { ...query, fields } }
				})
			)
	},
	'/api/v1/sites/field-values': {
		key: queryKeys.sites.fieldValues,
		fetch: async (fields, query) =>
			unwrapData(
				await apiClient.GET('/api/v1/sites/field-values', {
					params: { query: { ...query, fields } }
				})
			)
	},
	'/api/v1/tags/field-values': {
		key: queryKeys.tags.fieldValues,
		fetch: async (fields, query) =>
			unwrapData(
				await apiClient.GET('/api/v1/tags/field-values', {
					params: { query: { ...query, fields } }
				})
			)
	},
	'/api/v1/vlans/field-values': {
		key: queryKeys.vlans.fieldValues,
		fetch: async (fields, query) =>
			unwrapData(
				await apiClient.GET('/api/v1/vlans/field-values', {
					params: { query: { ...query, fields } }
				})
			)
	},
	'/api/v1/subnets/field-values': {
		key: queryKeys.subnets.fieldValues,
		fetch: async (fields, query) =>
			unwrapData(
				await apiClient.GET('/api/v1/subnets/field-values', {
					params: { query: { ...query, fields } }
				})
			)
	}
};

const fieldValuesQueryKey = (
	path: FieldValuesPath,
	fields: readonly string[],
	params: object | undefined
) => [...FIELD_VALUES_ENDPOINTS[path].key(), fields, params] as const;

/**
 * Counts for every field in `fields`, from one request: `data.site_id` holds the `site_id` counts.
 * Call it once per entity in a tab. Pass only the scope the list always carries (such as
 * `historical`), never the tab's active filters: the options would otherwise shrink to the rows
 * already selected.
 */
export function useFieldValuesQuery<
	P extends FieldValuesPath,
	const F extends FieldValuesField<P> & string
>(
	path: P,
	fields: readonly F[],
	params?: () => FieldValuesParams<P>,
	enabled: () => boolean = () => true
) {
	const endpoint: FieldValuesEndpoint<P> = FIELD_VALUES_ENDPOINTS[path];
	return createQuery(() => {
		const query = params?.();
		return {
			queryKey: fieldValuesQueryKey(path, fields, query),
			queryFn: async () => {
				const counted = await endpoint.fetch([...fields], query);
				// A field the response lacks reads as no values.
				const byField: Record<string, FieldValueCount[]> = Object.fromEntries(
					fields.map((field) => [field, []])
				);
				for (const { field, values } of counted) byField[field] = values;
				return byField as FieldValuesByField<F>;
			},
			enabled: enabled()
		};
	});
}

/**
 * The values some row holds, without nulls and empty strings: a filter's options, which the panel
 * labels through the field's `filterLabel` and orders by label.
 */
export function fieldValueOptions(counts: FieldValueCount[] | undefined): string[] {
	return [...new Set(presentFieldValues(counts))];
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
