<script lang="ts" context="module">
	import { entities } from '$lib/shared/stores/metadata';

	export const NetworkDisplay: EntityDisplayComponent<Network, object> = {
		getId: (network: Network) => network.id,
		getLabel: (network: Network) => network.name,
		getIcon: () => entities.getIconComponent('Network'),
		getIconColor: () => entities.getColorHelper('Network').icon,
		getCategory: () => null
	};
</script>

<script lang="ts">
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { EntityDisplayComponent } from '../types';
	import type { Network } from '$lib/features/networks/types';

	export let item: Network;
	export let context: object = {};
</script>

<ListSelectItem {item} {context} displayComponent={NetworkDisplay} />
