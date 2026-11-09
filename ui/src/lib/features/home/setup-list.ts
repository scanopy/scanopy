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

/** The sites `issues` are about, in first-seen order. An org-wide Issue names none. */
export function issueSiteIds(issues: SetupListEntry[]): string[] {
	return [...new Set(issues.flatMap((entry) => entry.site_ids))];
}

/**
 * The `issues` to show with `picked` sites on: an org-wide Issue always, and one about a site
 * when any of its sites is picked.
 */
export function issuesOnSites(issues: SetupListEntry[], picked: Set<string>): SetupListEntry[] {
	return issues.filter(
		(entry) => entry.site_ids.length === 0 || entry.site_ids.some((id) => picked.has(id))
	);
}

/** The server's key as a string, to key the list and the once-per-key `setup_issue_shown` event. */
export function entryId(entry: SetupListEntry): string {
	return JSON.stringify(entry.key);
}
