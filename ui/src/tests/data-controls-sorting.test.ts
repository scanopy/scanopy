import { describe, it, expect } from 'vitest';
import {
	nextSortState,
	sortableFields,
	groupableFields,
	defaultOrderingViolations,
	type SortState
} from '#lib/shared/components/data/controls/sorting.ts';
import {
	groupItems,
	computeGroupOffsets,
	serverGroupKey
} from '#lib/shared/components/data/controls/grouping.ts';
import { fieldsToColumns } from '#lib/shared/components/data/table/columns.ts';
import type { FieldConfig } from '#lib/shared/components/data/types.ts';

interface Row {
	name: string | null;
	active: boolean | null;
}

function row(partial: Partial<Row>): Row {
	return { name: null, active: null, ...partial };
}

const nameField: FieldConfig<Row> = {
	key: 'name',
	label: 'Name',
	type: 'string',
	getValue: (r) => r.name
};

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
	it('sorts and groups only fields the server orders', () => {
		const mixed: FieldConfig<Row, 'name' | 'active'>[] = [
			{ orderField: 'name', label: 'Name', type: 'string' },
			{ orderField: 'active', label: 'Active', type: 'boolean' },
			{ key: 'plain', label: 'Plain', type: 'string' }
		];

		expect(sortableFields(mixed).map((f) => f.label)).toEqual(['Name', 'Active']);
		expect(groupableFields(mixed).map((f) => f.label)).toEqual(['Name', 'Active']);
		expect(fieldsToColumns(mixed).map((c) => [c.id, c.sortable])).toEqual([
			['name', true],
			['active', true],
			['plain', false]
		]);
	});

	it('groups string and boolean orderable fields by default but honours an opt-out', () => {
		const mixed: FieldConfig<Row, 'name' | 'seen' | 'active' | 'hidden'>[] = [
			{ orderField: 'name', label: 'Name', type: 'string' },
			{ orderField: 'seen', label: 'Seen', type: 'date' },
			{ orderField: 'active', label: 'Active', type: 'boolean' },
			{ orderField: 'hidden', label: 'Hidden', type: 'boolean', groupable: false }
		];

		// A date field is not groupable.
		expect(groupableFields(mixed).map((f) => f.label)).toEqual(['Name', 'Active']);
	});

	it('never groups an array field', () => {
		// Grouping would bucket rows by the stringified, comma-joined list.
		const arrays: FieldConfig<Row, 'tags'>[] = [
			{ orderField: 'tags', label: 'Ordered', type: 'array' }
		];

		expect(groupableFields(arrays)).toEqual([]);
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

		const groups = groupItems(items, [activeField], 'active', labels);

		expect([...groups.keys()]).toEqual(['Yes', 'No', 'Ungrouped']);
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

		const groups = groupItems(items, [activeField], 'active', labels);
		const yes = offsets.get(serverGroupKey(groups.get('Yes')!, [activeField], 'active'));
		const no = offsets.get(serverGroupKey(groups.get('No')!, [activeField], 'active'));

		expect(yes).toEqual({ start: 30, count: 12 });
		expect(no).toEqual({ start: 0, count: 30 });
	});
});

describe('defaultOrderingViolations', () => {
	const site: FieldConfig<Row, 'site'> = {
		orderField: 'site',
		label: 'Site',
		type: 'string',
		getValue: (r) => r.name
	};
	const hiddenSite: FieldConfig<Row, 'site'> = { ...site, display: { hiddenByDefault: true } };

	it('accepts a default on a groupable, sortable column that renders by default', () => {
		const defaults = { group: 'site', sort: { field: 'site', direction: 'asc' as const } };
		expect(defaultOrderingViolations([site], defaults)).toEqual([]);
	});

	it('flags a default on a column hidden until the user shows it', () => {
		// The list orders by rendered columns only, so the tab would open ungrouped.
		const defaults = { group: 'site', sort: { field: 'site', direction: 'asc' as const } };
		expect(defaultOrderingViolations([hiddenSite], defaults)).toHaveLength(2);
	});

	it('flags a default on a missing field, or one the server cannot order', () => {
		expect(defaultOrderingViolations([nameField], { group: 'name' })).toHaveLength(1);
		expect(
			defaultOrderingViolations([site], { sort: { field: 'gone', direction: 'asc' } })
		).toHaveLength(1);
	});
});
