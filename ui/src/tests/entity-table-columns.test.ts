import { describe, it, expect } from 'vitest';
import {
	fieldsToColumns,
	defaultColumnVisibility,
	defaultColumnOrder,
	reconcileColumnState,
	visibleColumns,
	TAG_COLUMN_ID,
	type ColumnState
} from '$lib/shared/components/data/table/columns';
import { defineFields, getFieldKey, type FieldConfig } from '$lib/shared/components/data/types';

interface Row {
	name: string;
	network_id: string;
	created_at: string;
	port: number;
}

type RowOrderField = 'name' | 'network_id' | 'created_at';

function fields(): FieldConfig<Row, RowOrderField>[] {
	return defineFields<Row, RowOrderField>(
		{
			name: { label: 'Name', type: 'string', display: { primary: true, width: 240 } },
			network_id: { label: 'Network', type: 'string' },
			created_at: { label: 'Created', type: 'date', display: { hiddenByDefault: true } }
		},
		[
			{ key: 'description', label: 'Description', type: 'string' },
			{ key: 'labels', label: 'Labels', type: 'array', sortable: true },
			// Reserved: the list appends its own editable tags column, so a `tags`
			// field stays a filter/search input and never becomes a column here.
			{ key: 'tags', label: 'Tags', type: 'array', filterable: true },
			// Filter-only: drives a filter, has no value worth a column.
			{ key: 'port', label: 'Port', type: 'string', filterable: true, display: { hidden: true } }
		]
	);
}

describe('fieldsToColumns', () => {
	it('uses the field key as the column id', () => {
		// The load-bearing invariant: a header click dispatches this id as the sort
		// field, and for orderable fields that id IS the backend order-field value.
		const all = fields();

		for (const column of fieldsToColumns(all)) {
			expect(column.id).toBe(getFieldKey(column.field));
		}
	});

	it('marks orderable fields sortable and plain display fields not', () => {
		// Mirrors the sort dropdown's rule, so headers and dropdown cannot drift.
		const byId = new Map(fieldsToColumns(fields()).map((c) => [c.id, c]));

		expect(byId.get('name')!.sortable).toBe(true);
		expect(byId.get('created_at')!.sortable).toBe(true);
		expect(byId.get('labels')!.sortable).toBe(true);
		expect(byId.get('description')!.sortable).toBe(false);
	});

	it('drops fields marked hidden entirely', () => {
		const ids = fieldsToColumns(fields()).map((c) => c.id);

		expect(ids).not.toContain('port');
	});

	it('reserves the tags id for the column the list appends itself', () => {
		// Otherwise a tab declaring a `tags` field would render a second,
		// read-only tags column beside the editable one.
		const ids = fieldsToColumns(fields()).map((c) => c.id);

		expect(ids).not.toContain(TAG_COLUMN_ID);
	});

	it('carries per-column presentation through', () => {
		const byId = new Map(fieldsToColumns(fields()).map((c) => [c.id, c]));

		expect(byId.get('name')!.primary).toBe(true);
		expect(byId.get('name')!.width).toBe(240);
		expect(byId.get('network_id')!.primary).toBe(false);
		expect(byId.get('network_id')!.align).toBe('left');
	});
});

describe('defaults', () => {
	it('hides only the fields that opted out of first paint', () => {
		const visibility = defaultColumnVisibility(fieldsToColumns(fields()));

		expect(visibility.created_at).toBe(false);
		expect(visibility.name).toBe(true);
		expect(visibility.description).toBe(true);
	});

	it('produces a default order that is a permutation of the column ids', () => {
		const columns = fieldsToColumns(fields());
		const order = defaultColumnOrder(columns);

		expect(new Set(order)).toEqual(new Set(columns.map((c) => c.id)));
		expect(order).toHaveLength(columns.length);
	});

	it('keeps a hiddenByDefault column present but off, not absent', () => {
		// Present-but-off is what lets the column menu offer it back.
		const columns = fieldsToColumns(fields());

		expect(defaultColumnOrder(columns)).toContain('created_at');
		expect(defaultColumnVisibility(columns).created_at).toBe(false);
	});
});

describe('reconcileColumnState', () => {
	it('drops stored entries for fields that no longer exist', () => {
		// Renaming a field must not leave a stale entry hiding a real column.
		const columns = fieldsToColumns(fields());
		const state = reconcileColumnState(columns, {
			visibility: { name: false, removed_field: false },
			order: ['removed_field', 'name']
		});

		expect(Object.keys(state.visibility).sort()).toEqual(columns.map((c) => c.id).sort());
		expect(state.order).not.toContain('removed_field');
	});

	it('honours a stored visibility choice over the default', () => {
		const columns = fieldsToColumns(fields());
		const state = reconcileColumnState(columns, { visibility: { created_at: true }, order: [] });

		expect(state.visibility.created_at).toBe(true);
	});

	it('splices a newly added field in at its declared position', () => {
		// Appending instead would push a mid-list addition past the date columns.
		const columns = fieldsToColumns(fields());
		const withoutNetwork = columns.map((c) => c.id).filter((id) => id !== 'network_id');

		const state = reconcileColumnState(columns, { visibility: {}, order: withoutNetwork });
		const declaredIndex = columns.findIndex((c) => c.id === 'network_id');

		expect(state.order).toContain('network_id');
		expect(state.order.indexOf('network_id')).toBe(declaredIndex);
	});

	it('always covers exactly the current columns', () => {
		const columns = fieldsToColumns(fields());
		const inputs: (Partial<ColumnState> | undefined)[] = [
			undefined,
			{ visibility: {}, order: [] },
			{ visibility: { name: false }, order: ['tags'] },
			{ order: ['nope'] }
		];

		for (const stored of inputs) {
			const state = reconcileColumnState(columns, stored);
			expect(new Set(state.order)).toEqual(new Set(columns.map((c) => c.id)));
		}
	});
});

describe('visibleColumns', () => {
	it('returns columns in the persisted order', () => {
		const columns = fieldsToColumns(fields());
		const reversed = defaultColumnOrder(columns).slice().reverse();

		const visible = visibleColumns(columns, {
			visibility: defaultColumnVisibility(columns),
			order: reversed
		});

		expect(visible.map((c) => c.id)).toEqual(reversed.filter((id) => id !== 'created_at'));
	});

	it('omits columns switched off', () => {
		const columns = fieldsToColumns(fields());
		const state = reconcileColumnState(columns, undefined);
		state.visibility.name = false;

		expect(visibleColumns(columns, state).map((c) => c.id)).not.toContain('name');
	});
});
