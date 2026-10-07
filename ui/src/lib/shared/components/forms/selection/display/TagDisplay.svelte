<script lang="ts" module>
	import { entities } from '$lib/shared/stores/metadata';
	import { createColorHelper } from '$lib/shared/utils/styling';
	import { tagIcon, tagTooltip } from '$lib/features/tags/groups';

	/** A tag as a row: drawn with its own icon and colour, its group and description beneath. */
	export const TagDisplay: EntityDisplayComponent<Tag, object> = {
		getId: (tag) => tag.id,
		getLabel: (tag) => tag.name,
		getDescription: (tag) => tagTooltip(tag).replaceAll('\n', ' · '),
		getIcon: (tag) => tagIcon(tag) ?? entities.getIconComponent('Tag'),
		getIconColor: (tag) => createColorHelper(tag.color).icon,
		getCategory: () => null
	};
</script>

<script lang="ts">
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { EntityDisplayComponent } from '../types';
	import type { Tag } from '$lib/features/tags/types/base';

	interface Props {
		item: Tag;
		context?: object;
	}

	let { item, context = {} }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={TagDisplay} />
