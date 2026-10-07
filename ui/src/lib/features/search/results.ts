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
	items: T[];
}

/** One row of the flattened list, carrying its section's type so Enter knows what to open. */
export interface SearchRow<T = unknown> {
	type: EntityDiscriminants;
	item: T;
}

type GlobalSearchResponse = components['schemas']['GlobalSearchResponse'];
type SearchEntity = components['schemas']['Entity'];
/** The payload of each `Entity` variant (`{ Host: Host }` → `Host`), distributed over the union. */
type EntityPayload<E> = E extends Record<string, infer P> ? P : never;
/** Any entity a match can carry. */
export type SearchItem = EntityPayload<Exclude<SearchEntity, string>>;

/** The payload inside one `Entity` (`{ Host: {...} }` → the host). */
function entityPayload(entity: SearchEntity): SearchItem[] {
	return typeof entity === 'string' ? [] : (Object.values(entity) as SearchItem[]);
}

/** The server's groups as sections, each match unwrapped from its entity variant. */
export function responseGroups(response: GlobalSearchResponse): SearchGroup<SearchItem>[] {
	return response.groups.map((group) => ({
		type: group.entity_type,
		items: group.items.flatMap(entityPayload)
	}));
}

/** Every row in section order, sections with no matches dropped. */
export function flattenGroups<T>(groups: SearchGroup<T>[]): SearchRow<T>[] {
	return groups.flatMap((group) => group.items.map((item) => ({ type: group.type, item })));
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

/** Tab on a completion: the tag becomes a chip and the text it completed is cleared. */
export function acceptCompletion(state: GlobalSearchState, tag: Tag): GlobalSearchState {
	return { text: '', tagIds: [...state.tagIds, tag.id] };
}

/** The chip's x. */
export function removeChip(state: GlobalSearchState, tagId: string): GlobalSearchState {
	return { ...state, tagIds: state.tagIds.filter((id) => id !== tagId) };
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
