import { describe, it, expect } from 'vitest';
import { listIsEmpty } from '#lib/api/list-page.ts';

const page = (rows: number, isPlaceholderData = false) => ({
	data: { items: Array.from({ length: rows }, (_, i) => i) },
	isPlaceholderData
});

describe('listIsEmpty', () => {
	it('keeps the table up while a cleared filter refetches', () => {
		// The filtered page matched nothing and stays on screen while the unfiltered request is in
		// flight. Read as "none yet", it unmounted the table before the cleared view was saved, and
		// the remount restored the filter (Users tab stuck on "No items match").
		expect(listIsEmpty(page(0, true), false)).toBe(false);
	});

	it('reports a list with no rows once its own response arrives', () => {
		expect(listIsEmpty(page(0), false)).toBe(true);
	});

	it('leaves an empty filtered result to the table, which offers Clear', () => {
		expect(listIsEmpty(page(0), true)).toBe(false);
	});

	it('is not empty when the page has rows', () => {
		expect(listIsEmpty(page(2), false)).toBe(false);
	});
});
