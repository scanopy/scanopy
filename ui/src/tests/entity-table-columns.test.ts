import { describe, it, expect } from 'vitest';
import {
	fieldsToColumns,
	defaultColumnVisibility,
	defaultColumnOrder,
	reconcileColumnState,
	toggledColumnVisibility,
	visibleColumns,
	columnsWhileGrouped,
	moveColumn,
	moveColumnTo,
	movableIds,
	columnSections,
	movableGroups,
	moveGroup,
	moveGroupTo,
	buildTagColumn,
	columnWidth,
	MAX_COLUMN_WIDTH,
	MIN_COLUMN_WIDTH,
	TAG_COLUMN_ID,
	type ColumnState
} from '#lib/shared/components/data/table/columns.ts';
import { fieldGroups } from '#lib/shared/stores/metadata.ts';
import { defineFields, getFieldKey, type FieldConfig } from '#lib/shared/components/data/types.ts';
import {
	parseStoredState,
	serializeState
} from '#lib/shared/components/data/controls/dataControlsStorage.ts';

interface Row {
	name: string;
	site_id: string;
	created_at: string;
	port: number;
}

type RowOrderField = 'name' | 'site_id' | 'created_at';

function fields(): FieldConfig<Row, RowOrderField>[] {
	return defineFields<Row, RowOrderField>(
		{
			name: { label: 'Name', type: 'string', display: { primary: true, width: 240 } },
			site_id: { label: 'Site', type: 'string' },
			created_at: { label: 'Created', type: 'date', display: { hiddenByDefault: true } }
		},
		[
			{ key: 'description', label: 'Description', type: 'string' },
			{ key: 'labels', label: 'Labels', type: 'array' },
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
		expect(byId.get('labels')!.sortable).toBe(false);
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
		expect(byId.get('site_id')!.primary).toBe(false);
		expect(byId.get('site_id')!.align).toBe('left');
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

describe('toggledColumnVisibility', () => {
	it('stores only the columns moved off their default, so a changed default still applies', () => {
		const columns = fieldsToColumns(fields());
		const shown = reconcileColumnState(columns, { visibility: {} }).visibility;

		const stored = toggledColumnVisibility(columns, shown, 'name');
		expect(stored).toEqual({ name: false });

		// Toggled back, nothing is left deciding the column.
		const back = reconcileColumnState(columns, { visibility: stored }).visibility;
		expect(toggledColumnVisibility(columns, back, 'name')).toEqual({});
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
		const withoutSite = columns.map((c) => c.id).filter((id) => id !== 'site_id');

		const state = reconcileColumnState(columns, { visibility: {}, order: withoutSite });
		const declaredIndex = columns.findIndex((c) => c.id === 'site_id');

		expect(state.order).toContain('site_id');
		expect(state.order.indexOf('site_id')).toBe(declaredIndex);
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

describe('columnsWhileGrouped', () => {
	it('leaves out the grouped column until the table is ungrouped', () => {
		const columns = fieldsToColumns(fields());
		const ids = (cols: typeof columns) => cols.map((c) => c.id);

		expect(ids(columnsWhileGrouped(columns, 'site_id'))).toEqual(
			ids(columns).filter((id) => id !== 'site_id')
		);
		expect(ids(columnsWhileGrouped(columns, null))).toEqual(ids(columns));
	});

	it('keeps the primary column, which identifies the row', () => {
		const columns = fieldsToColumns(fields());
		expect(columnsWhileGrouped(columns, 'name').map((c) => c.id)).toContain('name');
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
			order: ['progress', 'site_id', 'name', 'description', 'labels', 'created_at']
		});

		expect(state.order[0]).toBe('name');
		expect(state.order[state.order.length - 1]).toBe('progress');
		expect(state.order.slice(1, -1)).toEqual(['site_id', 'description', 'labels', 'created_at']);
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

		const after = moveColumnTo(state.order, columns, 'site_id', 'labels', 'after');
		expect(after.indexOf('site_id')).toBe(after.indexOf('labels') + 1);

		const before = moveColumnTo(after, columns, 'site_id', 'description', 'before');
		expect(before.indexOf('site_id')).toBe(before.indexOf('description') - 1);
	});

	it('refuses a drag that involves a pinned column', () => {
		// Dropping before the primary column would put a column on its far side.
		const { columns, state } = setup();

		expect(moveColumnTo(state.order, columns, 'site_id', 'name', 'before')).toBe(state.order);
		expect(moveColumnTo(state.order, columns, 'progress', 'labels', 'before')).toBe(state.order);
	});

	it('places a new column after its default predecessor in a rearranged order', () => {
		// The user moved created_at to the end; description is declared right after
		// it, so it arrives beside created_at rather than at its default index.
		const { columns } = setup();
		const rearranged = ['name', 'site_id', 'labels', 'created_at', 'progress'];

		const state = reconcileColumnState(columns, { visibility: {}, order: rearranged });

		expect(state.order).toEqual([
			'name',
			'site_id',
			'labels',
			'created_at',
			'description',
			'progress'
		]);
	});

	it('survives a save and reload', () => {
		const { columns, state } = setup();
		const moved = moveColumn(state.order, columns, 'site_id', 1);

		const reloaded = parseStoredState(
			serializeState({
				searchQuery: '',
				filterState: {},
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
				currentPage: 1,
				columnSizing
			})
		)!.columnSizing;
	}

	it('survives a save and reload', () => {
		const columns = fieldsToColumns(fields());
		const sizing = reload({ name: 180, site_id: 96 });

		const state = reconcileColumnState(columns, { sizing });

		expect(state.sizing).toEqual({ name: 180, site_id: 96 });
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
		const state = reconcileColumnState(columns, { sizing: { site_id: 140 } });
		const byId = new Map(columns.map((c) => [c.id, c]));

		expect(columnWidth(byId.get('name')!, state.sizing)).toBe(240);
		expect(columnWidth(byId.get('description')!, state.sizing)).toBeUndefined();
	});

	it('keeps stored widths inside the resize bounds', () => {
		const columns = fieldsToColumns(fields());

		const state = reconcileColumnState(columns, {
			sizing: { name: 1, site_id: 99999, description: Number.NaN }
		});

		expect(state.sizing).toEqual({ name: MIN_COLUMN_WIDTH, site_id: MAX_COLUMN_WIDTH });
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

describe('column headings', () => {
	// `relationships` comes before `status` in field-groups.json; `name` is primary, so pinned.
	const grouped = (): FieldConfig<Row>[] => [
		{ key: 'name', label: 'Name', type: 'string', display: { primary: true, group: 'identity' } },
		{ key: 'status', label: 'Status', type: 'string', display: { group: 'status' } },
		{ key: 'port', label: 'Port', type: 'string' },
		{ key: 'site_id', label: 'Site', type: 'string', display: { group: 'relationships' } },
		{ key: 'hostname', label: 'Hostname', type: 'string', display: { group: 'identity' } },
		{ key: 'env', label: 'Environment', type: 'string', display: { group: 'status' } },
		{
			key: 'rack',
			label: 'Rack',
			type: 'string',
			display: { section: { id: 'tag_groups', label: 'Tag Groups' } }
		}
	];
	const columns = fieldsToColumns(grouped());
	const TAG_GROUPS = 'section:tag_groups';
	const reconcile = (stored?: Partial<ColumnState>) => reconcileColumnState(columns, stored);

	it('orders by default with unheaded columns first, field groups in order, sections last', () => {
		const { order, groupOrder } = reconcile();

		expect(groupOrder).toEqual(['identity', 'relationships', 'status', TAG_GROUPS]);
		expect(order).toEqual(['name', 'port', 'hostname', 'site_id', 'status', 'env', 'rack']);
		const byId = new Map(columns.map((c) => [c.id, c]));
		const sections = columnSections(order.map((id) => byId.get(id)!));
		expect(sections.map((s) => [s.key, s.heading])).toEqual([
			[null, null],
			['identity', fieldGroups.getName('identity')],
			['relationships', fieldGroups.getName('relationships')],
			['status', fieldGroups.getName('status')],
			[TAG_GROUPS, 'Tag Groups']
		]);
	});

	it('sorts a stored order by group order first, keeping its order within each group', () => {
		// An order saved before headings existed interleaves them.
		const stored = ['name', 'env', 'port', 'rack', 'site_id', 'status', 'hostname'];

		expect(reconcile({ order: stored }).order).toEqual([
			'name',
			'port',
			'hostname',
			'site_id',
			'env',
			'status',
			'rack'
		]);
	});

	it('moves a column only among the columns of its heading', () => {
		const { order } = reconcile();
		const status = columnSections(columns).find((s) => s.key === 'status')!.columns;

		expect(moveColumn(order, status, 'status', 1)).toEqual([
			'name',
			'port',
			'hostname',
			'site_id',
			'env',
			'status',
			'rack'
		]);
		expect(moveColumn(order, status, 'status', -1)).toBe(order);
	});

	it('moves a heading by step or by drag, and its columns follow', () => {
		const { order, groupOrder } = reconcile();
		const movable = movableGroups(groupOrder, columns);

		const up = moveGroup(groupOrder, movable, 'status', -1);
		expect(up).toEqual(['identity', 'status', 'relationships', TAG_GROUPS]);
		expect(reconcile({ order, groupOrder: up }).order).toEqual([
			'name',
			'port',
			'hostname',
			'status',
			'env',
			'site_id',
			'rack'
		]);

		const dragged = moveGroupTo(groupOrder, TAG_GROUPS, 'identity', 'before');
		expect(reconcile({ order, groupOrder: dragged }).order).toEqual([
			'name',
			'port',
			'rack',
			'hostname',
			'site_id',
			'status',
			'env'
		]);
	});

	it('keeps the primary column first when its heading moves', () => {
		const { order, groupOrder } = reconcile();
		const down = moveGroup(groupOrder, movableGroups(groupOrder, columns), 'identity', 1);

		expect(reconcile({ order, groupOrder: down }).order).toEqual([
			'name',
			'port',
			'site_id',
			'hostname',
			'status',
			'env',
			'rack'
		]);
	});

	it('leaves the heading order alone past either end', () => {
		const { groupOrder } = reconcile();
		const movable = movableGroups(groupOrder, columns);

		expect(moveGroup(groupOrder, movable, 'identity', -1)).toBe(groupOrder);
		expect(moveGroup(groupOrder, movable, TAG_GROUPS, 1)).toBe(groupOrder);
		expect(moveGroupTo(groupOrder, 'status', 'status', 'after')).toBe(groupOrder);
	});

	it('places a heading the stored order lacks after its default predecessor', () => {
		const { groupOrder } = reconcile({ groupOrder: ['relationships', 'identity', 'gone'] });

		expect(groupOrder).toEqual(['relationships', 'status', TAG_GROUPS, 'identity']);
	});

	it('keeps the heading order through a save and reload', () => {
		const { groupOrder } = reconcile();
		const moved = moveGroup(groupOrder, movableGroups(groupOrder, columns), TAG_GROUPS, -1);

		const reloaded = parseStoredState(
			serializeState({
				searchQuery: '',
				filterState: {},
				currentPage: 1,
				columnGroupOrder: moved
			})
		);

		expect(reconcile({ groupOrder: reloaded!.columnGroupOrder }).groupOrder).toEqual(moved);
	});
});
