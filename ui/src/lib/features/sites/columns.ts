import { entityRef, type LabelledCardFieldItem } from '$lib/shared/components/data/types';
import { entities } from '$lib/shared/stores/metadata';
import type { Site } from './types';

/**
 * Sites as navigable chips.
 *
 * Takes one id or many, because entities reference a site either way
 * (`site_id` on most, `site_ids` on user API keys). Building the chip
 * here keeps the colour and the entity link identical wherever a site
 * appears.
 */
export function siteItems(
	siteIds: string | string[] | null | undefined,
	sites: Site[]
): LabelledCardFieldItem[] {
	if (!siteIds) return [];
	const ids = Array.isArray(siteIds) ? siteIds : [siteIds];

	return ids
		.map((id) => sites.find((n) => n.id === id))
		.filter((site): site is Site => Boolean(site))
		.map((site) => ({
			id: site.id,
			label: site.name,
			color: entities.getColorHelper('Site').color,
			entityRef: entityRef('Site', site.id, site)
		}));
}
