import { describe, it, expect, vi } from 'vitest';
import {
	columnControls,
	columnControlState,
	dropHiddenColumnState,
	renderedFields,
	type ColumnBoundState
} from '$lib/shared/components/data/controls/headerControls';
import {
	blankFilterState,
	toggleBoolean,
	toggleValue,
	type FilterState
} from '$lib/shared/components/data/controls/filtering';
import {
	createFilterActions,
	type FilterActionsContext
} from '$lib/shared/components/data/controls/filterActions';
import type { FieldConfig } from '$lib/shared/components/data/types';

type Row = Record<string, unknown>;

const status: FieldConfig<Row> = {
	key: 'status',
	label: 'Status',
	type: 'string',
	filterable: true,
	groupable: true
};
const category: FieldConfig<Row> = {
	key: 'category',
	label: 'Category',
	type: 'string',
	filterable: true,
	serverFiltered: true,
	filterMode: 'exclude'
};
const hidden: FieldConfig<Row> = {
	key: 'hidden',
	label: 'Hidden',
	type: 'boolean',
	filterable: true,
	serverFiltered: true
};
const site: FieldConfig<Row> = { orderField: 'site_id', label: 'Site', type: 'string' };
const mode: FieldConfig<Row> = { key: 'mode', label: 'Mode', type: 'string', groupable: true };
const description: FieldConfig<Row> = { key: 'description', label: 'Desc', type: 'string' };
const lastSeen: FieldConfig<Row> = {
	key: 'last_seen_at',
	label: 'Last seen',
	type: 'date',
	staleFilter: true
};
const tags: FieldConfig<Row> = { key: 'tags', label: 'Tags', type: 'array', filterable: true };

const FIELDS = [status, category, hidden, site, mode, description, lastSeen, tags];

function filtersWith(mutate: (state: FilterState) => FilterState | null): FilterState {
	return mutate(blankFilterState(FIELDS, false)) ?? blankFilterState(FIELDS, false);
}

describe('columnControls', () => {
	it('offers filter and group from the field flags', () => {
		expect(columnControls('status', FIELDS, false, false)).toMatchObject({
			filter: true,
			group: true
		});
		expect(columnControls('category', FIELDS, false, false)).toMatchObject({
			filter: true,
			group: false
		});
		expect(columnControls('site_id', FIELDS, false, false)).toMatchObject({
			filter: false,
			group: true
		});
	});

	it('gives a column with no filter, group or stale toggle no control', () => {
		expect(columnControls('description', FIELDS, false, false)).toBeNull();
		expect(columnControls('missing', FIELDS, false, false)).toBeNull();
	});

	it('drops client-side grouping on a server-paginated list, keeping orderable grouping', () => {
		expect(columnControls('mode', FIELDS, true, false)).toBeNull();
		expect(columnControls('site_id', FIELDS, true, false)?.group).toBe(true);
	});

	it("resolves the appended tags column to the tab's tags field", () => {
		expect(columnControls('tags', FIELDS, false, false)?.field).toBe(tags);
	});

	it('puts the stale toggle on its column only when the parent applies staleness', () => {
		expect(columnControls('last_seen_at', FIELDS, true, true)?.stale).toBe(true);
		expect(columnControls('last_seen_at', FIELDS, true, false)).toBeNull();
	});
});

describe('columnControlState', () => {
	const state = (columnId: string, filterState: FilterState, staleOnly = false, group = null) =>
		columnControlState(
			columnControls(columnId, FIELDS, false, true)!,
			filterState,
			staleOnly,
			group
		);

	it('is inactive with nothing set', () => {
		expect(state('status', blankFilterState(FIELDS, false))).toEqual({
			filtered: false,
			grouped: false
		});
	});

	it('shows a value filter, including an exclude-mode one', () => {
		expect(
			state(
				'status',
				filtersWith((s) => toggleValue(s, 'status', 'up'))
			).filtered
		).toBe(true);
		expect(
			state(
				'category',
				filtersWith((s) => toggleValue(s, 'category', 'Media'))
			).filtered
		).toBe(true);
	});

	it('shows a boolean filter once a box is unchecked', () => {
		const filterState = filtersWith(
			(s) => toggleBoolean(s, 'hidden', 'showTrue', false)?.state ?? null
		);
		expect(state('hidden', filterState).filtered).toBe(true);
	});

	it('shows staleness on the column that carries it', () => {
		expect(state('last_seen_at', blankFilterState(FIELDS, false), true).filtered).toBe(true);
	});

	it('shows grouping on the grouped column only', () => {
		const blank = blankFilterState(FIELDS, false);
		expect(
			columnControlState(columnControls('status', FIELDS, false, false)!, blank, false, 'status')
		).toEqual({ filtered: false, grouped: true });
		expect(
			columnControlState(columnControls('mode', FIELDS, false, false)!, blank, false, 'status')
				.grouped
		).toBe(false);
	});

	it("does not light one column for another column's filter", () => {
		expect(
			state(
				'status',
				filtersWith((s) => toggleValue(s, 'category', 'Media'))
			).filtered
		).toBe(false);
	});
});

describe('dropHiddenColumnState', () => {
	function bound(overrides: Partial<ColumnBoundState> = {}): ColumnBoundState {
		return {
			filterState: blankFilterState(FIELDS, false),
			staleOnly: false,
			sortState: { field: null, direction: 'asc' },
			groupField: null,
			...overrides
		};
	}
	const allIds = new Set(FIELDS.map((f) => ('key' in f ? f.key : f.orderField)));
	const without = (...ids: string[]) => new Set([...allIds].filter((id) => !ids.includes(id)));

	it('changes nothing while every column renders', () => {
		const state = bound({
			filterState: filtersWith((s) => toggleValue(s, 'status', 'up')),
			sortState: { field: 'status', direction: 'desc' },
			groupField: 'status',
			staleOnly: true
		});
		expect(dropHiddenColumnState(state, FIELDS, allIds)).toBeNull();
	});

	it("clears a hidden column's filter, sort and group, and leaves visible columns alone", () => {
		let filterState = filtersWith((s) => toggleValue(s, 'status', 'up'));
		filterState = toggleValue(filterState, 'tags', 't1')!;
		const next = dropHiddenColumnState(
			bound({
				filterState,
				sortState: { field: 'status', direction: 'desc' },
				groupField: 'status'
			}),
			FIELDS,
			without('status')
		)!;

		expect(next.filterState.status.values.size).toBe(0);
		expect(next.filterState.tags.values.has('t1')).toBe(true);
		expect(next.sortState).toEqual({ field: null, direction: 'asc' });
		expect(next.groupField).toBeNull();
		expect(next.serverKeys).toEqual([]);
	});

	it('keeps sort and group on visible columns while clearing a hidden filter', () => {
		const next = dropHiddenColumnState(
			bound({
				filterState: filtersWith((s) => toggleValue(s, 'category', 'Media')),
				sortState: { field: 'site_id', direction: 'desc' },
				groupField: 'status'
			}),
			FIELDS,
			without('category')
		)!;

		expect(next.sortState).toEqual({ field: 'site_id', direction: 'desc' });
		expect(next.groupField).toBe('status');
	});

	it('names the cleared server-side filters so the parent can be told', () => {
		let filterState = filtersWith((s) => toggleValue(s, 'category', 'Media'));
		filterState = toggleBoolean(filterState, 'hidden', 'showTrue', true)!.state;
		const next = dropHiddenColumnState(
			bound({ filterState }),
			FIELDS,
			without('category', 'hidden')
		)!;

		expect(next.serverKeys.sort()).toEqual(['category', 'hidden']);
		expect(next.filterState.hidden.showTrue).toBe(true);
	});

	it('clears staleness when its column is hidden', () => {
		const next = dropHiddenColumnState(
			bound({ staleOnly: true }),
			FIELDS,
			without('last_seen_at')
		)!;
		expect(next).toMatchObject({ staleOnly: false, staleCleared: true });
	});

	it('offers only rendered fields to the header controls', () => {
		expect(renderedFields(FIELDS, without('status', 'mode'))).not.toContain(status);
		expect(renderedFields(FIELDS, without('status', 'mode'))).toContain(site);
	});
});

describe('createFilterActions', () => {
	function setup(onFilterChange: FilterActionsContext<Row>['onFilterChange']) {
		const ctx: FilterActionsContext<Row> = {
			fields: FIELDS,
			filterState: blankFilterState(FIELDS, false),
			staleOnly: false,
			onFilterChange,
			onStaleFilterChange: vi.fn(),
			resetToFirstPage: vi.fn()
		};
		return { ctx, actions: createFilterActions(ctx) };
	}

	it("clears one column's filter and leaves the others", () => {
		const { ctx, actions } = setup(null);
		actions.toggleString('status', 'up');
		actions.toggleTag('t1');
		actions.clearField('status');

		expect(ctx.filterState.status.values.size).toBe(0);
		expect(ctx.filterState.tags.values.has('t1')).toBe(true);
	});

	it('tells the parent a cleared server filter means no constraint', () => {
		const onFilterChange = vi.fn();
		const { ctx, actions } = setup(onFilterChange);
		actions.toggleString('category', 'Media');
		actions.toggleBoolean('hidden', 'showFalse');
		onFilterChange.mockClear();

		actions.clearField('category');
		actions.clearField('hidden');

		expect(onFilterChange).toHaveBeenCalledWith('category', []);
		// Both boxes, not an empty selection, which a parent would read as "show neither".
		expect(onFilterChange).toHaveBeenCalledWith('hidden', ['true', 'false']);
		expect(ctx.resetToFirstPage).toHaveBeenCalled();
	});

	it('does not notify the parent for a client-side filter', () => {
		const onFilterChange = vi.fn();
		const { actions } = setup(onFilterChange);
		actions.toggleString('status', 'up');
		actions.clearField('status');
		expect(onFilterChange).not.toHaveBeenCalled();
	});

	it("clears staleness from its column's clear", () => {
		const { ctx, actions } = setup(null);
		actions.toggleStale();
		actions.clearField('last_seen_at');
		expect(ctx.staleOnly).toBe(false);
		expect(ctx.onStaleFilterChange).toHaveBeenLastCalledWith(null);
	});
});
