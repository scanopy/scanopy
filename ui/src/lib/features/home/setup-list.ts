import type { components } from '$lib/api/schema';
import warningCodes from '$lib/data/warning-codes.json';
import { setupItems } from '$lib/shared/stores/metadata';

export type SetupItem = components['schemas']['SetupItem'];
export type SetupItemType = components['schemas']['SetupItemDiscriminants'];
export type SetupItemTier = components['schemas']['SetupItemTier'];
export type SetupItemState = components['schemas']['SetupItemState'];
/** One Setup list row as the server serves it: the item, its key, its tier and its state. */
export type SetupListEntry = components['schemas']['SetupListEntry'];

/**
 * The category an Issue groups under, from type metadata: a run warning's code's
 * `issue_category`, else its type's `category`. The server groups the Issues list by the same.
 */
export function issueCategoryOf(item: SetupItem): string {
	if (item.type === 'RunWarnings') {
		const code = warningCodes.find((c) => c.id === item.code);
		return code?.metadata.issue_category ?? '';
	}
	return setupItems.getCategory(item.type);
}

/** The Setup items still to do, in the order the server listed them, so the first steps lead. */
export function openEntries(entries: SetupListEntry[]): SetupListEntry[] {
	return entries.filter((entry) => entry.tier === 'Setup' && entry.state === 'Open');
}

/** The Setup items waiting on an earlier step, shown disabled after the open ones. */
export function lockedEntries(entries: SetupListEntry[]): SetupListEntry[] {
	return entries.filter((entry) => entry.tier === 'Setup' && entry.state === 'Locked');
}

/** The Setup items done. */
export function doneEntries(entries: SetupListEntry[]): SetupListEntry[] {
	return entries.filter((entry) => entry.tier === 'Setup' && entry.state === 'Done');
}

/** Setup progress: the Done items out of every Setup item, Locked ones included. */
export function setupProgress(entries: SetupListEntry[]): { done: number; total: number } {
	const setup = entries.filter((entry) => entry.tier === 'Setup');
	return { done: doneEntries(setup).length, total: setup.length };
}

/**
 * The Setup steps an org has left, which the Setup tab shows while there are any. A lapsed org
 * has none: it keeps its plan's feature flags but can act on none of them.
 */
export function setupStepsLeft(entries: SetupListEntry[], lapsed: boolean): number {
	if (lapsed) return 0;
	const { done, total } = setupProgress(entries);
	return total - done;
}

/** The server's key as a string, to key the list and the once-per-key `setup_issue_shown` event. */
export function entryId(entry: SetupListEntry): string {
	return JSON.stringify(entry.key);
}
