import { entityRef, type CardFieldItem } from '$lib/shared/components/data/types';
import { entities } from '$lib/shared/stores/metadata';
import type { Daemon } from './types/base';

/**
 * A daemon as a single navigable chip.
 *
 * Several entities reference a `daemon_id` and surface it as a column; building
 * the chip here keeps the colour and the entity link identical wherever it
 * appears.
 */
export function daemonItems(
	daemonId: string | null | undefined,
	daemons: Daemon[]
): CardFieldItem[] {
	if (!daemonId) return [];

	const daemon = daemons.find((d) => d.id === daemonId);
	if (!daemon) return [];

	return [
		{
			id: daemon.id,
			label: daemon.name,
			color: entities.getColorHelper('Daemon').color,
			entityRef: entityRef('Daemon', daemon.id, daemon)
		}
	];
}
