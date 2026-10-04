import { describe, it, expect } from 'vitest';
import {
	fieldsToColumns,
	defaultColumnVisibility,
	defaultColumnOrder,
	reconcileColumnState,
	visibleColumns,
	moveColumn,
	moveColumnTo,
	movableIds,
	buildTagColumn,
	columnWidth,
	MAX_COLUMN_WIDTH,
	MIN_COLUMN_WIDTH,
	TAG_COLUMN_ID,
	type ColumnState
} from '$lib/shared/components/data/table/columns';
import { defineFields, getFieldKey, type FieldConfig } from '$lib/shared/components/data/types';
import {
	parseStoredState,
	serializeState
} from '$lib/shared/components/data/controls/dataControlsStorage';

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

describe('column reordering', () => {
	// A trailing column alongside the primary one, so both pinned ends are exercised.
	function reorderFields(): FieldConfig<Row, RowOrderField>[] {
		return [
			...fields(),
			{ key: 'progress', label: 'Progress', type: 'string', display: { trailing: true } }
		];
	}

	function setup(stored?: Partial<ColumnState>) {
		const columns = fieldsToColumns(reorderFields());
		return { columns, state: reconcileColumnState(columns, stored) };
	}

	it('keeps the primary column first and trailing columns last whatever was stored', () => {
		// A hand-edited or older blob must not be able to unpin either end.
		const { state } = setup({
			visibility: {},
			order: ['progress', 'network_id', 'name', 'description', 'labels', 'created_at']
		});

		expect(state.order[0]).toBe('name');
		expect(state.order[state.order.length - 1]).toBe('progress');
		expect(state.order.slice(1, -1)).toEqual(['network_id', 'description', 'labels', 'created_at']);
	});

	it('moves a column one step among the movable columns', () => {
		const { columns, state } = setup();
		const movable = movableIds(state.order, columns);

		const down = moveColumn(state.order, columns, movable[0], 1);
		expect(movableIds(down, columns).slice(0, 2)).toEqual([movable[1], movable[0]]);

		const up = moveColumn(down, columns, movable[0], -1);
		expect(up).toEqual(state.order);
	});

	it('leaves the order alone for a pinned column or a step past either end', () => {
		const { columns, state } = setup();
		const movable = movableIds(state.order, columns);

		expect(moveColumn(state.order, columns, 'name', 1)).toBe(state.order);
		expect(moveColumn(state.order, columns, 'progress', -1)).toBe(state.order);
		expect(moveColumn(state.order, columns, movable[0], -1)).toBe(state.order);
		expect(moveColumn(state.order, columns, movable[movable.length - 1], 1)).toBe(state.order);
	});

	it('drops a dragged column before or after its target', () => {
		const { columns, state } = setup();

		const after = moveColumnTo(state.order, columns, 'network_id', 'labels', 'after');
		expect(after.indexOf('network_id')).toBe(after.indexOf('labels') + 1);

		const before = moveColumnTo(after, columns, 'network_id', 'description', 'before');
		expect(before.indexOf('network_id')).toBe(before.indexOf('description') - 1);
	});

	it('refuses a drag that involves a pinned column', () => {
		// Dropping before the primary column would put a column on its far side.
		const { columns, state } = setup();

		expect(moveColumnTo(state.order, columns, 'network_id', 'name', 'before')).toBe(state.order);
		expect(moveColumnTo(state.order, columns, 'progress', 'labels', 'before')).toBe(state.order);
	});

	it('places a new column after its default predecessor in a rearranged order', () => {
		// The user moved created_at to the end; description is declared right after
		// it, so it arrives beside created_at rather than at its default index.
		const { columns } = setup();
		const rearranged = ['name', 'network_id', 'labels', 'created_at', 'progress'];

		const state = reconcileColumnState(columns, { visibility: {}, order: rearranged });

		expect(state.order).toEqual([
			'name',
			'network_id',
			'labels',
			'created_at',
			'description',
			'progress'
		]);
	});

	it('survives a save and reload', () => {
		const { columns, state } = setup();
		const moved = moveColumn(state.order, columns, 'network_id', 1);

		const reloaded = parseStoredState(
			serializeState({
				searchQuery: '',
				filterState: {},
				sortState: { field: null, direction: 'asc' },
				selectedGroupField: null,
				showFilters: false,
				viewMode: 'table',
				currentPage: 1,
				columnOrder: moved
			})
		);

		expect(
			reconcileColumnState(columns, { visibility: {}, order: reloaded!.columnOrder }).order
		).toEqual(moved);
	});
});

describe('column widths', () => {
	function reload(columnSizing: Record<string, number>) {
		return parseStoredState(
			serializeState({
				searchQuery: '',
				filterState: {},
				sortState: { field: null, direction: 'asc' },
				selectedGroupField: null,
				showFilters: false,
				viewMode: 'table',
				currentPage: 1,
				columnSizing
			})
		)!.columnSizing;
	}

	it('survives a save and reload', () => {
		const columns = fieldsToColumns(fields());
		const sizing = reload({ name: 180, network_id: 96 });

		const state = reconcileColumnState(columns, { sizing });

		expect(state.sizing).toEqual({ name: 180, network_id: 96 });
		const byId = new Map(columns.map((c) => [c.id, c]));
		expect(columnWidth(byId.get('name')!, state.sizing)).toBe(180);
	});

	it('drops the width of a column that no longer exists', () => {
		const columns = fieldsToColumns(fields());

		const state = reconcileColumnState(columns, { sizing: reload({ removed_field: 120 }) });

		expect(state.sizing).toEqual({});
	});

	it('gives a column the user never resized its declared width, or none', () => {
		// A newly added field has no stored entry, so it starts where the tab declared.
		const columns = fieldsToColumns(fields());
		const state = reconcileColumnState(columns, { sizing: { network_id: 140 } });
		const byId = new Map(columns.map((c) => [c.id, c]));

		expect(columnWidth(byId.get('name')!, state.sizing)).toBe(240);
		expect(columnWidth(byId.get('description')!, state.sizing)).toBeUndefined();
	});

	it('keeps stored widths inside the resize bounds', () => {
		const columns = fieldsToColumns(fields());

		const state = reconcileColumnState(columns, {
			sizing: { name: 1, network_id: 99999, description: Number.NaN }
		});

		expect(state.sizing).toEqual({ name: MIN_COLUMN_WIDTH, network_id: MAX_COLUMN_WIDTH });
	});

	it('keeps the width of the tags column the list appends', () => {
		const columns = fieldsToColumns(fields());
		const tagColumn = buildTagColumn<Row>('Tags', () => [], undefined);
		const stored = { sizing: { [TAG_COLUMN_ID]: 150 } };

		expect(reconcileColumnState(columns, stored, [tagColumn]).sizing).toEqual({
			[TAG_COLUMN_ID]: 150
		});
		// A list without tags has no such column, so the entry is stale there.
		expect(reconcileColumnState(columns, stored).sizing).toEqual({});
	});
});
