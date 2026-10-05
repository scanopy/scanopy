<script lang="ts" context="module">
	import { entities } from '$lib/shared/stores/metadata';

	export const SiteDisplay: EntityDisplayComponent<Site, object> = {
		getId: (site: Site) => site.id,
		getLabel: (site: Site) => site.name,
		getIcon: () => entities.getIconComponent('Site'),
		getIconColor: () => entities.getColorHelper('Site').icon,
		getCategory: () => null
	};
</script>

<script lang="ts">
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { EntityDisplayComponent } from '../types';
	import type { Site } from '$lib/features/sites/types';

	export let item: Site;
	export let context: object = {};
</script>

<ListSelectItem {item} {context} displayComponent={SiteDisplay} />
