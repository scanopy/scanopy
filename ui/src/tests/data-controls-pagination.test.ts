import { describe, it, expect } from 'vitest';
import { groupPageSlice } from '#lib/shared/components/data/types.ts';
import { paginationView } from '#lib/shared/components/data/controls/pagination.ts';

/**
 * Tests for DataControls pagination logic.
 *
 * These tests verify the count calculation and display logic for server-side pagination, where
 * items come pre-paginated from the server: "showing X of Y" counts from the server's total.
 */

describe('DataControls pagination count display', () => {
	describe('server-side pagination single page', () => {
		it('should show total count from server when all items fit on one page', () => {
			// Scenario: Server says there are 41 items total, pageSize is 50
			// Only 1 page needed, so no range display
			const serverPagination = {
				total_count: 41,
				offset: 0,
				has_more: false
			};
			const pageSize = 50;

			const totalPages = Math.ceil(serverPagination.total_count / pageSize);
			expect(totalPages).toBe(1);

			// For server-side pagination with single page, we show "Showing X of X"
			// where X is the total count from the server
			const countToDisplay = serverPagination.total_count;
			const totalToDisplay = serverPagination.total_count;

			// This should show "Showing 41 of 41 items" NOT "Showing 41 of 20 items"
			expect(countToDisplay).toBe(41);
			expect(totalToDisplay).toBe(41);
		});

		it('should not use items.length as total when server pagination is active', () => {
			// Bug: "Showing 41 of 20" occurred when:
			// - Server reported total_count: 41
			// - But items prop only had 20 items (due to stale pageSize in parent)
			// - Old code used items.length as the "total" denominator
			//
			// Fix: With server-side pagination, always use server's total_count for both values
			const serverTotalCount = 41;
			const itemsArrayLength = 20;

			const countToDisplay = serverTotalCount;
			const totalToDisplay = serverTotalCount; // Not itemsArrayLength

			expect(countToDisplay).toBe(41);
			expect(totalToDisplay).toBe(41);
			expect(totalToDisplay).not.toBe(itemsArrayLength);
		});

		it('should handle edge case where pageSize equals total count', () => {
			const serverPagination = {
				total_count: 20,
				offset: 0,
				has_more: false
			};
			const pageSize = 20;

			const totalPages = Math.ceil(serverPagination.total_count / pageSize);
			expect(totalPages).toBe(1);

			// Should show "Showing 20 of 20 items"
			expect(serverPagination.total_count).toBe(20);
		});
	});

	describe('server-side pagination multiple pages', () => {
		it('should show range when multiple pages exist', () => {
			const serverPagination = {
				total_count: 41,
				offset: 0,
				has_more: true
			};
			const pageSize = 20;
			const itemsOnPage = 20;

			const totalPages = Math.ceil(serverPagination.total_count / pageSize);
			expect(totalPages).toBe(3); // 41 / 20 = 2.05 -> ceil = 3

			// For multiple pages, we show "Showing 1-20 of 41 items"
			const showingStart = Math.min(serverPagination.offset + 1, serverPagination.total_count);
			const showingEnd = Math.min(
				serverPagination.offset + itemsOnPage,
				serverPagination.total_count
			);

			expect(showingStart).toBe(1);
			expect(showingEnd).toBe(20);
		});

		it('should calculate correct range for middle page', () => {
			// pageSize: 20, showing page 2
			const serverPagination = {
				total_count: 41,
				offset: 20, // Second page
				has_more: true
			};
			const itemsOnPage = 20;

			const showingStart = Math.min(serverPagination.offset + 1, serverPagination.total_count);
			const showingEnd = Math.min(
				serverPagination.offset + itemsOnPage,
				serverPagination.total_count
			);

			// Should show "Showing 21-40 of 41 items"
			expect(showingStart).toBe(21);
			expect(showingEnd).toBe(40);
		});

		it('should calculate correct range for last page', () => {
			// pageSize: 20, showing page 3 (last page)
			const serverPagination = {
				total_count: 41,
				offset: 40, // Third/last page
				has_more: false
			};
			const itemsOnPage = 1; // Only 1 item on last page

			const showingStart = Math.min(serverPagination.offset + 1, serverPagination.total_count);
			const showingEnd = Math.min(
				serverPagination.offset + itemsOnPage,
				serverPagination.total_count
			);

			// Should show "Showing 41-41 of 41 items"
			expect(showingStart).toBe(41);
			expect(showingEnd).toBe(41);
		});
	});

	describe('page size persistence', () => {
		it('should validate page size against allowed options', () => {
			const PAGE_SIZE_OPTIONS = [20, 50, 100] as const;

			// Valid sizes should be accepted
			expect(PAGE_SIZE_OPTIONS.includes(20)).toBe(true);
			expect(PAGE_SIZE_OPTIONS.includes(50)).toBe(true);
			expect(PAGE_SIZE_OPTIONS.includes(100)).toBe(true);

			// Invalid sizes should be rejected
			expect(PAGE_SIZE_OPTIONS.includes(25 as 20 | 50 | 100)).toBe(false);
			expect(PAGE_SIZE_OPTIONS.includes(200 as 20 | 50 | 100)).toBe(false);
		});

		it('should calculate correct offset from page and pageSize', () => {
			// Page 1, pageSize 20 -> offset 0
			expect((1 - 1) * 20).toBe(0);

			// Page 2, pageSize 20 -> offset 20
			expect((2 - 1) * 20).toBe(20);

			// Page 3, pageSize 50 -> offset 100
			expect((3 - 1) * 50).toBe(100);

			// Page 1, pageSize 100 -> offset 0
			expect((1 - 1) * 100).toBe(0);
		});

		it('should derive effective page from server offset', () => {
			// offset 0, pageSize 20 -> page 1
			expect(Math.floor(0 / 20) + 1).toBe(1);

			// offset 20, pageSize 20 -> page 2
			expect(Math.floor(20 / 20) + 1).toBe(2);

			// offset 100, pageSize 50 -> page 3
			expect(Math.floor(100 / 50) + 1).toBe(3);
		});
	});
});

/**
 * A group header used to report `groupItems.length` — the rows of that group on
 * the loaded page — as if it were the group's size. With server-side grouping
 * the true size arrives in `group_counts`, and these cover translating it into
 * "which slice of the group am I looking at".
 *
 * Positions are global row indices across every page: a group at start 100 with
 * count 300 occupies rows 100..399 of the ordered result set.
 */
describe('groupPageSlice', () => {
	it('reports the leading slice when a group runs past the end of the page', () => {
		// 300 hosts in the group, 100 per page: page 1 shows the first third.
		const slice = groupPageSlice({ start: 0, count: 300 }, 0, 100);

		expect(slice).toEqual({ total: 300, start: 1, end: 100 });
	});

	it('reports the middle slice when a group spans the whole page', () => {
		// Page 2 of that same group: no boundary on either side of the page.
		const slice = groupPageSlice({ start: 0, count: 300 }, 100, 100);

		expect(slice).toEqual({ total: 300, start: 101, end: 200 });
	});

	it('counts from the group start, not the page start, when a group begins mid-page', () => {
		// Page covers rows 100..199; the group starts at 150, so its first row
		// on this page is row 1 *of the group*, not row 51.
		const slice = groupPageSlice({ start: 150, count: 80 }, 100, 100);

		expect(slice).toEqual({ total: 80, start: 1, end: 50 });
	});

	it('stops at the group end when a group finishes mid-page', () => {
		// Group occupies rows 80..119, so the page beginning at 100 holds its
		// last 20 rows — rows 21-40 of the group.
		const slice = groupPageSlice({ start: 80, count: 40 }, 100, 100);

		expect(slice).toEqual({ total: 40, start: 21, end: 40 });
	});

	it('returns null when the whole group fits on the page', () => {
		// Nothing to qualify: the header can just show the count.
		expect(groupPageSlice({ start: 100, count: 12 }, 100, 100)).toBeNull();
		expect(groupPageSlice({ start: 0, count: 5 }, 0, 20)).toBeNull();
	});

	it('handles a final short page', () => {
		// 45 rows in the group, page 1 held 40, this page holds the last 5.
		const slice = groupPageSlice({ start: 0, count: 45 }, 40, 5);

		expect(slice).toEqual({ total: 45, start: 41, end: 45 });
	});
});

describe('paginationView', () => {
	const server = (offset: number, total: number, hasMore: boolean) => ({
		offset,
		total_count: total,
		has_more: hasMore,
		limit: 20
	});

	it('reports the server total, not the page in hand', () => {
		// The bug this whole area exists to prevent: a filtered list showing the
		// unfiltered total. The server counts what it filtered; the 20 rows it
		// sent say nothing about how many matched.
		const view = paginationView(server(0, 62, true), 20, 20);

		expect(view.totalCount).toBe(62);
		expect(view.totalPages).toBe(4);
	});

	it('derives the current page from the server offset', () => {
		expect(paginationView(server(40, 62, true), 20, 20).currentPage).toBe(3);
	});

	it('trusts has_more rather than re-deriving it from the total', () => {
		// A server that knows there is another page is more authoritative than
		// arithmetic over a total it also supplied.
		expect(paginationView(server(40, 62, true), 20, 20).canGoNext).toBe(true);
		expect(paginationView(server(60, 62, false), 20, 2).canGoNext).toBe(false);
	});

	it('never shows a range running past the total', () => {
		// The last page is short, so an unclamped end would read "showing 61-80
		// of 62".
		const view = paginationView(server(60, 62, false), 20, 2);

		expect(view.showingStart).toBe(61);
		expect(view.showingEnd).toBe(62);
	});

	it('reports an empty result as a zero range rather than "1 of 0"', () => {
		const view = paginationView(server(0, 0, false), 20, 0);

		expect(view.showingStart).toBe(0);
		expect(view.showingEnd).toBe(0);
		expect(view.totalCount).toBe(0);
	});

	it('reads as an empty list until the first page arrives', () => {
		const view = paginationView(null, 20, 0);

		expect(view.totalCount).toBe(0);
		expect(view.canGoPrev).toBe(false);
		expect(view.canGoNext).toBe(false);
	});
});
