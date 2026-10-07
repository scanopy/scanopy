import { entityRef, type CardFieldItem } from '$lib/shared/components/data/types';
import { entities } from '$lib/shared/stores/metadata';
import { formatDate } from '$lib/shared/utils/formatting';
import { discovery_runLabel } from '$lib/paraglide/messages';
import type { Discovery } from './types/base';

/**
 * A discovery run as a single navigable chip, labelled with the scan's name and the run's date.
 *
 * Entities record the runs that first and last found them (`first_discovery_id`,
 * `last_discovery_id`); building the chip here keeps those columns identical on every tab.
 * Runs of one scan share its name, so the date is what tells them apart.
 */
export function discoveryRunItems(
	discoveryId: string | null | undefined,
	runs: Discovery[]
): CardFieldItem[] {
	if (!discoveryId) return [];

	const run = runs.find((d) => d.id === discoveryId);
	if (!run) return [];

	return [
		{
			id: run.id,
			label: discovery_runLabel({ name: run.name, date: formatDate(run.created_at) }),
			color: entities.getColorHelper('Discovery').color,
			entityRef: entityRef('Discovery', run.id, run)
		}
	];
}

/** The distinct run ids a list of entities was first or last found by, for `useDiscoveriesByIds`. */
export function discoveryRunIds(
	items: { first_discovery_id?: string | null; last_discovery_id?: string | null }[]
): string[] {
	const ids = new Set<string>();
	for (const item of items) {
		if (item.first_discovery_id) ids.add(item.first_discovery_id);
		if (item.last_discovery_id) ids.add(item.last_discovery_id);
	}
	return [...ids];
}
