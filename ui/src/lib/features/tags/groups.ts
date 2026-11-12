import type { components } from '#lib/api/schema';
import type { IconComponent } from '#lib/shared/utils/types.ts';
import { concepts } from '#lib/shared/stores/metadata.ts';
import { createIconComponent } from '#lib/shared/utils/styling.ts';
import {
	common_application,
	common_status,
	common_visibility,
	tags_tagGroup,
	tags_tagGroupTooltip
} from '#lib/paraglide/messages.js';
import type { TooltipContent, TooltipSection } from '#lib/shared/actions/tooltip.ts';
import type { Tag, TagGroup, TagGroupType } from './types/base';

export type { TagGroup };

type EntityDiscriminants = components['schemas']['EntityDiscriminants'];

/** A group type Scanopy defines, as opposed to one an organization names. */
export type SystemTagGroupType = Exclude<TagGroupType, 'Custom'>;

/**
 * How each system group reads in the UI. Keyed by every system group type, so a system group the
 * backend adds fails the type check until it is described here. Its icon is the concept of the same
 * name.
 */
const SYSTEM_GROUPS: Record<SystemTagGroupType, { label: () => string }> = {
	Application: { label: () => common_application() },
	Status: { label: () => common_status() },
	Visibility: { label: () => common_visibility() }
};

/** Whether Scanopy defines this group: it can't be renamed or deleted. */
export function isSystemGroup(group: Pick<TagGroup, 'group_type'> | null | undefined): boolean {
	return !!group && group.group_type !== 'Custom';
}

/** How a tag group is named in the UI. */
export function tagGroupLabel(group: TagGroup | null | undefined): string | null {
	if (!group) return null;
	return group.group_type === 'Custom' ? group.name : SYSTEM_GROUPS[group.group_type].label();
}

/** The icon a system group is drawn with; `null` for a group the organization named. */
export function tagGroupIcon(group: TagGroup | null | undefined) {
	return group && group.group_type !== 'Custom'
		? concepts.getIconComponent(group.group_type)
		: null;
}

/** The group a tag belongs to, if any. */
export function groupOf(
	tag: Pick<Tag, 'tag_group_id'> | null | undefined,
	groups: TagGroup[]
): TagGroup | null {
	if (!tag?.tag_group_id) return null;
	return groups.find((group) => group.id === tag.tag_group_id) ?? null;
}

/** The group named `name`, ignoring case and matching a system group by its label too. */
export function groupNamed(groups: TagGroup[], name: string): TagGroup | null {
	const needle = name.trim().toLowerCase();
	return (
		groups.find(
			(group) =>
				group.name.toLowerCase() === needle || tagGroupLabel(group)?.toLowerCase() === needle
		) ?? null
	);
}

/** The id of the organization's Application group, whose tags place entities in applications. */
export function applicationGroupId(groups: TagGroup[]): string | null {
	return groups.find((group) => group.group_type === 'Application')?.id ?? null;
}

/** Whether this tag drives the application view. */
export function isApplicationTag(
	tag: Pick<Tag, 'tag_group_id'> | null | undefined,
	applicationGroup: string | null | undefined
): boolean {
	return !!applicationGroup && tag?.tag_group_id === applicationGroup;
}

/** Whether two tags are in the same group. A tag in no group shares it with nothing. */
export function sameGroup(
	a: Pick<Tag, 'tag_group_id'> | null | undefined,
	b: Pick<Tag, 'tag_group_id'> | null | undefined
): boolean {
	return !!a?.tag_group_id && a.tag_group_id === b?.tag_group_id;
}

/** Whether a group's tags may be applied to `entityType`. `null` scope allows every type. */
export function groupAllows(
	group: Pick<TagGroup, 'entity_types'> | null | undefined,
	entityType: EntityDiscriminants
): boolean {
	return !group?.entity_types || group.entity_types.includes(entityType);
}

/** Whether `tag` may be applied to `entityType`: its group, if it has one, allows that type. */
export function tagAllowedOn(
	tag: Pick<Tag, 'tag_group_id'>,
	groups: TagGroup[],
	entityType: EntityDiscriminants
): boolean {
	return groupAllows(groupOf(tag, groups), entityType);
}

/** The groups whose tags may be applied to `entityType`, in the server's order. */
export function groupsFor(groups: TagGroup[], entityType: EntityDiscriminants): TagGroup[] {
	return groups.filter((group) => groupAllows(group, entityType));
}

/** Orders tags the way their group lists them: by position, then by name. */
export function compareTagOrder(
	a: Pick<Tag, 'position' | 'name'>,
	b: Pick<Tag, 'position' | 'name'>
): number {
	const pa = a.position ?? Number.MAX_SAFE_INTEGER;
	const pb = b.position ?? Number.MAX_SAFE_INTEGER;
	return pa - pb || a.name.localeCompare(b.name);
}

/**
 * Orders tags by group, in the server's group order, then by each group's own order. Tags in no
 * group come last, by name.
 */
export function compareTagsByGroup(groups: TagGroup[]) {
	const rank = new Map(groups.map((group, index) => [group.id, index]));
	return (a: Tag, b: Tag): number => {
		const ra = a.tag_group_id ? (rank.get(a.tag_group_id) ?? groups.length) : groups.length + 1;
		const rb = b.tag_group_id ? (rank.get(b.tag_group_id) ?? groups.length) : groups.length + 1;
		return ra - rb || compareTagOrder(a, b);
	};
}

/**
 * The key a server-paged tags list groups a tag under by its tag group: `0` for a system group or
 * `1`, the group's type, its lowercased name, then its id. It matches the server's
 * `TAG_GROUP_KEY_SQL`, which orders groups the way the tag groups list does, so a group finds its
 * total in `group_counts`.
 */
export function tagGroupServerKey(group: TagGroup | null | undefined): string | null {
	if (!group) return null;
	return `${isSystemGroup(group) ? 0 : 1}:${group.group_type}:${group.name.toLowerCase()}:${group.id}`;
}

/** A group's tags, in its order. */
export function tagsInGroup(tags: Tag[], groupId: string): Tag[] {
	return tags.filter((tag) => tag.tag_group_id === groupId).sort(compareTagOrder);
}

/**
 * A tag chip's tooltip: its tag group as a labelled row, then its description. Null when the tag
 * has neither, so the chip shows no tooltip.
 */
export function tagTooltip(
	tag: Pick<Tag, 'tag_group_id' | 'description'> | null | undefined,
	groups: TagGroup[]
): TooltipContent | null {
	const group = tagGroupLabel(groupOf(tag, groups));
	const description = tag?.description?.trim();
	const sections: TooltipSection[] = [];
	if (group) sections.push({ rows: [{ label: tags_tagGroup(), value: group }] });
	if (description) sections.push({ text: description });
	return sections.length > 0 ? { sections } : null;
}

/**
 * A tag's tag group and description as one line of text, for a select option's description.
 * Empty when the tag has neither.
 */
export function tagDescription(
	tag: Pick<Tag, 'tag_group_id' | 'description'> | null | undefined,
	groups: TagGroup[]
): string {
	const group = tagGroupLabel(groupOf(tag, groups));
	const description = tag?.description?.trim();
	return [group ? tags_tagGroupTooltip({ group }) : null, description || null]
		.filter((part): part is string => part !== null)
		.join(' · ');
}

/**
 * The icon a tag is drawn with: the application icon for application tags, the tag's own icon
 * otherwise, or none.
 */
export function tagIcon(
	tag: Pick<Tag, 'tag_group_id' | 'icon'> | null | undefined,
	applicationGroup: string | null | undefined
) {
	if (!tag) return null;
	if (isApplicationTag(tag, applicationGroup)) return concepts.getIconComponent('Application');
	return tag.icon ? (createIconComponent(tag.icon) as IconComponent) : null;
}

/**
 * The tag ids left after adding `tagId`: the other tags of its tag group drop out, which is what
 * the server does on assignment.
 */
export function withTagAdded(selectedIds: string[], tagId: string, tags: Tag[]): string[] {
	const displaced = tagsDisplacedBy(selectedIds, tagId, tags, true);
	return [...selectedIds.filter((id) => id !== tagId && !displaced.includes(id)), tagId];
}

/**
 * The held tag ids that adding `tagId` drops. With `onePerGroup` the selection is one entity's
 * tags, which hold one tag of a group, so the other tags of its group drop. Without it (a list of
 * a group's members, as in the application wizard) nothing drops.
 */
export function tagsDisplacedBy(
	selectedIds: string[],
	tagId: string,
	tags: Tag[],
	onePerGroup: boolean
): string[] {
	if (!onePerGroup) return [];
	const added = tags.find((t) => t.id === tagId);
	return selectedIds.filter(
		(id) =>
			id !== tagId &&
			sameGroup(
				tags.find((t) => t.id === id),
				added
			)
	);
}
