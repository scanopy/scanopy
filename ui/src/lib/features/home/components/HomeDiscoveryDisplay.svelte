<script lang="ts" module>
	import { entities } from '$lib/shared/stores/metadata';
	import { runOutcomeTag } from '$lib/features/discovery/utils/outcome';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import type { Discovery } from '$lib/features/discovery/types/base';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import type { components } from '$lib/api/schema';

	type NetworkSummary = components['schemas']['NetworkSummary'];

	export interface HomeDiscoveryContext {
		daemons: Daemon[];
		networks: NetworkSummary[];
	}

	export const HomeDiscoveryDisplay: EntityDisplayComponent<Discovery, HomeDiscoveryContext> = {
		getId: (discovery) => discovery.id,
		getLabel: (discovery, context) => {
			// New records already have enriched names ("Type — Network").
			// Old records missing the separator get enriched client-side.
			if (discovery.name.includes(' \u2014 ')) return discovery.name;
			const network = context?.networks.find((n) => n.id === discovery.network_id);
			if (network) return `${discovery.name} \u2014 ${network.name}`;
			return discovery.name;
		},
		getDescription: (discovery, context) => {
			const daemon = context.daemons.find((d) => d.id === discovery.daemon_id);
			const daemonName = daemon?.name ?? 'Unknown Daemon';
			return `${daemonName} \u00b7 ${formatTimestamp(discovery.created_at)}`;
		},
		getIcon: () => entities.getIconComponent('Discovery'),
		getIconColor: () => entities.getColorHelper('Discovery').icon,
		getTags: (discovery) => {
			const tag = runOutcomeTag(
				discovery.run_type.type === 'Historical' ? discovery.run_type.results : null,
				{ includeCompleted: true }
			);
			return tag ? [tag] : [];
		}
	};
</script>

<script lang="ts">
	import type { EntityDisplayComponent } from '$lib/shared/components/forms/selection/types';
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';

	let {
		item,
		context = { daemons: [], networks: [] }
	}: { item: Discovery; context?: HomeDiscoveryContext } = $props();
</script>

<ListSelectItem {item} {context} displayComponent={HomeDiscoveryDisplay} />
