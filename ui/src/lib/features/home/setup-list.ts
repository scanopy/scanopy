import type { components } from '$lib/api/schema';

export type SetupItem = components['schemas']['SetupItem'];
export type SetupItemType = components['schemas']['SetupItemDiscriminants'];
export type SetupItemTier = components['schemas']['SetupItemTier'];
export type SetupItemState = components['schemas']['SetupItemState'];
/** One Setup list row as the server serves it: the item, its key, its tier and its state. */
export type SetupListEntry = components['schemas']['SetupListEntry'];

/** The Issues among `entries`, in the order the server listed them. */
export function issueEntries(entries: SetupListEntry[]): SetupListEntry[] {
	return entries.filter((entry) => entry.tier === 'Issue');
}

/** The Issues among `entries`: the Home nav badge and the Issues heading badge. */
export function setupIssueCount(entries: SetupListEntry[]): number {
	return issueEntries(entries).length;
}

/** The Setup items still to do, in the order the server listed them, so the first steps lead. */
export function openEntries(entries: SetupListEntry[]): SetupListEntry[] {
	return entries.filter((entry) => entry.tier === 'Setup' && entry.state === 'Open');
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

/** The server's key as a string, to key the list and the once-per-key `setup_issue_shown` event. */
export function entryId(entry: SetupListEntry): string {
	return JSON.stringify(entry.key);
}
