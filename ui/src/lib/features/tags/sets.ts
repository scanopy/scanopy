import type { components } from '$lib/api/schema';
import type { IconComponent } from '$lib/shared/utils/types';
import { concepts } from '$lib/shared/stores/metadata';
import { createIconComponent } from '$lib/shared/utils/styling';
import { common_application } from '$lib/paraglide/messages';
import type { Tag } from './types/base';

export type ExclusiveSet = components['schemas']['ExclusiveSet'];

/** Whether this tag drives the application view. */
export function isApplicationTag(tag: Pick<Tag, 'exclusive_set'> | null | undefined): boolean {
	return tag?.exclusive_set?.type === 'Application';
}

/** Whether two sets are the same set. `null` is no set, which is never shared. */
export function sameSet(a: ExclusiveSet | null | undefined, b: ExclusiveSet | null | undefined) {
	if (!a || !b || a.type !== b.type) return false;
	return a.type === 'Application' || (b.type === 'Group' && a.name === b.name);
}

/** How a set is named in the UI. */
export function exclusiveSetLabel(set: ExclusiveSet | null | undefined): string | null {
	if (!set) return null;
	return set.type === 'Application' ? common_application() : set.name;
}

/** The named sets in use, from the tags that carry them. A set exists while a tag carries it. */
export function groupNames(tags: Pick<Tag, 'exclusive_set'>[]): string[] {
	const names = new Set<string>();
	for (const tag of tags) {
		if (tag.exclusive_set?.type === 'Group') names.add(tag.exclusive_set.name);
	}
	return [...names].sort((a, b) => a.localeCompare(b));
}

/**
 * The icon a tag is drawn with: the application icon for application tags, the tag's own icon
 * otherwise, or none.
 */
export function tagIcon(tag: Pick<Tag, 'exclusive_set' | 'icon'> | null | undefined) {
	if (!tag) return null;
	if (isApplicationTag(tag)) return concepts.getIconComponent('Application');
	return tag.icon ? (createIconComponent(tag.icon) as IconComponent) : null;
}

/**
 * The tag ids left after adding `tagId`: the other tags of its exclusive set drop out, which is
 * what the server does on assignment.
 */
export function withTagAdded(selectedIds: string[], tagId: string, tags: Tag[]): string[] {
	const added = tags.find((t) => t.id === tagId);
	const kept = selectedIds.filter((id) => {
		if (id === tagId) return false;
		const held = tags.find((t) => t.id === id);
		return !sameSet(held?.exclusive_set, added?.exclusive_set);
	});
	return [...kept, tagId];
}
