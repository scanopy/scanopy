import { defineRegistryFields } from '#lib/shared/entity-fields/table.ts';
import type { EntityLookup } from '#lib/shared/entity-fields/index.ts';
import type { components, operations } from '#lib/api/schema';
import { fieldValueOptions, type FieldValueCount } from '#lib/shared/api/field-values.ts';
import { groupOf, tagGroupLabel, tagGroupServerKey, tagIcon } from '../groups';
import { tagItems } from '../columns';
import type { Tag, TagGroup } from '../types/base';
import { tags_tagGroup } from '#lib/paraglide/messages.js';

type TagOrderField = components['schemas']['TagOrderField'];
/** The tags list endpoint's query params, which the filters are sent and linked as. */
type TagListParam = keyof NonNullable<operations['list_tags']['parameters']['query']>;

/** What the tag columns read from the tab, each as a getter so values stay live. */
export interface TagColumnsContext {
	lookup: () => EntityLookup;
	tagGroups: () => TagGroup[];
	applicationGroup: () => string | null;
	colorValues: () => FieldValueCount[] | undefined;
	iconValues: () => FieldValueCount[] | undefined;
}

/**
 * Columns from the entity field registry; this tab adds how each one searches, filters and
 * sorts. Every TagOrderField value must be listed.
 */
export function tagColumns(ctx: TagColumnsContext) {
	return defineRegistryFields<Tag, TagOrderField, TagListParam>(
		'Tag',
		ctx.lookup,
		{
			// Identity field: grouping by it would render a header per tag. Drawn as the tag itself,
			// with its icon and colour, as it appears on every entity carrying it.
			name: {
				searchable: true,
				groupable: false,
				display: {
					primary: true,
					width: 220,
					getItems: (tag) => tagItems([tag.id], [tag], ctx.tagGroups())
				}
			},
			color: {
				filterable: true,
				serverFiltered: true,
				queryParam: 'colors',
				// The colours some tag has.
				filterOptions: fieldValueOptions(ctx.colorValues()),
				// A colour name is worth showing in its own colour. Hidden by default: the name column
				// already draws it, so this column is there to group and filter by.
				display: {
					hiddenByDefault: true,
					getItems: (tag) => [
						{
							id: tag.color,
							label: tag.color.charAt(0).toUpperCase() + tag.color.slice(1),
							color: tag.color
						}
					]
				}
			},
			tag_group: {
				label: tags_tagGroup(),
				filterable: true,
				serverFiltered: true,
				queryParam: 'tag_group_ids',
				filterOptions: ctx.tagGroups().map((group) => group.id),
				// A system group reads under its translated name.
				getValue: (tag) => tagGroupLabel(groupOf(tag, ctx.tagGroups())),
				getFilterValue: (tag) => tag.tag_group_id ?? null,
				filterLabel: (id) => tagGroupLabel(ctx.tagGroups().find((group) => group.id === id)) ?? id,
				// The server groups in the tag groups list's order, under this key.
				getGroupValue: (tag) => tagGroupServerKey(groupOf(tag, ctx.tagGroups()))
			},
			// Hidden by default for the same reason as colour: the name column draws the icon.
			icon: {
				filterable: true,
				serverFiltered: true,
				queryParam: 'icons',
				// The icons some tag has.
				filterOptions: fieldValueOptions(ctx.iconValues()),
				getGroupValue: (tag) => tag.icon ?? null,
				display: {
					hiddenByDefault: true,
					getItems: (tag) => {
						const icon = tagIcon(tag, ctx.applicationGroup());
						return icon && tag.icon ? [{ id: tag.icon, label: tag.icon, icon }] : [];
					}
				}
			},
			created_at: { type: 'date', display: { hiddenByDefault: true } },
			updated_at: { type: 'date', display: { hiddenByDefault: true } }
		},
		[{ key: 'description' }]
	);
}
