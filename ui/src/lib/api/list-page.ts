/**
 * One page of a server-paginated list, as every entity table reads it.
 *
 * A list's own query key holds the page and the list's view, so paging never overwrites the full
 * list other consumers (pickers, topology, lookups) read from the entity's `all` key. The key sits
 * under the entity's `all` prefix, so invalidating the entity refreshes the page too.
 */

import { createQuery, keepPreviousData, type QueryClient } from '@tanstack/svelte-query';
import type { components } from '#lib/api/schema';
import { unwrapEnvelope, type FetchResult } from './query-helpers';
import { withListParams } from './list-params';

export type PaginationMeta = components['schemas']['PaginationMeta'];

/** A page of rows and the pagination the server reported for it. */
export interface PaginatedResult<T> {
	items: T[];
	pagination: PaginationMeta | null;
}

/** Which page to read, and the list's view as the endpoint's own params. */
export interface ListPageParams {
	limit: number;
	offset: number;
	/** The view `DataControls` sends: search, filters and ordering. */
	list?: URLSearchParams;
	enabled?: boolean;
}

/** The page and the serializer that appends the view, for an endpoint call. */
export interface PageRequest {
	query: { limit: number; offset: number };
	querySerializer: (query: Record<string, unknown>) => string;
}

/**
 * A server-paginated list query.
 *
 * @param key - The entity's `all` key; the page and view are appended to it.
 * @param fetchPage - Calls the list endpoint with the request it is given, plus any params of its
 *   own that never change (such as `historical`).
 * @param params - The page and view, read reactively.
 */
export function useListPageQuery<T>(
	key: readonly unknown[],
	fetchPage: (
		request: PageRequest
	) => Promise<FetchResult<{ data?: T[] | null; meta?: { pagination?: PaginationMeta } }>>,
	params: () => ListPageParams
) {
	return createQuery(() => {
		const { limit, offset, list, enabled = true } = params();
		return {
			queryKey: [...key, 'page', { limit, offset, list: list?.toString() }],
			queryFn: async (): Promise<PaginatedResult<T>> => {
				const envelope = unwrapEnvelope(
					await fetchPage({ query: { limit, offset }, querySerializer: withListParams(list) })
				);
				return { items: envelope.data, pagination: envelope.meta?.pagination ?? null };
			},
			enabled,
			// Keep showing the previous page while the next one loads.
			placeholderData: keepPreviousData
		};
	});
}

/** What `listIsEmpty` reads from a list page query. */
export interface ListPageQueryState {
	data?: { items: unknown[] };
	isPlaceholderData: boolean;
}

/**
 * Whether a list holds no rows at all, so its tab shows "none yet" in place of the table.
 *
 * Decided only from a response for the view on screen. While a changed view's request is in
 * flight, the previous page stays up as placeholder data; judged against the new view, a
 * filtered-empty page would read as "none yet" the moment a filter is cleared. The tab would then
 * unmount the table before it saved the cleared view, and remount it with the old filter.
 */
export function listIsEmpty(query: ListPageQueryState, narrowed: boolean): boolean {
	return !query.isPlaceholderData && !narrowed && (query.data?.items.length ?? 0) === 0;
}

/**
 * Refetch every loaded page of an entity's list. For a write that patches the entity's full-list
 * cache in place, which the pages, under their own keys, do not read.
 */
export function invalidateListPages(queryClient: QueryClient, key: readonly unknown[]) {
	return queryClient.invalidateQueries({ queryKey: [...key, 'page'] });
}
