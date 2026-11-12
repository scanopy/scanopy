<script lang="ts" module>
	import { entities } from '#lib/shared/stores/metadata.ts';
	import { createColorHelper } from '#lib/shared/utils/styling.ts';
	import { applicationGroupId, tagDescription, tagIcon } from '#lib/features/tags/groups.ts';
	import { queryClient, queryKeys } from '#lib/api/query-client.ts';
	import type { TagGroup } from '#lib/features/tags/types/base.ts';

	/** The organization's tag groups, as the tag-group query last loaded them. */
	const cachedTagGroups = (): TagGroup[] =>
		queryClient.getQueryData<TagGroup[]>(queryKeys.tagGroups.all) ?? [];

	/** A tag as a row: drawn with its own icon and colour, its group and description beneath. */
	export const TagDisplay: EntityDisplayComponent<Tag, object> = {
		getId: (tag) => tag.id,
		getLabel: (tag) => tag.name,
		getDescription: (tag) => tagDescription(tag, cachedTagGroups()),
		getIcon: (tag) =>
			tagIcon(tag, applicationGroupId(cachedTagGroups())) ?? entities.getIconComponent('Tag'),
		getIconColor: (tag) => createColorHelper(tag.color).icon,
		getCategory: () => null
	};
</script>

<script lang="ts">
	import ListSelectItem from '#lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { EntityDisplayComponent } from '../types';
	import type { Tag } from '#lib/features/tags/types/base.ts';

	interface Props {
		item: Tag;
		context?: object;
	}

	let { item, context = {} }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={TagDisplay} />
