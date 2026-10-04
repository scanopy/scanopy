<script lang="ts" module>
	import { edgeTypes } from '$lib/shared/stores/metadata';
	import type { RenderableTopology, TopologyEdge } from '$lib/features/topology/types/base';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';
	import { common_host, common_unknownEntity } from '$lib/paraglide/messages';

	export const SameHostEdgeDisplay: EntityDisplayComponent<TopologyEdge, EdgeDisplayContext> = {
		getId: (edge) => edge.id,
		getLabel: (edge, context) => {
			if (!context?.topology || !('host_id' in edge)) return edgeTypes.getName(edge.edge_type);
			const host = context.topology.hosts.find((h) => h.id === edge.host_id);
			return host ? hostDisplayName(host) : common_unknownEntity({ entity: common_host() });
		},
		getDescription: (edge, context) => {
			if (!context?.topology) return '';
			const sourceIf = context.topology.ip_addresses.find((i) => i.id === edge.source);
			const targetIf = context.topology.ip_addresses.find((i) => i.id === edge.target);
			const parts: string[] = [];
			if (sourceIf?.ip_address) parts.push(sourceIf.ip_address);
			if (targetIf?.ip_address) parts.push(targetIf.ip_address);
			return parts.join(' ↔ ') || '';
		},
		getIcon: () => edgeTypes.getIconComponent('SameHost'),
		getIconColor: () => edgeTypes.getColorHelper('SameHost').icon
	};

	export interface EdgeDisplayContext {
		topology?: RenderableTopology;
	}
</script>

<script lang="ts">
	import type { EntityDisplayComponent } from '../types';
	import ListSelectItem from '../ListSelectItem.svelte';

	interface Props {
		item: TopologyEdge;
		context: EdgeDisplayContext;
	}

	let { item, context }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={SameHostEdgeDisplay} />
