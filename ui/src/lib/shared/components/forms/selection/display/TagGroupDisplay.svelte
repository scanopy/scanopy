<script lang="ts" module>
	import { entities } from '#lib/shared/stores/metadata.ts';
	import { tagGroupIcon, tagGroupLabel } from '#lib/features/tags/groups.ts';
	import { common_none } from '#lib/paraglide/messages.js';

	/** A tag group as a row: its name and description, drawn with the system group's icon when it is one. */
	export const TagGroupDisplay: EntityDisplayComponent<TagGroup, object> = {
		getId: (group) => group.id,
		getLabel: (group) => tagGroupLabel(group) ?? group.name,
		getDescription: (group) => group.description ?? '',
		getIcon: (group) => tagGroupIcon(group) ?? entities.getIconComponent('TagGroup'),
		getIconColor: () => entities.getColorHelper('TagGroup').icon,
		getCategory: () => null
	};

	/** The id of the None row in a tag's group field. */
	export const NO_TAG_GROUP = 'none';

	/** A tag's group field: each group, plus `null` for the None row that clears it. */
	export const TagGroupOptionDisplay: EntityDisplayComponent<TagGroup | null, object> = {
		getId: (group) => group?.id ?? NO_TAG_GROUP,
		getLabel: (group) => (group ? TagGroupDisplay.getLabel(group) : common_none()),
		getDescription: (group, context) =>
			group ? (TagGroupDisplay.getDescription?.(group, context) ?? '') : '',
		getIcon: (group, context) =>
			group ? (TagGroupDisplay.getIcon?.(group, context) ?? null) : null,
		getIconColor: (group, context) =>
			group ? (TagGroupDisplay.getIconColor?.(group, context) ?? null) : null,
		getCategory: () => null
	};
</script>

<script lang="ts">
	import ListSelectItem from '#lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { EntityDisplayComponent } from '../types';
	import type { TagGroup } from '#lib/features/tags/types/base.ts';

	interface Props {
		item: TagGroup;
		context?: object;
	}

	let { item, context = {} }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={TagGroupDisplay} />
