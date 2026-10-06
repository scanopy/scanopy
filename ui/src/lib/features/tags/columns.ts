import type { LabelledCardFieldItem } from '$lib/shared/components/data/types';
import { tagIcon, tagTooltip } from './groups';
import type { Tag } from './types/base';

/**
 * Resolve an entity's tag ids into renderable chips.
 *
 * Entities store tags as ids, so every surface that shows them has to join
 * against the tag list. Doing it here means the table cell and the filter
 * describe a tag the same way, and no component has to open a tags
 * query of its own just to render a label.
 */
export function tagItems(tagIds: string[], tags: Tag[]): LabelledCardFieldItem[] {
	return tagIds
		.map((id) => tags.find((tag) => tag.id === id))
		.filter((tag): tag is Tag => Boolean(tag))
		.map((tag) => ({
			id: tag.id,
			label: tag.name,
			color: tag.color,
			icon: tagIcon(tag) ?? undefined,
			title: tagTooltip(tag) || undefined
		}));
}

/** The tag names an entity carries, for search and filter matching. */
export function tagNames(tagIds: string[], tags: Tag[]): string[] {
	return tagItems(tagIds, tags).map((item) => item.label);
}
