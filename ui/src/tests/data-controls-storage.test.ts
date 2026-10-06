import { describe, it, expect } from 'vitest';
import {
	parseStoredState,
	resolveOrdering,
	serializeState,
	STORED_STATE_VERSION,
	type StoredState
} from '$lib/shared/components/data/controls/dataControlsStorage';

function baseState(overrides: Partial<StoredState> = {}): StoredState {
	return {
		version: STORED_STATE_VERSION,
		searchQuery: '',
		filterState: {},
		sortState: { field: null, direction: 'asc' },
		selectedGroupField: null,
		currentPage: 1,
		...overrides
	};
}

describe('parseStoredState', () => {
	it('round-trips filter selections through serialization', () => {
		const state = baseState({
			searchQuery: 'switch',
			filterState: {
				name: { type: 'string', values: ['a', 'b'] },
				hidden: { type: 'boolean', values: [], showTrue: true, showFalse: false }
			},
			sortState: { field: 'name', direction: 'desc' },
			selectedGroupField: 'site_id',
			currentPage: 3,
			pageSize: 50
		});

		const parsed = parseStoredState(serializeState(state));

		expect(parsed).not.toBeNull();
		expect(new Set(parsed!.filterState.name.values)).toEqual(new Set(['a', 'b']));
		expect(parsed!.filterState.hidden.showTrue).toBe(true);
		expect(parsed!.filterState.hidden.showFalse).toBe(false);
		expect(parsed!.sortState).toEqual({ field: 'name', direction: 'desc' });
		expect(parsed!.searchQuery).toBe('switch');
		expect(parsed!.selectedGroupField).toBe('site_id');
		expect(parsed!.currentPage).toBe(3);
		expect(parsed!.pageSize).toBe(50);
	});

	it('opens a blob that chose card view on the table, keeping its other choices', () => {
		const raw = JSON.stringify({
			...baseState({ selectedGroupField: 'site_id' }),
			viewMode: 'card',
			showFilters: true
		});
		const parsed = parseStoredState(raw)!;

		expect(parsed).not.toHaveProperty('viewMode');
		expect(parsed).not.toHaveProperty('showFilters');
		expect(parsed.selectedGroupField).toBe('site_id');
	});

	it("reads an older build's null sort and group as no choice", () => {
		// Builds before version 2 wrote both as null on first mount, so the null was never picked.
		const raw = JSON.stringify({ ...baseState(), version: undefined });
		const parsed = parseStoredState(raw)!;

		expect(parsed.sortState).toBeUndefined();
		expect(parsed.selectedGroupField).toBeUndefined();
	});

	it('keeps a chosen "none" from the current build', () => {
		const parsed = parseStoredState(serializeState(baseState()))!;

		expect(parsed.sortState).toEqual({ field: null, direction: 'asc' });
		expect(parsed.selectedGroupField).toBeNull();
	});

	it('returns null rather than throwing on malformed input', () => {
		// A corrupt key must never blank a tab.
		expect(parseStoredState('{not json')).toBeNull();
		expect(parseStoredState('null')).toBeNull();
		expect(parseStoredState('"a string"')).toBeNull();
		expect(parseStoredState(null)).toBeNull();
		expect(parseStoredState('')).toBeNull();
	});

	it('fills defaults for a blob written by an older build', () => {
		// Every field is optional so a partial blob still loads.
		const parsed = parseStoredState(JSON.stringify({ searchQuery: 'x' }));

		expect(parsed).not.toBeNull();
		expect(parsed!.searchQuery).toBe('x');
		expect(parsed!.filterState).toEqual({});
		expect(parsed!.sortState).toBeUndefined();
		expect(parsed!.selectedGroupField).toBeUndefined();
		expect(parsed!.currentPage).toBe(1);
		expect(parsed!.pageSize).toBeUndefined();
	});

	it('rejects a page size that is no longer offered', () => {
		const parsed = parseStoredState(JSON.stringify({ ...baseState(), pageSize: 37 }));

		expect(parsed!.pageSize).toBeUndefined();
	});

	it('coerces a malformed sort direction to ascending', () => {
		const parsed = parseStoredState(
			JSON.stringify({ ...baseState(), sortState: { field: 'name', direction: 'sideways' } })
		);

		expect(parsed!.sortState).toEqual({ field: 'name', direction: 'asc' });
	});

	it('drops column state that is not the right shape', () => {
		const parsed = parseStoredState(
			JSON.stringify({
				...baseState(),
				columnVisibility: { name: 'yes' },
				columnSizing: { name: 'wide' }
			})
		);

		expect(parsed!.columnVisibility).toBeUndefined();
		expect(parsed!.columnSizing).toBeUndefined();
	});

	it('keeps well-formed column state', () => {
		const parsed = parseStoredState(
			JSON.stringify({
				...baseState(),
				columnVisibility: { name: true, created_at: false },
				columnOrder: ['name', 'created_at'],
				columnSizing: { name: 240 }
			})
		);

		expect(parsed!.columnVisibility).toEqual({ name: true, created_at: false });
		expect(parsed!.columnOrder).toEqual(['name', 'created_at']);
		expect(parsed!.columnSizing).toEqual({ name: 240 });
	});
});

describe('resolveOrdering', () => {
	const fieldKeys = new Set(['name', 'cidr', 'site_id']);
	const defaults = {
		group: 'cidr',
		sort: { field: 'name', direction: 'asc' as const }
	};

	it('applies the defaults when the user has chosen nothing', () => {
		expect(resolveOrdering({}, defaults, fieldKeys)).toEqual({
			sortState: { field: 'name', direction: 'asc' },
			groupField: 'cidr'
		});
	});

	it("keeps the user's sort and grouping over the defaults", () => {
		const resolved = resolveOrdering(
			{ sort: { field: 'site_id', direction: 'desc' }, group: 'site_id' },
			defaults,
			fieldKeys
		);

		expect(resolved).toEqual({
			sortState: { field: 'site_id', direction: 'desc' },
			groupField: 'site_id'
		});
	});

	it('keeps a chosen "no grouping" over a default grouping', () => {
		expect(resolveOrdering({ group: null }, defaults, fieldKeys).groupField).toBeNull();
	});

	it('falls back to the default for a choice on a field the tab no longer has', () => {
		const resolved = resolveOrdering(
			{ sort: { field: 'retired', direction: 'desc' }, group: 'retired' },
			defaults,
			fieldKeys
		);

		expect(resolved).toEqual({
			sortState: { field: 'name', direction: 'asc' },
			groupField: 'cidr'
		});
	});

	it('lands a legacy blob on the defaults', () => {
		const stored = parseStoredState(
			JSON.stringify({ sortState: { field: null, direction: 'asc' }, selectedGroupField: null })
		)!;
		const resolved = resolveOrdering(
			{ sort: stored.sortState, group: stored.selectedGroupField },
			defaults,
			fieldKeys
		);

		expect(resolved.groupField).toBe('cidr');
		expect(resolved.sortState.field).toBe('name');
	});
});
