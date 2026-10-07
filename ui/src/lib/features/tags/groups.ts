import type { components } from '$lib/api/schema';
import type { IconComponent } from '$lib/shared/utils/types';
import { concepts } from '$lib/shared/stores/metadata';
import { createIconComponent } from '$lib/shared/utils/styling';
import { common_application, tags_tagGroupTooltip } from '$lib/paraglide/messages';
import type { Tag } from './types/base';

export type TagGroup = components['schemas']['TagGroup'];

/** Whether this tag drives the application view. */
export function isApplicationTag(tag: Pick<Tag, 'tag_group'> | null | undefined): boolean {
	return tag?.tag_group?.type === 'Application';
}

/** Whether two tag groups are the same group. `null` is no group, which is never shared. */
export function sameGroup(a: TagGroup | null | undefined, b: TagGroup | null | undefined) {
	if (!a || !b || a.type !== b.type) return false;
	return a.type === 'Application' || (b.type === 'Named' && a.name === b.name);
}

/** How a tag group is named in the UI. */
export function tagGroupLabel(group: TagGroup | null | undefined): string | null {
	if (!group) return null;
	return group.type === 'Application' ? common_application() : group.name;
}

/**
 * A tag chip's hover text: its tag group, then its description, one per line. Empty when the tag
 * has neither, so the chip shows no tooltip.
 */
export function tagTooltip(tag: Pick<Tag, 'tag_group' | 'description'> | null | undefined): string {
	const group = tagGroupLabel(tag?.tag_group);
	const description = tag?.description?.trim();
	return [group ? tags_tagGroupTooltip({ group }) : null, description || null]
		.filter((line): line is string => line !== null)
		.join('\n');
}

/** The named groups in use, from the tags that carry them. A group exists while a tag carries it. */
export function groupNames(tags: Pick<Tag, 'tag_group'>[]): string[] {
	const names = new Set<string>();
	for (const tag of tags) {
		if (tag.tag_group?.type === 'Named') names.add(tag.tag_group.name);
	}
	return [...names].sort((a, b) => a.localeCompare(b));
}

/**
 * The icon a tag is drawn with: the application icon for application tags, the tag's own icon
 * otherwise, or none.
 */
export function tagIcon(tag: Pick<Tag, 'tag_group' | 'icon'> | null | undefined) {
	if (!tag) return null;
	if (isApplicationTag(tag)) return concepts.getIconComponent('Application');
	return tag.icon ? (createIconComponent(tag.icon) as IconComponent) : null;
}

/**
 * The tag ids left after adding `tagId`: the other tags of its tag group drop out, which is what
 * the server does on assignment.
 */
export function withTagAdded(selectedIds: string[], tagId: string, tags: Tag[]): string[] {
	const added = tags.find((t) => t.id === tagId);
	const kept = selectedIds.filter((id) => {
		if (id === tagId) return false;
		const held = tags.find((t) => t.id === id);
		return !sameGroup(held?.tag_group, added?.tag_group);
	});
	return [...kept, tagId];
}
