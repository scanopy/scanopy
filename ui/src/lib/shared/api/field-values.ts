/**
 * The distinct values of one orderable field, with how many rows hold each.
 *
 * Any entity that mounts the generic `GET /api/v1/<entity>/field-values/{field}` endpoint can
 * use this. The path, the field and the query parameters are all typed from that entity's
 * operation in the generated schema, so an entity that has not mounted the endpoint, or a field
 * its `*OrderField` lacks, fails to compile.
 *
 * A server-paginated list's filter options come from here: the loaded page only holds the values
 * that happen to appear on it.
 */

import { createQuery } from '@tanstack/svelte-query';
import { apiClient, type components, type paths } from '$lib/api/client';
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

export const fieldValuesQueryKey = (path: string, field: string, params: object | undefined) =>
	['field-values', path, field, params] as const;

export function useFieldValuesQuery<P extends FieldValuesPath>(
	path: P,
	field: FieldValuesField<P>,
	params?: () => FieldValuesParams<P>
) {
	// The caller is typed against its own path; the request is made against the union of them,
	// which openapi-fetch can resolve where it cannot resolve a generic path.
	const url: FieldValuesPath = path;
	const pathField: FieldValuesField<FieldValuesPath> = field;
	return createQuery(() => {
		const query: FieldValuesParams<FieldValuesPath> | undefined = params?.();
		return {
			queryKey: fieldValuesQueryKey(url, pathField, query),
			queryFn: async (): Promise<FieldValueCount[]> =>
				unwrapData(
					await apiClient.GET(url, {
						params: { path: { field: pathField }, query }
					})
				)
		};
	});
}

/** The non-null values, sorted for a filter panel. */
export function fieldValueOptions(counts: FieldValueCount[] | undefined): string[] {
	return (counts ?? [])
		.map((count) => count.value)
		.filter((value): value is string => value !== null && value !== undefined && value !== '')
		.sort((a, b) => a.localeCompare(b));
}
