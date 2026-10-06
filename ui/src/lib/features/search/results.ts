/**
 * Global search: the pure half of the Cmd+K palette.
 *
 * The palette shows one section per entity type and moves a single highlight through all of them
 * with the arrow keys, so the sections are flattened into one list that the highlight indexes.
 * Kept free of components and the API so the ordering and keyboard rules are testable on their own.
 */

import { writable } from 'svelte/store';
import type { EntityDiscriminants } from '$lib/api/entities';

/** Whether the palette is open. The sidebar's search bar and the shortcut both set it. */
export const globalSearchOpen = writable(false);

/**
 * A query for the palette to reopen with, set by "Back to Search" on an entity opened from it.
 * The palette consumes it on open and resets it to `null`.
 */
export const globalSearchRestoreQuery = writable<string | null>(null);

/** Open the palette with `query` already searched, as the user left it. */
export function reopenGlobalSearch(query: string) {
	globalSearchRestoreQuery.set(query);
	globalSearchOpen.set(true);
}

/** One section of results: every match of one entity type the server returned. */
export interface SearchGroup<T = unknown> {
	type: EntityDiscriminants;
	items: T[];
}

/** One row of the flattened list, carrying its section's type so Enter knows what to open. */
export interface SearchRow<T = unknown> {
	type: EntityDiscriminants;
	item: T;
}

/** Every row in section order, sections with no matches dropped. */
export function flattenGroups<T>(groups: SearchGroup<T>[]): SearchRow<T>[] {
	return groups.flatMap((group) => group.items.map((item) => ({ type: group.type, item })));
}

/**
 * Where the highlight lands after an arrow key. Wraps at both ends, so Up on the first row reaches
 * the last. `-1` (nothing highlighted) moves to the first row on Down and the last on Up.
 */
export function moveHighlight(current: number, delta: 1 | -1, total: number): number {
	if (total === 0) return -1;
	if (current < 0) return delta === 1 ? 0 : total - 1;
	return (current + delta + total) % total;
}

/**
 * The palette's shortcut: Cmd+K on macOS, Ctrl+K elsewhere. Either modifier is accepted on every
 * platform, matching the topology's Cmd/Ctrl+F; Shift and Alt variants are left to the browser.
 */
export function isGlobalSearchShortcut(
	event: Pick<KeyboardEvent, 'key' | 'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey'>
): boolean {
	return (
		(event.metaKey || event.ctrlKey) &&
		!event.altKey &&
		!event.shiftKey &&
		event.key.toLowerCase() === 'k'
	);
}

/** The shortcut as the sidebar shows it: `⌘K` on macOS, `Ctrl K` elsewhere. */
export function shortcutLabel(platform: string): string {
	return /mac|iphone|ipad/i.test(platform) ? '⌘K' : 'Ctrl K';
}
