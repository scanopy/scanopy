import { defineRegistryFields } from '#lib/shared/entity-fields/table.ts';
import type { EntityLookup } from '#lib/shared/entity-fields/index.ts';
import { NO_VALUE, valuesOrNoneCodec } from '#lib/shared/components/data/controls/listQuery.ts';
import type { components, operations } from '#lib/api/schema';
import { entities } from '#lib/shared/stores/metadata.ts';
import { isSystemGroup, tagGroupIcon, tagGroupLabel, tagsInGroup } from '../groups';
import { tagItems } from '../columns';
import type { Tag, TagGroup } from '../types/base';
import {
	common_custom,
	common_description,
	common_name,
	common_system,
	common_tags,
	common_type,
	tags_appliesTo,
	tags_appliesToEvery,
	tags_orderByName,
	tags_orderSet
} from '#lib/paraglide/messages.js';

type TagGroupOrderField = components['schemas']['TagGroupOrderField'];
/** The tag groups list endpoint's query params, which the filters are sent and linked as. */
type TagGroupListParam = keyof NonNullable<operations['list_tag_groups']['parameters']['query']>;

/** What the tag group columns read from the tab, each as a getter so values stay live. */
export interface TagGroupColumnsContext {
	/** Related entities the registry's tag group fields read; none beyond the group itself. */
	lookup: () => EntityLookup;
	tags: () => Tag[];
	groups: () => TagGroup[];
}

/** The scope of a group that applies to every entity type, which stores none. */
function scopeName(type: string): string {
	if (type === NO_VALUE) return tags_appliesToEvery();
	const entityType = entities.getItems().find((item) => item.id === type);
	return entityType ? (entities.getMetadata(entityType.id)?.entity_name_plural ?? type) : type;
}

function scopeNames(group: TagGroup): string[] {
	return (group.entity_types ?? [NO_VALUE]).map(scopeName);
}

/** Every TagGroupOrderField value is listed; the server sorts, groups and filters each. */
export function tagGroupColumns(ctx: TagGroupColumnsContext) {
	return defineRegistryFields<TagGroup, TagGroupOrderField, TagGroupListParam>(
		'TagGroup',
		ctx.lookup,
		{
			name: {
				label: common_name(),
				type: 'string',
				searchable: true,
				// One header per group.
				groupable: false,
				getValue: (group) => tagGroupLabel(group),
				display: {
					primary: true,
					width: 220,
					getItems: (group) => [
						{
							id: group.id,
							label: tagGroupLabel(group) ?? group.name,
							icon: tagGroupIcon(group) ?? undefined
						}
					]
				}
			},
			group_type: {
				label: common_type(),
				type: 'string',
				filterable: true,
				serverFiltered: true,
				queryParam: 'system',
				filterOptions: ['true', 'false'],
				getValue: (group) => (isSystemGroup(group) ? common_system() : common_custom()),
				getFilterValue: (group) => String(isSystemGroup(group)),
				getGroupValue: (group) => String(isSystemGroup(group)),
				filterLabel: (value) => (value === 'true' ? common_system() : common_custom())
			},
			ordered: {
				type: 'string',
				filterable: true,
				serverFiltered: true,
				filterOptions: ['true', 'false'],
				getValue: (group) => (group.ordered ? tags_orderSet() : tags_orderByName()),
				getFilterValue: (group) => String(group.ordered),
				getGroupValue: (group) => String(group.ordered),
				filterLabel: (value) => (value === 'true' ? tags_orderSet() : tags_orderByName())
			},
			created_at: {
				type: 'date',
				groupable: false,
				display: { hiddenByDefault: true }
			},
			updated_at: {
				type: 'date',
				groupable: false,
				display: { hiddenByDefault: true }
			}
		},
		[
			{
				key: 'description',
				label: common_description(),
				type: 'string',
				getValue: (group) => group.description,
				display: { wrap: true }
			},
			{
				key: 'entity_types',
				label: tags_appliesTo(),
				type: 'array',
				filterable: true,
				serverFiltered: true,
				// Every entity type, and "every type" for a group that applies to all of them.
				filterOptions: [NO_VALUE, ...entities.getItems().map((item) => item.id)],
				filterCodec: valuesOrNoneCodec<TagGroupListParam>(
					'entity_types',
					'include_every_entity_type'
				),
				getValue: (group) => scopeNames(group),
				getFilterValue: (group) => group.entity_types ?? [NO_VALUE],
				filterLabel: scopeName
			},
			{
				key: 'tags',
				label: common_tags(),
				type: 'array',
				filterable: true,
				getValue: (group) => tagsInGroup(ctx.tags(), group.id).map((tag) => tag.name),
				getFilterValue: (group) => tagsInGroup(ctx.tags(), group.id).map((tag) => tag.id),
				display: {
					getItems: (group) =>
						tagItems(
							tagsInGroup(ctx.tags(), group.id).map((tag) => tag.id),
							ctx.tags(),
							ctx.groups()
						)
				}
			}
		]
	);
}
