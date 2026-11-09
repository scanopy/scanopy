import type { components } from '$lib/api/schema';
import { setupItems, setupItemTiers } from '$lib/shared/stores/metadata';

export type SetupItem = components['schemas']['SetupItem'];
export type SetupItemType = components['schemas']['SetupItemDiscriminants'];
/** A `setup_item_tiers` metadata id. The tier is server metadata, so the API types don't carry it. */
export type SetupItemTier = string;

/** One tier of the Setup list and its rows, in the order the server listed them. */
export interface SetupTierGroup {
	tier: SetupItemTier;
	items: SetupItem[];
}

/** The tier an item sits in, from the item type's metadata. */
export function tierOf(item: Pick<SetupItem, 'type'>): SetupItemTier {
	return setupItems.getCategory(item.type) as SetupItemTier;
}

/** The items grouped by tier, in the tiers' display order. A tier with no items is left out. */
export function groupByTier(items: SetupItem[]): SetupTierGroup[] {
	return (setupItemTiers.getItems() ?? [])
		.map((t) => ({
			tier: t.id as SetupItemTier,
			items: items.filter((item) => tierOf(item) === t.id)
		}))
		.filter((group) => group.items.length > 0);
}

/** The Issues among `items`: the Home nav badge. */
export function setupIssueCount(items: SetupItem[]): number {
	return items.filter((item) => tierOf(item) === 'Issue').length;
}

/** Whether the row offers Dismiss. Issues clear when fixed and Next steps when done. */
export function isDismissable(item: SetupItem): boolean {
	return tierOf(item) === 'Suggestion';
}

/**
 * The row's identity: the item type and the entity it concerns, so two daemons' health issues
 * are two rows. Keys the list and the once-per-key `setup_issue_shown` event.
 */
export function setupItemKey(item: SetupItem): string {
	switch (item.type) {
		case 'DaemonHealth':
			return `${item.type}:${item.daemon_id}`;
		case 'UncoveredIntegration':
			return `${item.type}:${item.integration}`;
		case 'TargetlessCredential':
			return `${item.type}:${item.credential_id}`;
		case 'SiteWithoutDaemon':
			return `${item.type}:${item.site_id}`;
		case 'RunWarnings':
			return `${item.type}:${item.daemon_id}:${item.code}`;
		default:
			return item.type;
	}
}
