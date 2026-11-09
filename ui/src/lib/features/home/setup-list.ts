import type { components } from '$lib/api/schema';
import { setupItemTiers } from '$lib/shared/stores/metadata';

export type SetupItem = components['schemas']['SetupItem'];
export type SetupItemType = components['schemas']['SetupItemDiscriminants'];
export type SetupItemTier = components['schemas']['SetupItemTier'];
/** One Setup list row as the server serves it: the item, its key and its tier. */
export type SetupListEntry = components['schemas']['SetupListEntry'];

/** One tier of the Setup list and its rows, in the order the server listed them. */
export interface SetupTierGroup {
	tier: SetupItemTier;
	entries: SetupListEntry[];
}

/** The entries grouped by tier, in the tiers' display order. A tier with no entries is left out. */
export function groupByTier(entries: SetupListEntry[]): SetupTierGroup[] {
	return (setupItemTiers.getItems() ?? [])
		.map((t) => ({
			tier: t.id as SetupItemTier,
			entries: entries.filter((entry) => entry.tier === t.id)
		}))
		.filter((group) => group.entries.length > 0);
}

/** The Issues among `entries`: the Home nav badge. */
export function setupIssueCount(entries: SetupListEntry[]): number {
	return entries.filter((entry) => entry.tier === 'Issue').length;
}

/** Whether the row offers Dismiss. Issues clear when fixed and Next steps when done. */
export function isDismissable(entry: SetupListEntry): boolean {
	return entry.tier === 'Suggestion';
}

/** The server's key as a string, to key the list and the once-per-key `setup_issue_shown` event. */
export function entryId(entry: SetupListEntry): string {
	return JSON.stringify(entry.key);
}
