import { describe, it, expect } from 'vitest';
import {
	migrateViewMode,
	parseStoredState,
	serializeState,
	DEFAULT_VIEW_MODE,
	type StoredState
} from '$lib/shared/components/data/controls/dataControlsStorage';

function baseState(overrides: Partial<StoredState> = {}): StoredState {
	return {
		searchQuery: '',
		filterState: {},
		sortState: { field: null, direction: 'asc' },
		selectedGroupField: null,
		showFilters: false,
		viewMode: 'card',
		currentPage: 1,
		...overrides
	};
}

describe('migrateViewMode', () => {
	it('carries a stored list view onto the table', () => {
		// List view was folded into the table, so someone who chose the dense view
		// keeps a dense view rather than being reset to cards.
		expect(migrateViewMode('list')).toBe('table');
	});

	it('preserves the two live modes', () => {
		expect(migrateViewMode('table')).toBe('table');
		expect(migrateViewMode('card')).toBe('card');
	});

	it('never yields a value outside the union, whatever is stored', () => {
		const junk = [undefined, null, '', 'grid', 'LIST', 0, 1, {}, [], true, 'table ', 'Card'];

		for (const raw of junk) {
			expect(['card', 'table']).toContain(migrateViewMode(raw));
		}
	});

	it('falls back to the default for unrecognised values', () => {
		expect(migrateViewMode('grid')).toBe(DEFAULT_VIEW_MODE);
	});
});

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

	it('migrates a stored list view while parsing', () => {
		const raw = JSON.stringify({ ...baseState(), viewMode: 'list' });

		expect(parseStoredState(raw)!.viewMode).toBe('table');
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
		expect(parsed!.sortState).toEqual({ field: null, direction: 'asc' });
		expect(parsed!.viewMode).toBe(DEFAULT_VIEW_MODE);
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
