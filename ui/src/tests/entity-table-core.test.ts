import { describe, it, expect } from 'vitest';
import {
	constructEntityTable,
	type EntityTableOptions
} from '#lib/shared/components/data/table/createSvelteTable.svelte.ts';

/**
 * The adapter's contract, exercised against real table-core.
 *
 * `createSvelteTable` itself needs runes and a component instance, but the part
 * that broke is pure: how a caller's partial `state` is resolved into the full
 * state every feature expects. That is worth pinning, because the failure mode
 * is a runtime throw while building header groups — nothing a type check or a
 * lint can see.
 */

interface Row {
	id: string;
	name: string;
}

const data: Row[] = [
	{ id: 'a', name: 'alpha' },
	{ id: 'b', name: 'beta' }
];

const columns = [
	{ id: 'name', accessorFn: (row: Row) => row.name, enableSorting: true },
	{ id: 'id', accessorFn: (row: Row) => row.id, enableSorting: false }
];

/** Builds a table the same way the adapter does, from a caller's partial state. */
function buildTable(state: EntityTableOptions<Row>['state']) {
	return constructEntityTable<Row>({
		data,
		columns,
		manualSorting: true,
		getRowId: (row) => row.id,
		state
	});
}

describe('table-core option resolution', () => {
	it('builds header groups when the caller supplies only some state', () => {
		// table-core v8 returned options.state verbatim, so a partial state left
		// columnPinning undefined and threw on `.left` while assembling the headers.
		const table = buildTable({ sorting: [{ id: 'name', desc: false }] });

		expect(() => table.getHeaderGroups()).not.toThrow();
		expect(table.getHeaderGroups()[0].headers.map((h) => h.column.id)).toEqual(['name', 'id']);
	});

	it('builds header groups when the caller supplies no state at all', () => {
		const table = buildTable({});

		expect(() => table.getHeaderGroups()).not.toThrow();
		expect(table.getRowModel().rows).toHaveLength(2);
	});

	it('leaves every feature slice defined after resolution', () => {
		const table = buildTable({ sorting: [] });
		const state = table.store.state;

		// The slices the registered features read while building headers.
		expect(state.columnSizing).toBeDefined();
		expect(state.columnResizing).toBeDefined();
		expect(state.columnResizing.isResizingColumn).toBe(false);
	});

	it('lets the caller override a default rather than the other way round', () => {
		const table = buildTable({ sorting: [{ id: 'name', desc: true }] });

		expect(table.store.state.sorting).toEqual([{ id: 'name', desc: true }]);
	});

	it('keeps rows in the order given, since sorting is the caller’s job', () => {
		// No sorted row model is registered: table-core must not reorder.
		const table = buildTable({ sorting: [{ id: 'name', desc: true }] });

		expect(table.getRowModel().rows.map((r) => r.original.name)).toEqual(['alpha', 'beta']);
	});

	it('reports sortability per column', () => {
		const table = buildTable({ sorting: [] });
		const headers = table.getHeaderGroups()[0].headers;

		expect(headers.find((h) => h.column.id === 'name')!.column.getCanSort()).toBe(true);
		expect(headers.find((h) => h.column.id === 'id')!.column.getCanSort()).toBe(false);
	});
});
