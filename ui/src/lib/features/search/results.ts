/**
 * Global search: the pure half of the global search palette.
 *
 * The palette shows one section per entity type and moves a single highlight through all of them
 * with the arrow keys, so the sections are flattened into one list that the highlight indexes.
 * Typing a tag's name offers that tag as a completion; Tab turns it into a chip, and chips narrow
 * the results to entities carrying every one of them.
 * Kept free of components and the API so the ordering, completion and keyboard rules are testable
 * on their own.
 */

import { writable } from 'svelte/store';
import type { components } from '$lib/api/schema';
import type { EntityDiscriminants } from '$lib/api/entities';
import type { Tag } from '$lib/features/tags/types/base';
import { isEditableTarget } from '$lib/shared/utils/shortcuts';

/** What the palette searches for: the typed text and the tag chips before it. */
export interface GlobalSearchState {
	text: string;
	tagIds: string[];
}

export const EMPTY_SEARCH: GlobalSearchState = { text: '', tagIds: [] };

/** Whether there is anything to search for. */
export function hasSearchTerms(state: GlobalSearchState): boolean {
	return state.text.trim().length > 0 || state.tagIds.length > 0;
}

/** Whether the palette is open. The sidebar's search bar and the shortcut both set it. */
export const globalSearchOpen = writable(false);

/**
 * A search for the palette to reopen with, set by "Back to Search" on an entity opened from it.
 * The palette consumes it on open and resets it to `null`.
 */
export const globalSearchRestoreQuery = writable<GlobalSearchState | null>(null);

/** Open the palette with `state` already searched, as the user left it. */
export function reopenGlobalSearch(state: GlobalSearchState) {
	globalSearchRestoreQuery.set(state);
	globalSearchOpen.set(true);
}

/** One section of results: every match of one entity type the server returned. */
export interface SearchGroup<T = unknown> {
	type: EntityDiscriminants;
	/** The matches loaded so far: the first page, then any pages "Show more" added. */
	items: T[];
	/** Matches of this type in all. */
	total: number;
}

/**
 * One row of the flattened list, carrying its section's type so Enter knows what to act on: a
 * match to open, or the section's "Show more" row with how many matches are not loaded yet.
 */
export type SearchRow<T = unknown> =
	{ type: EntityDiscriminants; item: T } | { type: EntityDiscriminants; more: number };

type GlobalSearchResponse = components['schemas']['GlobalSearchResponse'];
type SearchHit = components['schemas']['SearchHit'];
/** The payload of each hit variant (`{ Host: HostResponse }` → the host), distributed over the union. */
type HitPayload<H> = H extends Record<string, infer P> ? P : never;
/** Any entity a match can carry, in the shape its list returns it. */
export type SearchItem = HitPayload<SearchHit>;

/** The payload inside one hit (`{ Host: {...} }` → the host). */
function hitPayload(hit: SearchHit): SearchItem {
	return Object.values(hit)[0] as SearchItem;
}

/** The server's groups as sections, each match unwrapped from its hit variant. */
export function responseGroups(response: GlobalSearchResponse): SearchGroup<SearchItem>[] {
	return response.groups.map((group) => ({
		type: group.entity_type,
		items: group.items.map(hitPayload),
		total: group.total_count
	}));
}

/** One type's later page, unwrapped. */
export function responsePage(response: GlobalSearchResponse): SearchItem[] {
	return response.groups.flatMap((group) => group.items.map(hitPayload));
}

/** Each section with the pages "Show more" loaded for its type appended. */
export function withMorePages<T>(
	groups: SearchGroup<T>[],
	more: Partial<Record<EntityDiscriminants, T[]>>
): SearchGroup<T>[] {
	return groups.map((group) => ({
		...group,
		items: [...group.items, ...(more[group.type] ?? [])]
	}));
}

/**
 * Every row in section order, sections with no matches dropped. A section with matches still to
 * load ends in a "Show more" row, so the arrows reach it like any match.
 */
export function flattenGroups<T>(groups: SearchGroup<T>[]): SearchRow<T>[] {
	return groups.flatMap((group): SearchRow<T>[] => {
		const rows: SearchRow<T>[] = group.items.map((item) => ({ type: group.type, item }));
		const remaining = group.total - group.items.length;
		return remaining > 0 ? [...rows, { type: group.type, more: remaining }] : rows;
	});
}

/**
 * The tag the typed text completes to, or `null`. The text, trimmed and ignoring case, must start
 * a tag's name; a tag already chipped is not offered again. Of several, an exact name wins, then the
 * shortest name (the fewest letters left to type), then the alphabetically first.
 */
export function tagCompletion(text: string, tags: Tag[], chosenIds: string[]): Tag | null {
	const typed = text.trim().toLowerCase();
	if (!typed) return null;
	const candidates = tags.filter(
		(tag) => !chosenIds.includes(tag.id) && tag.name.toLowerCase().startsWith(typed)
	);
	candidates.sort(
		(a, b) =>
			Number(b.name.toLowerCase() === typed) - Number(a.name.toLowerCase() === typed) ||
			a.name.length - b.name.length ||
			a.name.localeCompare(b.name)
	);
	return candidates[0] ?? null;
}

/** A click on a tag a result carries: it becomes a chip, keeping the text. Once only. */
export function addChip(state: GlobalSearchState, tagId: string): GlobalSearchState {
	if (state.tagIds.includes(tagId)) return state;
	return { ...state, tagIds: [...state.tagIds, tagId] };
}

/** Tab on a completion: the tag becomes a chip and the text it completed is cleared. */
export function acceptCompletion(state: GlobalSearchState, tag: Tag): GlobalSearchState {
	return addChip({ ...state, text: '' }, tag.id);
}

/** The chip's x. */
export function removeChip(state: GlobalSearchState, tagId: string): GlobalSearchState {
	return { ...state, tagIds: state.tagIds.filter((id) => id !== tagId) };
}

/**
 * Where the chip selection lands after Left or Right, as an index into the chips; `null` is the
 * text. Left from the text selects the last chip, but only with the caret at the start of the
 * text, so Left still moves through what was typed. Left stops at the first chip; Right past the
 * last chip returns to the text.
 */
export function moveChipCursor(
	cursor: number | null,
	direction: 'left' | 'right',
	chipCount: number,
	caretAtStart: boolean
): number | null {
	if (chipCount === 0) return null;
	if (direction === 'left') {
		if (cursor === null) return caretAtStart ? chipCount - 1 : null;
		return Math.max(cursor - 1, 0);
	}
	if (cursor === null) return null;
	return cursor + 1 < chipCount ? cursor + 1 : null;
}

/**
 * Backspace or Delete on a selected chip: it is removed and the selection moves to the chip before
 * it (or the new first chip), back to the text once none are left.
 */
export function removeSelectedChip(
	state: GlobalSearchState,
	cursor: number
): { state: GlobalSearchState; cursor: number | null } {
	const tagIds = state.tagIds.filter((_, index) => index !== cursor);
	return {
		state: { ...state, tagIds },
		cursor: tagIds.length === 0 ? null : Math.min(Math.max(cursor - 1, 0), tagIds.length - 1)
	};
}

/**
 * Backspace on an empty input removes the last chip, as it deletes the character before the
 * caret. `null` when there is text or no chip, so the key keeps its usual meaning.
 */
export function backspaceChip(state: GlobalSearchState): GlobalSearchState | null {
	if (state.text !== '' || state.tagIds.length === 0) return null;
	return { ...state, tagIds: state.tagIds.slice(0, -1) };
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

/** The key that opens the palette ("Search all"). */
export const GLOBAL_SEARCH_KEY = '/';

/**
 * Whether a keypress should open the palette: a bare `/` while nothing editable has focus, so
 * typing a slash into any field still types it.
 */
export function isGlobalSearchShortcut(
	event: Pick<KeyboardEvent, 'key' | 'metaKey' | 'ctrlKey' | 'altKey' | 'target'>
): boolean {
	if (event.key !== GLOBAL_SEARCH_KEY || event.metaKey || event.ctrlKey || event.altKey) {
		return false;
	}
	return !isEditableTarget(event.target);
}

/**
 * Cmd+F on macOS, Ctrl+F elsewhere: focuses the filter of the list page on screen, as it does the
 * topology's find. Either modifier is accepted on every platform; Shift and Alt variants are left
 * to the browser.
 */
export function isFindShortcut(
	event: Pick<KeyboardEvent, 'key' | 'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey'>
): boolean {
	return (
		(event.metaKey || event.ctrlKey) &&
		!event.altKey &&
		!event.shiftKey &&
		event.key.toLowerCase() === 'f'
	);
}
