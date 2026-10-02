/**
 * How an entity came to exist, as a column: discovered, imported, inferred from a neighbour, or
 * created by hand. Read from the stamped `source.type`, and labelled from the `entity_sources`
 * fixture, so every entity list names a source the same way.
 */

import type { components } from '$lib/api/schema';
import type { CardFieldItem } from '$lib/shared/components/data/types';
import { entitySources } from '$lib/shared/stores/metadata';

type EntitySource = components['schemas']['EntitySource'];

/** The source as one chip, with its icon, colour and description. */
export function entitySourceItems(source: EntitySource): CardFieldItem[] {
	return [
		{
			id: source.type,
			label: entitySources.getName(source.type),
			color: entitySources.getColorHelper(source.type).color,
			icon: entitySources.getIconComponent(source.type),
			title: entitySources.getDescription(source.type)
		}
	];
}
