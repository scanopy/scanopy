import { tagGroupFieldKey, tagGroupIdOf } from '#lib/features/tags/groupFields.ts';
import { SEARCH_MAX_LENGTH } from '#lib/shared/components/forms/input/SearchInput.svelte';
import { getFieldKey, type FieldConfig, type FilterCodec } from '../types';
import { parseSortState, type StoredFieldFilter } from './dataControlsStorage';
import { booleanFilterValues, isFieldFilterActive, type FilterState } from './filtering';
import type { SortState } from './sorting';

/**
 * A list's view as query params, named and valued as the list's endpoint takes them.
 *
 * One pass builds both the URL a list shows and the request its parent sends, so a link and the
 * API return the same rows: for a list the server filters, the URL is the request plus `page`. A list
 * held whole uses the same names, from each field's `queryParam` or key, and
 * applies them here.
 */

export const SEARCH_PARAM = 'search';
export const STALE_PARAM = 'stale';
export const PAGE_PARAM = 'page';
const TAG_IDS_PARAM = 'tag_ids';
const ORDER_BY = 'order_by';
const ORDER_DIRECTION = 'order_direction';
const GROUP_BY = 'group_by';
const ORDER_BY_TAG_GROUP = 'order_by_tag_group';
const GROUP_BY_TAG_GROUP = 'group_by_tag_group';
const ORDER_PARAMS = [ORDER_BY, ORDER_DIRECTION, GROUP_BY, ORDER_BY_TAG_GROUP, GROUP_BY_TAG_GROUP];

/**
 * The parts of the view an endpoint may not take. Ordering and `serverFiltered` fields always
 * reach the server; search, tags and staleness only where the endpoint has them. A list whose
 * endpoint takes no search shows no search box.
 */
export interface ServerQuery {
	tags: boolean;
	stale: boolean;
	search: boolean;
}

export interface ListQueryState {
	sort: SortState;
	group: string | null;
	filterState: FilterState;
	staleOnly: boolean;
	search: string;
}

/** The param a field's filter goes in when it takes one. */
function filterParam<T>(field: FieldConfig<T>): string {
	const key = getFieldKey(field);
	return field.queryParam ?? (key === 'tags' ? TAG_IDS_PARAM : key);
}

/** A field's codec: its own, else its values repeated in its param. */
export function filterCodecOf<T>(field: FieldConfig<T>): FilterCodec {
	if (field.filterCodec) return field.filterCodec;
	const param = filterParam(field);
	return {
		keys: [param],
		encode: (values) => values.map((value) => [param, value]),
		decode: (params) => (params.has(param) ? params.getAll(param).filter(Boolean) : undefined)
	};
}

/** The selection a filter holds, as values: a boolean's checked boxes as `true`/`false`. */
function selectedValues(filter: FilterState[string]): string[] {
	if (filter.type === 'boolean') {
		return booleanFilterValues(filter.showTrue ?? false, filter.showFalse ?? false);
	}
	return Array.from(filter.values);
}

/**
 * The params a boolean filter writes. Neither box checked writes the param empty, since leaving
 * it out would read as "no filter".
 */
function encodeFilter<T>(field: FieldConfig<T>, filter: FilterState[string]): [string, string][] {
	const values = selectedValues(filter);
	const codec = filterCodecOf(field);
	if (filter.type === 'boolean' && values.length === 0) return [[codec.keys[0], '']];
	return codec.encode(values);
}

function serverApplies<T>(field: FieldConfig<T>, server: ServerQuery): boolean {
	if (getFieldKey(field) === 'tags') return server.tags;
	return field.serverFiltered === true;
}

function orderEntries(sort: SortState, group: string | null): [string, string][] {
	const entries: [string, string][] = [];
	if (group !== null) {
		const tagGroup = tagGroupIdOf(group);
		entries.push(tagGroup ? [GROUP_BY_TAG_GROUP, tagGroup] : [GROUP_BY, group]);
	}
	if (sort.field !== null) {
		const tagGroup = tagGroupIdOf(sort.field);
		entries.push(tagGroup ? [ORDER_BY_TAG_GROUP, tagGroup] : [ORDER_BY, sort.field]);
	}
	// Written even with no sort field: some order param has to be present for a link to say "no
	// sort, no grouping" over a tab's default.
	entries.push([ORDER_DIRECTION, sort.direction]);
	return entries;
}

/**
 * The view as params, and the part of it the server applies. Filters, search and staleness are
 * omitted when inactive. The ordering is always in the request, since the server doesn't know the
 * tab's default.
 */
export function listParams<T>(
	state: ListQueryState,
	fields: FieldConfig<T>[],
	server: ServerQuery
): { url: URLSearchParams; request: URLSearchParams } {
	const url = new URLSearchParams();
	const request = new URLSearchParams();
	const add = (entries: [string, string][], toServer: boolean) => {
		for (const [key, value] of entries) {
			url.append(key, value);
			if (toServer) request.append(key, value);
		}
	};

	const search = state.search.trim();
	if (search) add([[SEARCH_PARAM, search]], server.search);

	for (const field of fields) {
		const filter = state.filterState[getFieldKey(field)];
		if (!filter || !isFieldFilterActive(field, filter)) continue;
		add(encodeFilter(field, filter), serverApplies(field, server));
	}

	if (state.staleOnly) add([[STALE_PARAM, 'true']], server.stale);

	add(orderEntries(state.sort, state.group), true);

	return { url, request };
}

/** Every param a list with these fields may write, so the URL manager can clear them. */
export function listParamKeys<T>(fields: FieldConfig<T>[]): string[] {
	return [
		SEARCH_PARAM,
		STALE_PARAM,
		PAGE_PARAM,
		...ORDER_PARAMS,
		...fields.filter((f) => f.filterable).flatMap((f) => filterCodecOf(f).keys)
	];
}

/** What a list URL carried. Undefined parts were absent or malformed. */
export interface DecodedListView {
	sort?: SortState;
	group?: string | null;
	filterState: Record<string, StoredFieldFilter>;
	staleOnly: boolean;
	search: string;
	page?: number;
}

function decodeOrdering(params: URLSearchParams): { sort?: SortState; group?: string | null } {
	if (!ORDER_PARAMS.some((key) => params.has(key))) return {};

	const groupTag = params.get(GROUP_BY_TAG_GROUP);
	const group = groupTag ? tagGroupFieldKey(groupTag) : params.get(GROUP_BY) || null;

	const orderTag = params.get(ORDER_BY_TAG_GROUP);
	const field = orderTag ? tagGroupFieldKey(orderTag) : params.get(ORDER_BY) || null;
	const direction = params.get(ORDER_DIRECTION) ?? 'asc';
	// An unknown direction drops the sort alone; the grouping still applies.
	const sort =
		direction === 'asc' || direction === 'desc'
			? parseSortState({ field, direction }, true)
			: undefined;

	return { sort, group };
}

/** The filter type a field's filter has, as `blankFilter` shapes it. */
function filterTypeOf<T>(field: FieldConfig<T>): StoredFieldFilter['type'] {
	return field.type === 'boolean' || field.type === 'array' ? field.type : 'string';
}

/** The filters these fields' params carry, keyed by field. */
export function decodeFilters<T>(
	params: URLSearchParams,
	fields: FieldConfig<T>[]
): Record<string, StoredFieldFilter> {
	const filters: Record<string, StoredFieldFilter> = {};
	for (const field of fields) {
		if (!field.filterable) continue;
		const values = filterCodecOf(field).decode(params);
		if (values === undefined) continue;

		const type = filterTypeOf(field);
		filters[getFieldKey(field)] =
			type === 'boolean'
				? {
						type,
						values: [],
						showTrue: values.includes('true'),
						showFalse: values.includes('false')
					}
				: { type, values };
	}
	return filters;
}

/**
 * Whether these params carry a list view at all. A URL with none leaves the list to its saved
 * state; a URL with any describes the whole view, and what it leaves out is at its default.
 */
export function hasListParams<T>(params: URLSearchParams, fields: FieldConfig<T>[]): boolean {
	const keys = new Set(listParamKeys(fields));
	return [...params.keys()].some((key) => keys.has(key));
}

/**
 * Read a view from list params. Each part is read on its own, so a malformed one (an unknown
 * direction, a page that isn't a positive integer) is ignored and the rest still apply. A sort or
 * group on a field the list doesn't have is left for `resolveOrdering`, which lands it on the
 * tab's default; a param for a filter it doesn't have is never read.
 */
export function decodeListParams<T>(
	params: URLSearchParams,
	fields: FieldConfig<T>[]
): DecodedListView {
	const page = params.get(PAGE_PARAM);

	return {
		...decodeOrdering(params),
		filterState: decodeFilters(params, fields),
		staleOnly: params.get(STALE_PARAM) === 'true',
		search: (params.get(SEARCH_PARAM) ?? '').slice(0, SEARCH_MAX_LENGTH),
		page: page !== null && /^\d+$/.test(page) && Number(page) > 0 ? Number(page) : undefined
	};
}

/** The filter value of "none": a row with no value, offered as its own option. */
export const NO_VALUE = '';

/**
 * A filter the API takes as values in one param and "none" as a flag in another, such as
 * Virtualized By: `virtualization_service_names` and `include_unvirtualized`.
 */
export function valuesOrNoneCodec<P extends string>(valuesParam: P, noneParam: P): FilterCodec {
	return {
		keys: [valuesParam, noneParam],
		encode: (values) => [
			...values
				.filter((value) => value !== NO_VALUE)
				.map((value): [string, string] => [valuesParam, value]),
			...(values.includes(NO_VALUE) ? [[noneParam, 'true'] as [string, string]] : [])
		],
		decode: (params) => {
			const values = params.getAll(valuesParam).filter(Boolean);
			if (params.get(noneParam) === 'true') values.push(NO_VALUE);
			return values.length > 0 ? values : undefined;
		}
	};
}

/** Whether a list's query narrows its rows: anything beyond ordering. */
export function narrowsList(query: URLSearchParams): boolean {
	return [...query.keys()].some((key) => !ORDER_PARAMS.includes(key));
}
