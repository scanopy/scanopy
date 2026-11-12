import type { components } from '#lib/api/schema';

type PaginationMeta = components['schemas']['PaginationMeta'];

/** What the pagination bar shows, and what the page buttons are allowed to do. */
export interface PaginationView {
	/** 1-based, resolved from the server's offset. */
	currentPage: number;
	totalPages: number;
	canGoPrev: boolean;
	canGoNext: boolean;
	/** 1-based index of the first row shown, clamped to the total. */
	showingStart: number;
	/** 1-based index of the last row shown, clamped to the total. */
	showingEnd: number;
	/** Rows matching the filters, across every page — not this page's length. */
	totalCount: number;
}

/**
 * Resolve the pagination state from the server's metadata.
 *
 * `server` is null until the first page arrives, which reads as an empty list. `loadedCount` is
 * how many rows the page holds.
 */
export function paginationView(
	server: PaginationMeta | null,
	pageSize: number,
	loadedCount: number
): PaginationView {
	if (!server) {
		return {
			currentPage: 1,
			totalPages: 0,
			canGoPrev: false,
			canGoNext: false,
			showingStart: 0,
			showingEnd: 0,
			totalCount: 0
		};
	}

	const currentPage = Math.floor(server.offset / pageSize) + 1;

	return {
		currentPage,
		totalPages: Math.ceil(server.total_count / pageSize),
		canGoPrev: currentPage > 1,
		// The server says whether more rows exist; a page count derived from a
		// total it also supplied would only ever repeat that claim less exactly.
		canGoNext: server.has_more,
		showingStart: Math.min(server.offset + 1, server.total_count),
		showingEnd: Math.min(server.offset + loadedCount, server.total_count),
		totalCount: server.total_count
	};
}
