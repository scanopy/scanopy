import { describe, it, expect } from 'vitest';
import {
	hasActiveFilters,
	serverFilterViolations,
	blankFilterState,
	toggleValue,
	toggleBoolean,
	type FilterState,
	type ServerFilterMode
} from '#lib/shared/components/data/controls/filtering.ts';
import { filterOptionsOf } from '#lib/shared/components/data/controls/fieldValues.ts';
import type { FieldConfig } from '#lib/shared/components/data/types.ts';

interface Row {
	name: string | null;
	category: string | null;
	hidden: boolean | null;
	tags: string[];
}

function row(partial: Partial<Row>): Row {
	return { name: null, category: null, hidden: null, tags: [], ...partial };
}

const nameField: FieldConfig<Row> = {
	key: 'name',
	label: 'Name',
	type: 'string',
	searchable: true,
	filterable: true,
	getValue: (r) => r.name
};
const hiddenField: FieldConfig<Row> = {
	key: 'hidden',
	label: 'Hidden',
	type: 'boolean',
	filterable: true,
	getValue: (r) => r.hidden
};
const tagsField: FieldConfig<Row> = {
	key: 'tags',
	label: 'Tags',
	type: 'array',
	filterable: true,
	getValue: (r) => r.tags
};

function stringFilter(values: string[]): FilterState[string] {
	return { type: 'string', values: new Set(values) };
}

describe('serverFilterViolations', () => {
	const SERVER_FIELDS: ServerFilterMode = { tags: true };

	it('reports the shape that made a filtered host list keep the unfiltered count', () => {
		// HostTab offered these filters under server pagination while passing no
		// onFilterChange, so they narrowed the loaded page while total_count kept
		// describing every host — "62 of 1550", paging through the wrong rows.
		const hostFields: FieldConfig<Row>[] = [
			{ ...nameField, key: 'site_id', filterable: true },
			{ ...hiddenField, key: 'hidden' },
			tagsField
		];

		expect(serverFilterViolations(hostFields, { tags: true })).toEqual(['site_id', 'hidden']);
	});

	it('clears once each field is handled by the side that produced the count', () => {
		const fields: FieldConfig<Row>[] = [
			{ ...nameField, key: 'site_id', filterable: true, serverFiltered: true },
			{ ...hiddenField, key: 'hidden', serverFiltered: true },
			tagsField
		];

		expect(serverFilterViolations(fields, SERVER_FIELDS)).toEqual([]);
	});

	it('counts tags as handled only when the tag callback is wired', () => {
		expect(serverFilterViolations([tagsField], { tags: true })).toEqual([]);
		expect(serverFilterViolations([tagsField], { tags: false })).toEqual(['tags']);
	});
});

describe('hasActiveFilters', () => {
	it('reports staleness on its own', () => {
		expect(hasActiveFilters([nameField], {}, true)).toBe(true);
	});

	it('reports a boolean field only when a side is unchecked', () => {
		const both: FilterState = {
			hidden: { type: 'boolean', values: new Set(), showTrue: true, showFalse: true }
		};
		const one: FilterState = {
			hidden: { type: 'boolean', values: new Set(), showTrue: true, showFalse: false }
		};

		expect(hasActiveFilters([hiddenField], both, false)).toBe(false);
		expect(hasActiveFilters([hiddenField], one, false)).toBe(true);
	});

	it('reports a value field only once something is selected', () => {
		expect(hasActiveFilters([nameField], { name: stringFilter([]) }, false)).toBe(false);
		expect(hasActiveFilters([nameField], { name: stringFilter(['a']) }, false)).toBe(true);
	});
});

describe('filterOptionsOf', () => {
	it('flattens array fields so each element is its own option', () => {
		const items = [row({ tags: ['b', 'a'] }), row({ tags: ['a'] })];

		expect(filterOptionsOf(items, tagsField)).toEqual(['a', 'b']);
	});

	it('drops nulls and empty strings', () => {
		const items = [row({ name: 'a' }), row({ name: null }), row({ name: '' })];

		expect(filterOptionsOf(items, nameField)).toEqual(['a']);
	});

	it('offers the filter values, ordered by the label each shows', () => {
		// The panel holds ids and shows names: ordering by the id would scatter the names.
		const names: Record<string, string> = { s1: 'Zurich', s2: 'Amsterdam' };
		const site: FieldConfig<Row> = {
			...nameField,
			getValue: (r) => names[r.name ?? ''] ?? null,
			getFilterValue: (r) => r.name,
			filterLabel: (id) => names[id] ?? id
		};

		expect(filterOptionsOf([row({ name: 's1' }), row({ name: 's2' })], site)).toEqual(['s2', 's1']);
	});
});

describe('filter state transitions', () => {
	const category: FieldConfig<Row> = {
		key: 'category',
		label: 'Category',
		type: 'string',
		filterable: true,
		filterDefaults: ['OpenPorts'],
		getValue: (r) => r.category
	};

	it('seeds a fresh mount from the field defaults but a clear from nothing', () => {
		// The two callers differ only here, and the difference is deliberate:
		// opening the list applies the product's default, while "clear filters"
		// clears that default too rather than reinstating it.
		expect([...blankFilterState([category], true).category.values]).toEqual(['OpenPorts']);
		expect([...blankFilterState([category], false).category.values]).toEqual([]);
	});

	it('gives a boolean field both boxes checked, which is no constraint', () => {
		const state = blankFilterState([hiddenField], true);

		expect(state.hidden.showTrue).toBe(true);
		expect(state.hidden.showFalse).toBe(true);
	});

	it('skips fields that are not filterable', () => {
		const plain: FieldConfig<Row> = { ...nameField, filterable: false };

		expect(Object.keys(blankFilterState([plain, category], true))).toEqual(['category']);
	});

	it('adds and removes a value without mutating the state it was given', () => {
		// The component reassigns rather than mutates so downstream $derived work
		// re-runs; a transition that edited in place would render stale.
		const before = blankFilterState([category], false);

		const added = toggleValue(before, 'category', 'Web')!;
		expect([...added.category.values]).toEqual(['Web']);
		expect([...before.category.values]).toEqual([]);

		const removed = toggleValue(added, 'category', 'Web')!;
		expect([...removed.category.values]).toEqual([]);
		expect([...added.category.values]).toEqual(['Web']);
	});

	it('declines a key that names no togglable filter', () => {
		expect(toggleValue(blankFilterState([category], false), 'nonexistent', 'x')).toBeNull();
		expect(toggleValue(blankFilterState([hiddenField], true), 'hidden', 'x')).toBeNull();
	});

	it('lets a client-side boolean select nothing, but never a server-side one', () => {
		// Unticking the last box asks for an empty set. A query string cannot
		// express that, and it renders an empty table under a non-zero count — so
		// a server-side filter falls back to no constraint instead.
		const state = blankFilterState([hiddenField], true);
		const onlyTrue = toggleBoolean(state, 'hidden', 'showFalse', false)!;

		const client = toggleBoolean(onlyTrue.state, 'hidden', 'showTrue', false)!;
		expect([client.showTrue, client.showFalse]).toEqual([false, false]);

		const server = toggleBoolean(onlyTrue.state, 'hidden', 'showTrue', true)!;
		expect([server.showTrue, server.showFalse]).toEqual([true, true]);
	});

	it('narrows to one value when a box is unticked', () => {
		const state = blankFilterState([hiddenField], true);
		const next = toggleBoolean(state, 'hidden', 'showFalse', true)!;

		expect([next.showTrue, next.showFalse]).toEqual([true, false]);
	});
});
