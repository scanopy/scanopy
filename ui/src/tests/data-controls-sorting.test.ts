import { describe, it, expect } from 'vitest';
import {
	compareByField,
	sortItems,
	nextSortState,
	sortableFields,
	groupableFields,
	serverOrderViolations,
	type SortState
} from '$lib/shared/components/data/controls/sorting';
import {
	groupItems,
	computeGroupOffsets,
	serverGroupKey
} from '$lib/shared/components/data/controls/grouping';
import { fieldsToColumns } from '$lib/shared/components/data/table/columns';
import type { FieldConfig } from '$lib/shared/components/data/types';

interface Row {
	name: string | null;
	seen: string | null;
	active: boolean | null;
	tags: string[];
}

function row(partial: Partial<Row>): Row {
	return { name: null, seen: null, active: null, tags: [], ...partial };
}

const nameField: FieldConfig<Row> = {
	key: 'name',
	label: 'Name',
	type: 'string',
	getValue: (r) => r.name
};
const dateField: FieldConfig<Row> = {
	key: 'seen',
	label: 'Seen',
	type: 'date',
	getValue: (r) => r.seen
};
const boolField: FieldConfig<Row> = {
	key: 'active',
	label: 'Active',
	type: 'boolean',
	getValue: (r) => r.active
};
const arrayField: FieldConfig<Row> = {
	key: 'tags',
	label: 'Tags',
	type: 'array',
	getValue: (r) => r.tags
};

const fields = [nameField, dateField, boolField, arrayField];

const asc: SortState = { field: 'name', direction: 'asc' };
const desc: SortState = { field: 'name', direction: 'desc' };

describe('sorting', () => {
	it('sorts rows with no value last in both directions', () => {
		// A row with no value is missing data, not an extreme. Surfacing a page of
		// blanks at the top of a descending sort would bury what the user asked for.
		const items = [row({ name: null }), row({ name: 'b' }), row({ name: 'a' })];

		expect(sortItems(items, fields, asc).map((r) => r.name)).toEqual(['a', 'b', null]);
		expect(sortItems(items, fields, desc).map((r) => r.name)).toEqual(['b', 'a', null]);
	});

	it('orders embedded numbers numerically rather than by codepoint', () => {
		const items = [row({ name: 'host10' }), row({ name: 'host9' }), row({ name: 'host1' })];

		expect(sortItems(items, fields, asc).map((r) => r.name)).toEqual(['host1', 'host9', 'host10']);
	});

	it('compares dates by instant, mixing Date objects and ISO strings', () => {
		// getValue is typed to allow either, and the server sends strings.
		const mixed: FieldConfig<Row> = {
			key: 'seen',
			label: 'Seen',
			type: 'date',
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			getValue: (r) => (r.name === 'obj' ? (new Date(r.seen!) as any) : r.seen)
		};

		const items = [
			row({ name: 'str', seen: '2026-03-01T00:00:00Z' }),
			row({ name: 'obj', seen: '2026-01-01T00:00:00Z' })
		];

		const sorted = sortItems(items, [mixed], { field: 'seen', direction: 'asc' });
		expect(sorted.map((r) => r.name)).toEqual(['obj', 'str']);
	});

	it('is idempotent — sorting an already sorted list changes nothing', () => {
		const items = [row({ name: 'c' }), row({ name: 'a' }), row({ name: 'b' })];
		const once = sortItems(items, fields, asc);
		const twice = sortItems(once, fields, asc);

		expect(twice.map((r) => r.name)).toEqual(once.map((r) => r.name));
	});

	it('reverses exactly when there are no ties and no nulls', () => {
		const items = [row({ name: 'c' }), row({ name: 'a' }), row({ name: 'b' })];

		expect(sortItems(items, fields, desc).map((r) => r.name)).toEqual(
			sortItems(items, fields, asc)
				.map((r) => r.name)
				.reverse()
		);
	});

	it('does not mutate the input array', () => {
		const items = [row({ name: 'c' }), row({ name: 'a' })];
		sortItems(items, fields, asc);

		expect(items.map((r) => r.name)).toEqual(['c', 'a']);
	});

	it('leaves items untouched when the sorted field is not configured', () => {
		const items = [row({ name: 'c' }), row({ name: 'a' })];
		const result = sortItems(items, fields, { field: 'nonexistent', direction: 'asc' });

		expect(result.map((r) => r.name)).toEqual(['c', 'a']);
	});

	it('orders arrays by length, then by first element', () => {
		const items = [
			row({ name: 'two', tags: ['b', 'z'] }),
			row({ name: 'none', tags: [] }),
			row({ name: 'twoA', tags: ['a', 'z'] })
		];

		const sorted = sortItems(items, fields, { field: 'tags', direction: 'asc' });
		expect(sorted.map((r) => r.name)).toEqual(['none', 'twoA', 'two']);
	});

	it('orders false before true ascending', () => {
		const items = [row({ name: 't', active: true }), row({ name: 'f', active: false })];

		const sorted = sortItems(items, fields, { field: 'active', direction: 'asc' });
		expect(sorted.map((r) => r.name)).toEqual(['f', 't']);
	});

	it('keeps a server-ordered page in the order the server sent it', () => {
		// MACs in the server's order. Re-sorted here, `14:..` would drop below `7E:..`.
		const macField: FieldConfig<Row, 'mac'> = {
			orderField: 'mac',
			label: 'MAC',
			type: 'string',
			getValue: (r) => r.name
		};
		const items = ['0A:BA', '14:E9', '3A:C1', '7E:C7', null].map((name) => row({ name }));
		const sort: SortState = { field: 'mac', direction: 'asc' };

		expect(sortItems(items, [macField], sort, true).map((r) => r.name)).toEqual(
			items.map((r) => r.name)
		);
	});

	it('sorts an opted-in display field when the server orders a list it loaded whole', () => {
		// Every row is present, so the client order is the order of the whole list.
		const opted: FieldConfig<Row> = { ...nameField, sortable: true };
		const items = [row({ name: 'b' }), row({ name: 'a' })];

		expect(sortItems(items, [opted], asc, true).map((r) => r.name)).toEqual(['a', 'b']);
	});

	it('sorts nothing on a server-paginated page, display field or not', () => {
		// Sorting one page here would present that page's order as the list's.
		const opted: FieldConfig<Row> = { ...nameField, sortable: true };
		const items = [row({ name: 'b' }), row({ name: 'a' })];

		expect(sortItems(items, [opted], asc, true, true).map((r) => r.name)).toEqual(['b', 'a']);
		expect(sortItems(items, [opted], asc, false, true).map((r) => r.name)).toEqual(['b', 'a']);
	});

	it('signs the comparison by direction for non-null values', () => {
		const a = row({ name: 'a' });
		const b = row({ name: 'b' });

		expect(compareByField(a, b, nameField, 'asc')).toBeLessThan(0);
		expect(compareByField(a, b, nameField, 'desc')).toBeGreaterThan(0);
	});
});

describe('nextSortState', () => {
	it('flips direction when the same field is chosen again', () => {
		expect(nextSortState({ field: 'name', direction: 'asc' }, 'name')).toEqual({
			field: 'name',
			direction: 'desc'
		});
		expect(nextSortState({ field: 'name', direction: 'desc' }, 'name')).toEqual({
			field: 'name',
			direction: 'asc'
		});
	});

	it('starts a newly chosen field ascending', () => {
		expect(nextSortState({ field: 'name', direction: 'desc' }, 'seen')).toEqual({
			field: 'seen',
			direction: 'asc'
		});
	});

	it('always names exactly one field', () => {
		// This is what lets the table mark exactly one header aria-sort non-"none".
		const states: SortState[] = [
			{ field: null, direction: 'asc' },
			{ field: 'name', direction: 'asc' },
			{ field: 'seen', direction: 'desc' }
		];

		for (const state of states) {
			for (const key of ['name', 'seen', 'tags']) {
				const next = nextSortState(state, key);
				expect(next.field).toBe(key);
				expect(['asc', 'desc']).toContain(next.direction);
			}
		}
	});
});

describe('field capability selectors', () => {
	it('offers orderable fields and opted-in display fields for sorting', () => {
		const mixed: FieldConfig<Row, 'name'>[] = [
			{ orderField: 'name', label: 'Name', type: 'string' },
			{ key: 'plain', label: 'Plain', type: 'string' },
			{ key: 'opted', label: 'Opted', type: 'string', sortable: true }
		];

		expect(sortableFields(mixed).map((f) => f.label)).toEqual(['Name', 'Opted']);
	});

	it('groups string and boolean orderable fields by default but honours an opt-out', () => {
		const mixed: FieldConfig<Row, 'name' | 'seen' | 'active' | 'hidden'>[] = [
			{ orderField: 'name', label: 'Name', type: 'string' },
			{ orderField: 'seen', label: 'Seen', type: 'date' },
			{ orderField: 'active', label: 'Active', type: 'boolean' },
			{ orderField: 'hidden', label: 'Hidden', type: 'boolean', groupable: false },
			{ key: 'noGroup', label: 'NoGroup', type: 'string' },
			{ key: 'opted', label: 'Opted', type: 'string', groupable: true },
			{ key: 'optedBool', label: 'OptedBool', type: 'boolean', groupable: true }
		];

		// A date field is not groupable, and a display field must opt in.
		expect(groupableFields(mixed).map((f) => f.label)).toEqual([
			'Name',
			'Active',
			'Opted',
			'OptedBool'
		]);
	});

	it('never groups an array field, even one that opts in', () => {
		// Grouping would bucket rows by the stringified, comma-joined list.
		const arrays: FieldConfig<Row, 'tags'>[] = [
			{ orderField: 'tags', label: 'Ordered', type: 'array' },
			{ key: 'targets', label: 'Opted', type: 'array', groupable: true }
		];

		expect(groupableFields(arrays)).toEqual([]);
	});

	it('offers only orderable fields on a server-paginated list', () => {
		const mixed: FieldConfig<Row, 'name' | 'active'>[] = [
			{ orderField: 'name', label: 'Name', type: 'string' },
			{ orderField: 'active', label: 'Active', type: 'boolean' },
			{ key: 'opted', label: 'Opted', type: 'string', sortable: true, groupable: true }
		];

		expect(sortableFields(mixed, true).map((f) => f.label)).toEqual(['Name', 'Active']);
		expect(groupableFields(mixed, true).map((f) => f.label)).toEqual(['Name', 'Active']);
		expect(fieldsToColumns(mixed, true).map((c) => [c.id, c.sortable])).toEqual([
			['name', true],
			['active', true],
			['opted', false]
		]);
	});
});

describe('serverOrderViolations', () => {
	const opted: FieldConfig<Row, 'name'>[] = [
		{ orderField: 'name', label: 'Name', type: 'string' },
		{ key: 'sortOnly', label: 'SortOnly', type: 'string', sortable: true },
		{ key: 'groupOnly', label: 'GroupOnly', type: 'string', groupable: true },
		{ key: 'plain', label: 'Plain', type: 'string' }
	];

	it('names display fields opted into client-side ordering on a server-paginated list', () => {
		expect(serverOrderViolations(opted, true)).toEqual(['sortOnly', 'groupOnly']);
	});

	it('says nothing about a client-paginated list, where ordering here is correct', () => {
		expect(serverOrderViolations(opted, false)).toEqual([]);
	});

	it('reports exactly the fields the server-paginated selectors withhold', () => {
		// The guard and the selectors read one rule: a field the guard names is an
		// opted-in field the controls stopped offering. Were they to drift, the
		// guard would either cry wolf or miss a field silently dropped.
		const offered = new Set(
			[...sortableFields(opted, false), ...groupableFields(opted, false)].map((f) => f.label)
		);
		const offeredPaginated = new Set(
			[...sortableFields(opted, true), ...groupableFields(opted, true)].map((f) => f.label)
		);
		const withheld = opted
			.filter((f) => offered.has(f.label) && !offeredPaginated.has(f.label))
			.map((f) => ('key' in f ? f.key : f.orderField));

		expect(serverOrderViolations(opted, true)).toEqual(withheld);
	});
});

describe('boolean grouping', () => {
	const labels = { ungrouped: 'Ungrouped', yes: 'Yes', no: 'No' };
	const activeField: FieldConfig<Row, 'active'> = {
		orderField: 'active',
		label: 'Active',
		type: 'boolean',
		getValue: (r) => r.active
	};

	it('heads boolean groups with the yes/no labels', () => {
		const items = [
			row({ name: 'a', active: true }),
			row({ name: 'b', active: false }),
			row({ name: 'c', active: null })
		];

		const groups = groupItems(items, [activeField], 'active', labels, false);

		expect([...groups.keys()].sort()).toEqual(['No', 'Ungrouped', 'Yes']);
		expect(groups.get('Yes')!.map((r) => r.name)).toEqual(['a']);
	});

	it('finds each boolean group in server counts keyed "true" and "false"', () => {
		// count_by_group casts the grouped column to text, so a boolean group comes
		// back as "true"/"false" while its header reads Yes/No.
		const items = [row({ name: 'f', active: false }), row({ name: 't', active: true })];
		const offsets = computeGroupOffsets([
			{ value: 'false', count: 30 },
			{ value: 'true', count: 12 },
			{ value: null, count: 4 }
		]);

		const groups = groupItems(items, [activeField], 'active', labels, true);
		const yes = offsets.get(serverGroupKey(groups.get('Yes')!, [activeField], 'active'));
		const no = offsets.get(serverGroupKey(groups.get('No')!, [activeField], 'active'));

		expect(yes).toEqual({ start: 30, count: 12 });
		expect(no).toEqual({ start: 0, count: 30 });
	});
});
