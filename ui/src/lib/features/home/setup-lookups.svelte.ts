import { useDaemonsQuery } from '#lib/features/daemons/queries.ts';
import { useHostsByIds } from '#lib/features/hosts/queries.ts';
import { useSitesQuery } from '#lib/features/sites/queries.ts';
import { useCredentialsQuery } from '#lib/features/credentials/queries.ts';
import { hostIdsOf } from '#lib/features/discovery/utils/warnings.ts';
import type { SetupLookups } from './setup-row-view';
import type { SetupListEntry } from './setup-list';

/**
 * The loaded entities Setup list rows name and open. Hosts load for the rows given: an uncovered
 * integration's hosts, and the hosts a scan warning concerns, one chip each. Call during
 * component init.
 */
export function setupLookups(entries: () => SetupListEntry[]): SetupLookups {
	const daemonsQuery = useDaemonsQuery();
	const credentialsQuery = useCredentialsQuery();
	const sitesQuery = useSitesQuery();
	const hostIds = $derived([
		// eslint-disable-next-line svelte/prefer-svelte-reactivity
		...new Set(
			entries().flatMap(({ item }) =>
				item.type === 'UncoveredIntegration'
					? item.host_ids
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
