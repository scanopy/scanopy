<script lang="ts" module>
	import { entities } from '$lib/shared/stores/metadata';
	import { queryClient, queryKeys } from '$lib/api/query-client';
	import type { Network } from '$lib/features/networks/types';
	import { common_unknownNetwork } from '$lib/paraglide/messages';

	export const TopologyDisplay: EntityDisplayComponent<Topology, object> = {
		getId: (topology: Topology) => topology.id,
		getLabel: (topology: Topology) =>
			(topology as Topology & { name?: string }).name ?? topology.id,
		getDescription: (topology: Topology) => {
			const networksData = queryClient.getQueryData<Network[]>(queryKeys.networks.all) ?? [];
			const network = networksData.find((n) => n.id == topology.network_id);
			return network ? network.name : common_unknownNetwork();
		},
		getIcon: () => entities.getIconComponent('Topology'),
		getIconColor: () => entities.getColorHelper('Topology').icon
	};
</script>

<script lang="ts">
	import type { EntityDisplayComponent } from '../types';
	import ListSelectItem from '../ListSelectItem.svelte';
	import type { Topology } from '$lib/features/topology/types/base';

	let {
		item,
		context = {}
	}: {
		item: Topology;
		context: object;
	} = $props();

	$effect(() => {
		void entities;
	});
</script>

<ListSelectItem {item} {context} displayComponent={TopologyDisplay} />
