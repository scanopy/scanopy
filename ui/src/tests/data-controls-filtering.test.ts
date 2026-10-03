import { describe, it, expect } from 'vitest';
import {
	matchesFilters,
	matchesSearch,
	hasActiveFilters,
	serverFilterViolations,
	blankFilterState,
	toggleValue,
	toggleBoolean,
	restoredServerFilters,
	type FilterState,
	type ServerFilterMode
} from '$lib/shared/components/data/controls/filtering';
import { getUniqueValues } from '$lib/shared/components/data/controls/fieldValues';
import type { FieldConfig } from '$lib/shared/components/data/types';

interface Row {
	name: string | null;
	category: string | null;
	hidden: boolean | null;
	tags: string[];
}

function row(partial: Partial<Row>): Row {
	return { name: null, category: null, hidden: null, tags: [], ...partial };
}

const CLIENT_ONLY: ServerFilterMode = { tags: false, fields: false };

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

describe('matchesFilters — string fields', () => {
	it('matches everything while nothing is selected', () => {
		// Otherwise opening the filter panel would blank the list before the user
		// has chosen anything.
		const state: FilterState = { name: stringFilter([]) };

		expect(matchesFilters(row({ name: 'a' }), [nameField], state, CLIENT_ONLY)).toBe(true);
		expect(matchesFilters(row({ name: null }), [nameField], state, CLIENT_ONLY)).toBe(true);
	});

	it('keeps only selected values in include mode, dropping nulls', () => {
		const state: FilterState = { name: stringFilter(['a']) };

		expect(matchesFilters(row({ name: 'a' }), [nameField], state, CLIENT_ONLY)).toBe(true);
		expect(matchesFilters(row({ name: 'b' }), [nameField], state, CLIENT_ONLY)).toBe(false);
		expect(matchesFilters(row({ name: null }), [nameField], state, CLIENT_ONLY)).toBe(false);
	});

	it('hides selected values in exclude mode but keeps nulls', () => {
		// Asymmetric with include mode on purpose: a row with no value has nothing
		// to exclude, so excluding "a" must not also hide the unset rows.
		const excludeField: FieldConfig<Row> = { ...nameField, filterMode: 'exclude' };
		const state: FilterState = { name: stringFilter(['a']) };

		expect(matchesFilters(row({ name: 'a' }), [excludeField], state, CLIENT_ONLY)).toBe(false);
		expect(matchesFilters(row({ name: 'b' }), [excludeField], state, CLIENT_ONLY)).toBe(true);
		expect(matchesFilters(row({ name: null }), [excludeField], state, CLIENT_ONLY)).toBe(true);
	});
});

describe('matchesFilters — boolean fields', () => {
	it('treats a null boolean as unknown, so neither box excludes it', () => {
		const state: FilterState = {
			hidden: { type: 'boolean', values: new Set(), showTrue: false, showFalse: false }
		};

		expect(matchesFilters(row({ hidden: null }), [hiddenField], state, CLIENT_ONLY)).toBe(true);
	});

	it('matches nothing when both boxes are unchecked', () => {
		const state: FilterState = {
			hidden: { type: 'boolean', values: new Set(), showTrue: false, showFalse: false }
		};

		expect(matchesFilters(row({ hidden: true }), [hiddenField], state, CLIENT_ONLY)).toBe(false);
		expect(matchesFilters(row({ hidden: false }), [hiddenField], state, CLIENT_ONLY)).toBe(false);
	});

	it('selects each side independently', () => {
		const onlyTrue: FilterState = {
			hidden: { type: 'boolean', values: new Set(), showTrue: true, showFalse: false }
		};

		expect(matchesFilters(row({ hidden: true }), [hiddenField], onlyTrue, CLIENT_ONLY)).toBe(true);
		expect(matchesFilters(row({ hidden: false }), [hiddenField], onlyTrue, CLIENT_ONLY)).toBe(
			false
		);
	});
});

describe('matchesFilters — array fields', () => {
	it('matches when any value is selected', () => {
		const state: FilterState = { tags: { type: 'array', values: new Set(['prod']) } };

		expect(matchesFilters(row({ tags: ['prod', 'db'] }), [tagsField], state, CLIENT_ONLY)).toBe(
			true
		);
		expect(matchesFilters(row({ tags: ['db'] }), [tagsField], state, CLIENT_ONLY)).toBe(false);
		expect(matchesFilters(row({ tags: [] }), [tagsField], state, CLIENT_ONLY)).toBe(false);
	});
});

describe('matchesFilters — client-side tag filter', () => {
	// Tabs give the tags field a getValue returning tag *names*, so search can
	// match them, while the filter panel stores tag *ids*. Comparing the two hid
	// every row the moment a tag was selected.
	const tagNames: Record<string, string> = { 'id-prod': 'prod', 'id-db': 'db' };
	const namedTagsField: FieldConfig<Row> = {
		key: 'tags',
		label: 'Tags',
		type: 'array',
		searchable: true,
		filterable: true,
		getValue: (r) => r.tags.map((id) => tagNames[id])
	};
	const getItemTags = (r: Row) => r.tags;
	const state: FilterState = { tags: { type: 'array', values: new Set(['id-prod']) } };

	it('matches selected tag ids against the item tag ids', () => {
		const fields = [namedTagsField];

		expect(
			matchesFilters(row({ tags: ['id-prod', 'id-db'] }), fields, state, CLIENT_ONLY, getItemTags)
		).toBe(true);
		expect(matchesFilters(row({ tags: ['id-db'] }), fields, state, CLIENT_ONLY, getItemTags)).toBe(
			false
		);
	});

	it('keeps search matching the tag names', () => {
		expect(matchesSearch(row({ tags: ['id-prod'] }), [namedTagsField], 'prod')).toBe(true);
	});
});

describe('matchesFilters — server-side handover', () => {
	it('skips a serverFiltered field only when the parent handles it', () => {
		// Filtering a server-paginated list on the client would only ever narrow
		// the page in hand, silently hiding every match on other pages.
		const serverField: FieldConfig<Row> = {
			key: 'category',
			label: 'Category',
			type: 'string',
			filterable: true,
			serverFiltered: true,
			getValue: (r) => r.category
		};
		const state: FilterState = { category: stringFilter(['a']) };
		const item = row({ category: 'b' });

		expect(matchesFilters(item, [serverField], state, { tags: false, fields: true })).toBe(true);
		expect(matchesFilters(item, [serverField], state, { tags: false, fields: false })).toBe(false);
	});

	it('skips the tags field only when tag filtering is server-side', () => {
		const state: FilterState = { tags: { type: 'array', values: new Set(['prod']) } };
		const item = row({ tags: ['other'] });

		expect(matchesFilters(item, [tagsField], state, { tags: true, fields: false })).toBe(true);
		expect(matchesFilters(item, [tagsField], state, { tags: false, fields: false })).toBe(false);
	});

	it('ignores fields that are not filterable at all', () => {
		const plain: FieldConfig<Row> = { ...nameField, filterable: false };
		const state: FilterState = { name: stringFilter(['a']) };

		expect(matchesFilters(row({ name: 'zzz' }), [plain], state, CLIENT_ONLY)).toBe(true);
	});
});

describe('serverFilterViolations', () => {
	const SERVER_FIELDS: ServerFilterMode = { tags: true, fields: true };

	it('reports the shape that made a filtered host list keep the unfiltered count', () => {
		// HostTab offered these filters under server pagination while passing no
		// onFilterChange, so they narrowed the loaded page while total_count kept
		// describing every host — "62 of 1550", paging through the wrong rows.
		const hostFields: FieldConfig<Row>[] = [
			{ ...nameField, key: 'network_id', filterable: true },
			{ ...hiddenField, key: 'hidden' },
			tagsField
		];

		expect(serverFilterViolations(hostFields, true, { tags: true, fields: false })).toEqual([
			'network_id',
			'hidden'
		]);
	});

	it('clears once each field is handled by the side that produced the count', () => {
		const fields: FieldConfig<Row>[] = [
			{ ...nameField, key: 'network_id', filterable: true, serverFiltered: true },
			{ ...hiddenField, key: 'hidden', serverFiltered: true },
			tagsField
		];

		expect(serverFilterViolations(fields, true, SERVER_FIELDS)).toEqual([]);
	});

	it('says nothing about a client-paginated list, where filtering here is correct', () => {
		const fields: FieldConfig<Row>[] = [{ ...nameField, filterable: true }];

		expect(serverFilterViolations(fields, false, { tags: false, fields: false })).toEqual([]);
	});

	it('counts tags as handled only when the tag callback is wired', () => {
		expect(serverFilterViolations([tagsField], true, { tags: true, fields: false })).toEqual([]);
		expect(serverFilterViolations([tagsField], true, { tags: false, fields: true })).toEqual([
			'tags'
		]);
	});

	it('reports exactly the fields the client pass would still apply', () => {
		// The guard and the filter have to read one rule: a field the guard names
		// is precisely a field matchesFilters does not skip. Were they to drift,
		// the guard would either cry wolf or miss the bug it exists to catch.
		const field: FieldConfig<Row> = { ...nameField, filterable: true, serverFiltered: true };
		const state: FilterState = { name: stringFilter(['a']) };
		const nonMatching = row({ name: 'zzz' });

		for (const server of [
			{ tags: false, fields: false },
			{ tags: false, fields: true }
		] satisfies ServerFilterMode[]) {
			const clientPassSkipped = matchesFilters(nonMatching, [field], state, server);
			const reported = serverFilterViolations([field], true, server).length > 0;
			expect(reported).toBe(!clientPassSkipped);
		}
	});
});

describe('matchesSearch', () => {
	it('only searches fields that opted in', () => {
		// Matching every field by default meant a date field turned "2026" into a
		// hit on every row.
		const optedOut: FieldConfig<Row> = { ...nameField, searchable: false };

		expect(matchesSearch(row({ name: 'switch' }), [nameField], 'switch')).toBe(true);
		expect(matchesSearch(row({ name: 'switch' }), [optedOut], 'switch')).toBe(false);
	});

	it('matches any element of an array field', () => {
		const searchableTags: FieldConfig<Row> = { ...tagsField, searchable: true };

		expect(matchesSearch(row({ tags: ['prod', 'db'] }), [searchableTags], 'db')).toBe(true);
		expect(matchesSearch(row({ tags: ['prod'] }), [searchableTags], 'db')).toBe(false);
	});

	it('matches everything for a blank or whitespace query', () => {
		expect(matchesSearch(row({ name: null }), [nameField], '')).toBe(true);
		expect(matchesSearch(row({ name: null }), [nameField], '   ')).toBe(true);
	});

	it('is case insensitive', () => {
		expect(matchesSearch(row({ name: 'Switch' }), [nameField], 'sWiTcH')).toBe(true);
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

describe('getUniqueValues', () => {
	it('flattens array fields so each element is its own option', () => {
		const items = [row({ tags: ['b', 'a'] }), row({ tags: ['a'] })];

		expect(getUniqueValues(items, tagsField)).toEqual(['a', 'b']);
	});

	it('drops nulls and empty strings', () => {
		const items = [row({ name: 'a' }), row({ name: null }), row({ name: '' })];

		expect(getUniqueValues(items, nameField)).toEqual(['a']);
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

describe('restoredServerFilters', () => {
	const serverString: FieldConfig<Row> = {
		...nameField,
		key: 'network_id',
		filterable: true,
		serverFiltered: true
	};
	const serverBoolean: FieldConfig<Row> = { ...hiddenField, serverFiltered: true };

	it('replays a restored selection so the request matches the panel', () => {
		const state: FilterState = { network_id: stringFilter(['alpha']) };

		expect(restoredServerFilters([serverString], state)).toEqual([
			{ key: 'network_id', values: ['alpha'] }
		]);
	});

	it('says nothing about a filter that selects nothing', () => {
		expect(restoredServerFilters([serverString], { network_id: stringFilter([]) })).toEqual([]);
	});

	it('replays a boolean only while it constrains', () => {
		// A boolean constrains when a box is *un*checked — the opposite test from
		// a value set being non-empty, which is why they cannot share a branch.
		const unconstrained: FilterState = {
			hidden: { type: 'boolean', values: new Set(), showTrue: true, showFalse: true }
		};
		const constrained: FilterState = {
			hidden: { type: 'boolean', values: new Set(), showTrue: true, showFalse: false }
		};

		expect(restoredServerFilters([serverBoolean], unconstrained)).toEqual([]);
		expect(restoredServerFilters([serverBoolean], constrained)).toEqual([
			{ key: 'hidden', values: ['true'] }
		]);
	});

	it('ignores fields the parent does not handle server-side', () => {
		const clientOnly: FieldConfig<Row> = { ...nameField, key: 'network_id', filterable: true };

		expect(restoredServerFilters([clientOnly], { network_id: stringFilter(['alpha']) })).toEqual(
			[]
		);
	});
});
