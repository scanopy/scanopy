import { describe, it, expect } from 'vitest';
import {
	isAllSelected,
	isPartiallySelected,
	visibleIds,
	rangeSelect
} from '$lib/shared/components/data/controls/selection';

interface Row {
	id: string;
}

const getId = (r: Row) => r.id;

function page(prefix: string, count: number): Row[] {
	return Array.from({ length: count }, (_, i) => ({ id: `${prefix}-${i}` }));
}

describe('isAllSelected', () => {
	it('is false when the selection came from a different page', () => {
		// The bug this replaces: comparing sizes made any 20 selected ids read as
		// "all" for a 20-row page, so the control offered "deselect all" while a
		// bulk delete would have acted on an entirely different set of rows.
		const pageOne = page('a', 20);
		const pageTwo = page('b', 20);
		const selected = new Set(pageOne.map(getId));

		expect(selected.size).toBe(pageTwo.length);
		expect(isAllSelected(pageTwo, selected, getId)).toBe(false);
	});

	it('is true only once every visible row is selected', () => {
		const visible = page('a', 3);
		const selected = new Set(['a-0', 'a-1']);

		expect(isAllSelected(visible, selected, getId)).toBe(false);

		selected.add('a-2');
		expect(isAllSelected(visible, selected, getId)).toBe(true);
	});

	it('ignores selections beyond the visible rows', () => {
		// Selecting page 1, paging, then selecting page 2 accumulates across pages —
		// the label reflects the current page without discarding the rest.
		const visible = page('b', 2);
		const selected = new Set(['a-0', 'a-1', 'b-0', 'b-1']);

		expect(isAllSelected(visible, selected, getId)).toBe(true);
	});

	it('is false for an empty visible set', () => {
		// Otherwise a filtered-to-nothing list would offer "deselect all".
		expect(isAllSelected([], new Set(), getId)).toBe(false);
		expect(isAllSelected([], new Set(['a-0']), getId)).toBe(false);
	});
});

describe('isPartiallySelected', () => {
	it('is true only when some but not all visible rows are selected', () => {
		const visible = page('a', 3);

		expect(isPartiallySelected(visible, new Set(), getId)).toBe(false);
		expect(isPartiallySelected(visible, new Set(['a-0']), getId)).toBe(true);
		expect(isPartiallySelected(visible, new Set(['a-0', 'a-1', 'a-2']), getId)).toBe(false);
	});

	it('never reports partial and all at the same time', () => {
		const visible = page('a', 3);
		const selections = [[], ['a-0'], ['a-0', 'a-1'], ['a-0', 'a-1', 'a-2']];

		for (const ids of selections) {
			const selected = new Set(ids);
			const all = isAllSelected(visible, selected, getId);
			const partial = isPartiallySelected(visible, selected, getId);
			expect(all && partial).toBe(false);
		}
	});
});

describe('visibleIds', () => {
	it('yields an id per visible row, dropping blanks', () => {
		const visible = [{ id: 'a' }, { id: '' }, { id: 'b' }];

		expect(visibleIds(visible, getId)).toEqual(['a', 'b']);
	});

	it('selecting all visible ids satisfies isAllSelected', () => {
		// The action and the label are derived from the same set, so they agree.
		const visible = page('a', 5);
		const selected = new Set(visibleIds(visible, getId));

		expect(isAllSelected(visible, selected, getId)).toBe(true);
	});
});

describe('rangeSelect', () => {
	it('covers the same rows regardless of drag direction', () => {
		const visible = page('a', 5);

		const down = rangeSelect(visible, 'a-1', 'a-3', getId);
		const up = rangeSelect(visible, 'a-3', 'a-1', getId);

		expect(down).toEqual(['a-1', 'a-2', 'a-3']);
		expect(up).toEqual(down);
	});

	it('includes both ends', () => {
		const visible = page('a', 5);

		expect(rangeSelect(visible, 'a-2', 'a-2', getId)).toEqual(['a-2']);
	});

	it('returns a subset of the visible rows', () => {
		const visible = page('a', 5);
		const ids = new Set(visibleIds(visible, getId));

		for (const id of rangeSelect(visible, 'a-0', 'a-4', getId)) {
			expect(ids.has(id)).toBe(true);
		}
	});

	it('yields nothing when an endpoint is no longer visible', () => {
		// A stale anchor is reachable by paging between clicks.
		const visible = page('a', 3);

		expect(rangeSelect(visible, 'gone', 'a-1', getId)).toEqual([]);
		expect(rangeSelect(visible, 'a-1', 'gone', getId)).toEqual([]);
	});
});
