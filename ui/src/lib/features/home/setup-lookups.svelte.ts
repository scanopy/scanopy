import { useDaemonsQuery } from '#lib/features/daemons/queries.ts';
import { useHostsByIds } from '#lib/features/hosts/queries.ts';
import { useSitesQuery } from '#lib/features/sites/queries.ts';
import { useCredentialsQuery } from '#lib/features/credentials/queries.ts';
import { hostIdsOf } from '#lib/features/discovery/utils/warnings.ts';
import { trackEvent } from '#lib/shared/utils/analytics.ts';
import type { SetupLookups } from './setup-row-view';
import { entryId, type SetupListEntry } from './setup-list';

/**
 * The loaded entities Setup list rows name and open. Hosts load for the rows given: an uncovered
 * integration's first host, and the hosts a scan warning concerns. Call during component init.
 */
export function setupLookups(entries: () => SetupListEntry[]): SetupLookups {
	const daemonsQuery = useDaemonsQuery();
	const credentialsQuery = useCredentialsQuery();
	const sitesQuery = useSitesQuery();
	const hostIds = $derived([
		...new Set(
			entries().flatMap(({ item }) =>
				item.type === 'UncoveredIntegration'
					? item.host_ids.slice(0, 1)
					: item.type === 'RunWarnings'
						? hostIdsOf(item)
						: []
			)
		)
	]);
	const hostsQuery = useHostsByIds(() => hostIds);
	return {
		daemon: (id) => (daemonsQuery.data ?? []).find((d) => d.id === id),
		host: (id) => (hostsQuery.data ?? []).find((h) => h.id === id),
		site: (id) => (sitesQuery.data ?? []).find((s) => s.id === id),
		credential: (id) => (credentialsQuery.data ?? []).find((c) => c.id === id)
	};
}

/** Keys `setup_issue_shown` has fired for since the page loaded. */
const shownKeys = new Set<string>();

/**
 * Fire `setup_issue_shown` once per page load for each of `entries` the trial sweep reports: an
 * Issue, or a first step still Open.
 */
export function reportShown(entries: SetupListEntry[]) {
	for (const entry of entries) {
		if (!entry.alerts_when_stuck) continue;
		const key = entryId(entry);
		if (shownKeys.has(key)) continue;
		shownKeys.add(key);
		trackEvent('setup_issue_shown', { item_type: entry.item.type, tier: entry.tier, key });
	}
}
