/**
 * What a list's single-key shortcuts need to know about the list they act on, provided by
 * `DataControls` to the controls rendered in its toolbar (such as `CreateButton`). A shortcut acts
 * only while its list is the one on screen, and its key hint dims while the table search has focus,
 * when typing goes to the search instead.
 */
import { getContext, setContext } from 'svelte';
import { writable } from 'svelte/store';
import { isEditableTarget } from '#lib/shared/utils/shortcuts.ts';

const KEY = Symbol('list-shortcuts');

/**
 * A table search has focus somewhere on the page. Shortcuts outside the list (the sidebar's global
 * search key) read it to dim their hints too, since typing goes to the table search.
 */
export const tableSearchFocused = writable(false);

export interface ListShortcuts {
	/** The list is on screen and nothing (a modal) is over it. */
	isActive: () => boolean;
	/** The table search has focus, so single keys type there. */
	isPaused: () => boolean;
}

export function provideListShortcuts(shortcuts: ListShortcuts) {
	setContext(KEY, shortcuts);
}

export function useListShortcuts(): ListShortcuts | undefined {
	return getContext<ListShortcuts | undefined>(KEY);
}

/**
 * A keydown a list shortcut may take: not typed into a field, not already handled by the focused
 * control (Delete on a focused tag removes the tag), and with no modal open.
 */
export function isFreeKeydown(event: KeyboardEvent): boolean {
	return (
		!event.defaultPrevented &&
		!isEditableTarget(event.target) &&
		!document.querySelector('[aria-modal="true"]')
	);
}

/** A single key with no modifier held. */
export function isPlainKey(event: KeyboardEvent, key: string): boolean {
	return (
		!event.metaKey &&
		!event.ctrlKey &&
		!event.altKey &&
		!event.shiftKey &&
		event.key.toUpperCase() === key.toUpperCase()
	);
}
